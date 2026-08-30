#!/usr/bin/env python3
"""shot-analyze.py — GUI 截图像素分析（窗口定位 + 渲染判定），固化自 2026-08-30 M1 书架视觉验收会话的内联分析。

背景（为什么存在）：agent 模型不支持读图（Read PNG 表面成功、媒体附件实际报错），
GUI 取证流水线 = gnome-screenshot 落盘 → 本脚本像素分析 → 用户肉眼终审。

用法:
  python3 script/shot-analyze.py <截图.png> [选项]
  常用:
    python3 script/shot-analyze.py shot.png                      # 整屏分析
    python3 script/shot-analyze.py shot.png --window mdor-app    # 按 X11 窗口类名定位内容区（需 xwininfo + X 权限）
    python3 script/shot-analyze.py shot.png --crop 159,245,800,600  # 手动裁剪 x,y,w,h

扩展约定（三次法则，见 script/AGENTS.md）:
  新分析需求一律扩展本脚本，禁止另写临时分析脚本。管线各步为独立函数：
  locate_window → crop_to_region → color_stats → row_segments → verdict，
  新能力 = 新函数 + argparse 参数 + scripts.md 登记用法；主干不动。
  扩展候选：OCR 文本提取（tesseract）、双截图 diff、主题自适应阈值。

依赖: Pillow（uv 环境自动装；系统 python3 直跑需 python3-pil / pip install pillow）。

退出码: 0 = 分析完成（verdict 见输出，blank/black 属正常分析结果非错误）；
        1 = 文件不存在/格式错；2 = 依赖缺失；3 = 参数错误。
"""

import argparse
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

try:
    from PIL import Image
except ImportError:
    print(
        "错误: 缺少 Pillow。uv 环境: uv run --directory script shot-analyze.py ...；"
        "系统直跑: sudo apt install python3-pil 或 pip install pillow",
        file=sys.stderr,
    )
    sys.exit(2)


# ---------------------------------------------------------------- 窗口定位


def locate_window(window_class: str) -> tuple[int, int, int, int] | None:
    """经 xwininfo -root -tree 找 X11 client 窗口的绝对几何 (x, y, w, h)。

    匹配 WM_CLASS 含 window_class 的行；跳过 mutter-x11-frames 装饰壳与
    10x10 之类的辅助窗口，取「内容窗口行」的最后一组 +x+y 绝对坐标与
    相对尺寸（w x h 取相对段）。
    """
    try:
        proc = subprocess.run(
            ["xwininfo", "-root", "-tree"],
            capture_output=True,
            text=True,
            timeout=10,
            env={"DISPLAY": ":0", "PATH": "/usr/bin:/bin", "HOME": "/root"},
        )
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return None
    if proc.returncode != 0:
        return None

    # 行样例:  0xc00003 "Dioxus App": ("mdor-app" "Mdor-app")  800x600+14+49  +66+69
    #          0xa00010 "Dioxus App": ("mutter-x11-frames" ...) 932x817+52+20 +52+20
    pat = re.compile(
        r'^\s*\S+\s+"[^"]*":\s*\("([^"]+)"[^)]*\)\s+(\d+)x(\d+)[+-]\d+[+-]\d+\s+\+(\d+)\+(\d+)'
    )
    candidates: list[tuple[int, int, int, int]] = []
    for line in proc.stdout.splitlines():
        if "mutter-x11-frames" in line:
            continue  # 装饰壳
        m = pat.match(line)
        if not m:
            continue
        wm_class, w, h, ax, ay = m.group(1), *map(int, m.groups()[1:])
        if window_class.lower() in wm_class.lower():
            candidates.append((ax, ay, w, h))
    if not candidates:
        return None
    # 取面积最大的匹配（内容窗口 > 辅助小窗）
    return max(candidates, key=lambda g: g[2] * g[3])


def crop_to_region(
    im: Image.Image, crop: str | None, window_class: str
) -> tuple[Image.Image, str]:
    """按优先级裁出分析区域：--crop 手动 > xwininfo 自动 > 整屏。返回 (图像, 区域描述)。"""
    if crop:
        try:
            x, y, w, h = (int(v) for v in crop.split(","))
        except ValueError:
            print(f"错误: --crop 格式须为 x,y,w,h（整数逗号分隔），收到: {crop}", file=sys.stderr)
            sys.exit(3)
        if min(x, y, w, h) < 0 or w <= 0 or h <= 0:
            print(f"错误: --crop 尺寸须为正、坐标非负，收到: {crop}", file=sys.stderr)
            sys.exit(3)
        return im.crop((x, y, x + w, y + h)), f"manual crop {crop}"
    geo = locate_window(window_class)
    if geo:
        x, y, w, h = geo
        return im.crop((x, y, x + w, y + h)), f"window '{window_class}' at +{x}+{y} {w}x{x and h}"
    return im, f"full frame {im.width}x{im.height}"


# ---------------------------------------------------------------- 分析


def color_stats(im: Image.Image, top: int) -> dict:
    """主色统计：top N RGB + 非主色像素占比（判空白/黑屏核心指标）。"""
    rgb = im.convert("RGB")
    cc = Counter(rgb.getdata())
    total = sum(cc.values())
    main_color, main_n = cc.most_common(1)[0]
    diverse = total - main_n
    return {
        "total": total,
        "top_colors": [{"rgb": list(c), "count": n, "pct": round(n / total * 100, 2)} for c, n in cc.most_common(top)],
        "main_color": list(main_color),
        "non_main_pct": round(diverse / total * 100, 2),
    }


def row_segments(
    im: Image.Image, thresh: float, min_density: float = 0.01, min_span: float = 0.5
) -> list[dict]:
    """按行灰度方差聚类文本行段（阈值以上 = 有内容），返回 [{y0, y1, h}]。

    三重判据（缺一不可，均为会话实测标定）：
    - 行灰度方差 > 中位背景 + thresh（高频明暗变化）
    - 对比像素（|灰度-背景主值|>64）占行宽 ≥ min_density
    - 对比像素 x 跨度 ≥ min_span×行宽 —— 区分「散布全行的文本/UI 内容」与
      「聚在窗口一角的小图标」：实测文本行 span≥0.96（stdev≥0.23），
      标题栏图标行 span≤0.16（stdev≤0.05），间隔两个数量级。
      注意不能用「行内最大饱和度」判彩色图标——UI 文字本身可能是彩色
      （如链接蓝书名），饱和度判据会把真文本行误杀。
    """
    rgb = im.convert("RGB")
    g = im.convert("L")
    px, gpx = rgb.load(), g.load()
    w, h = g.size
    if w == 0 or h == 0:
        return []
    import statistics

    rows = []
    for y in range(h):
        vals = [gpx[x, y] for x in range(0, w, 2)]
        mean = sum(vals) / len(vals)
        rows.append(sum((v - mean) ** 2 for v in vals) / len(vals))
    bg = statistics.median(rows)
    bg_lum = statistics.median(gpx[x, y] for y in range(0, h, 4) for x in range(0, w, 4))

    segs: list[list[int]] = []
    in_seg = False
    for y, v in enumerate(rows):
        xs = [x for x in range(w) if abs(sum(px[x, y]) // 3 - bg_lum) > 64]
        contrast = len(xs) / w
        span = (max(xs) - min(xs)) / w if xs else 0.0
        ok = v > bg + thresh and contrast >= min_density and span >= min_span
        if ok:
            if in_seg:
                segs[-1][1] = y
            else:
                segs.append([y, y])
                in_seg = True
        else:
            in_seg = False
    return [{"y0": a, "y1": b, "h": b - a} for a, b in segs if b - a >= 5]


def verdict(stats: dict, segs: list[dict]) -> str:
    """渲染判定：rendered / blank（单色底=无内容渲染）/ black（纯黑底）/ unclear。

    会话实测标定：WebKit EGL 崩溃空白窗主色 RGB(44,44,44)（GTK 暗底灰）、
    合成瘫痪黑窗主色 RGB(0,0,0)。亮度阈值 32 分开两者：
    <32 = black（渲染管线瘫）、32..64 = blank（窗底色无内容）。
    """
    if segs:
        return "rendered"
    if stats["non_main_pct"] < 0.5:
        r, g, b = stats["main_color"]
        lum = 0.299 * r + 0.587 * g + 0.114 * b
        return "black" if lum < 32 else "blank"
    return "unclear"


# ---------------------------------------------------------------- 入口


def main() -> int:
    ap = argparse.ArgumentParser(description="GUI 截图像素分析（窗口定位 + 渲染判定）")
    ap.add_argument("image", type=Path, help="截图 PNG 路径")
    ap.add_argument("--crop", help="手动区域 x,y,w,h（优先于 --window）")
    ap.add_argument("--window", default="mdor-app", help="X11 窗口类名匹配串（默认 mdor-app）")
    ap.add_argument("--row-thresh", type=float, default=300.0, help="文本行方差阈值（默认 300，实测标定值）")
    ap.add_argument("--min-density", type=float, default=0.01, help="行段对比像素最小占比（默认 0.01）")
    ap.add_argument("--min-span", type=float, default=0.5, help="对比像素 x 跨度最小占行宽比（默认 0.5，滤窗口一角的小图标）")
    ap.add_argument("--top", type=int, default=5, help="主色统计条数（默认 5）")
    ap.add_argument("--json", action="store_true", help="JSON 输出（供 agent 程序化消费）")
    args = ap.parse_args()

    if not args.image.is_file():
        print(f"错误: 文件不存在 {args.image}", file=sys.stderr)
        return 1
    try:
        im = Image.open(args.image)
        im.load()
    except Exception as e:
        print(f"错误: 无法读取图像 {args.image}: {e}", file=sys.stderr)
        return 1

    region, region_desc = crop_to_region(im, args.crop, args.window)
    stats = color_stats(region, args.top)
    segs = row_segments(region, args.row_thresh, args.min_density, args.min_span)
    result = {
        "image": str(args.image),
        "region": region_desc,
        "size": [region.width, region.height],
        "verdict": verdict(stats, segs),
        "color_stats": stats,
        "row_segments": segs,
    }

    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0

    print(f"图像: {args.image}")
    print(f"区域: {region_desc} ({region.width}x{region.height})")
    print(f"判定: {result['verdict']}")
    print(f"非主色像素占比: {stats['non_main_pct']}%")
    print("主色 top:")
    for c in stats["top_colors"]:
        print(f"  RGB{tuple(c['rgb'])}  {c['pct']}%")
    print(f"文本行段: {len(segs)}")
    for s in segs[:20]:
        print(f"  y {s['y0']}-{s['y1']} (h={s['h']})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
