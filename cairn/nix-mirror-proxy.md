---
type: project_topic
status: active
summary: "Nix 多用户环境（nixos-wsl / ubuntu-dev VM）的国内镜像矩阵与代理链路（2026-08-28 实测）：二进制缓存 USTC/TUNA/SJTU store；rust-static USTC（VM 直连 403→弃）/ rsproxy（VM 可用）/ SJTU 200 / TUNA·BFSU 404；crates rsproxy sparse；工具链 fixed-output 预置 nix-store --add-fixed；nix client 吃 shell 代理而 nix-daemon 不吃（须 systemctl edit 注入）；clash external-controller(9090)≠mixed-port(7890)；clashctl off 不清当前 shell 导出变量（假直连）；宿主机 TUN + VM clash = 双代理链；国际直连 ~50–80KB/s 为真实瓶颈→绕开而非调代理"
tags: [mdor, nix, mirror, proxy, clash, tun, rust-static, rsproxy, ustc, nix-daemon, fixed-output, network, china]
contains: [lesson, experience, decision]
created: "2026-08-28"
updated: "2026-08-28"
related: [ubuntu-vm-setup.md, nix-env-tooling.md, env.md]
authoring_mode: ai_generated
---
# Nix 国内镜像与代理链路（多用户环境通用）

## 背景

[nixos-wsl](../doc/env.md#开发环境拓扑) 与 ubuntu-dev VM 均为 Nix 多用户（daemon）环境。本专题沉淀 2026-08-28 在 ubuntu-dev VM 环境准备中实测的**国内镜像矩阵**与**代理链路架构**——跨机器可复用（nixos-wsl 同适用）。操作流程见 [ubuntu-vm-setup.md](ubuntu-vm-setup.md)。

## 国内镜像矩阵（本会话实测）

> 判定：`curl -I`/`curl -x` 状态码。以下均为 2026-08-28 实测。

**nix 二进制缓存**（覆盖 devShell 闭包大头）：

| 源 | URL | 结果 |
|---|---|---|
| USTC | `https://mirrors.ustc.edu.cn/nix-channels/store` | 200（0.31s） |
| TUNA | `https://mirrors.tuna.tsinghua.edu.cn/nix-channels/store` | 200（0.43s） |
| SJTU | `https://mirror.sjtu.edu.cn/nix-channels/store` | 200（0.15s） |
| 官方 | `https://cache.nixos.org` | 200（1.25s） |

```ini
# /etc/nix/nix.conf
substituters = https://mirrors.ustc.edu.cn/nix-channels/store https://cache.nixos.org/
```

**rust-static**（rust-overlay 工具链 tarball 源 `static.rust-lang.org`，国际直连仅 ~50–80KB/s）：

| 源 | URL（`dist/<YYYY-MM-DD>/<pkg>-1.97.1-x86_64-unknown-linux-gnu.tar.xz`） | 结果 |
|---|---|---|
| USTC | `https://mirrors.ustc.edu.cn/rust-static/dist/…` | 宿主机 200 / **VM 直连 403** |
| rsproxy | `https://rsproxy.cn/dist/…` | 200（307→CDN，`-L` 跟随），**VM 可用** ✅ |
| SJTU | `https://mirror.sjtu.edu.cn/rust-static/dist/…` | 200 |
| TUNA | `https://mirrors.tuna.tsinghua.edu.cn/rust-static/dist/…` | 404 |
| BFSU | `https://mirrors.bfsu.edu.cn/rust-static/dist/…` | 404 |

**crates.io**（dx 编译几百个 crate）：

```toml
# ~/.cargo/config.toml
[source.crates-io]
replace-with = "rsproxy"
[source.rsproxy]
registry = "sparse+https://rsproxy.cn/index/"
```

## 慢速根因链

1. **国际直连到 `static.rust-lang.org` 是本网络真实瓶颈**（~50–80KB/s）——关掉宿主机 TUN 后 VM 真直连反而更慢（~50KB/s），证实不是 NAT/代理配置问题，是链路本身差。
2. 对策 = **绕开而非调代理**：工具链走国内镜像预置、nix 缓存走 USTC、crates 走 rsproxy。
3. `nix develop` 大件（fixed-output fetch）**每次中断后从 0 重下**——失败/打断的抓取不落 store。

## 工具链 fixed-output 预置（跳过国际下载）

原理：`rustc/rust-std/rust-analyzer` 是 fixed-output 派生，**store 里已有同名同哈希路径则 nix 直接跳过抓取**。

```bash
mkdir -p /tmp/rust-dist && cd /tmp/rust-dist
base=https://rsproxy.cn/dist/2026-07-16
curl -fL --noproxy '*' -o rustc-1.97.1-x86_64-unknown-linux-gnu.tar.xz            $base/rustc-1.97.1-x86_64-unknown-linux-gnu.tar.xz
curl -fL --noproxy '*' -o rust-std-1.97.1-x86_64-unknown-linux-gnu.tar.xz         $base/rust-std-1.97.1-x86_64-unknown-linux-gnu.tar.xz
curl -fL --noproxy '*' -o rust-analyzer-1.97.1-x86_64-unknown-linux-gnu.tar.xz    $base/rust-analyzer-1.97.1-x86_64-unknown-linux-gnu.tar.xz
# 文件名必须与 store 路径 basename 完全一致：
nix-store --add-fixed sha256 ./rustc-1.97.1-x86_64-unknown-linux-gnu.tar.xz
nix-store --add-fixed sha256 ./rust-std-1.97.1-x86_64-unknown-linux-gnu.tar.xz
nix-store --add-fixed sha256 ./rust-analyzer-1.97.1-x86_64-unknown-linux-gnu.tar.xz
```
- 镜像与官方同源、逐字节一致 → 哈希必对上；若哈希不符，nix 照常重新抓取，**无害**。
- 日期子目录 `2026-07-16` 随工具链版本变化；`ls /nix/store` 或 `nix log` 可见实际 URL。

## 代理架构（nix client vs nix-daemon）

| 抓取 | 执行者 | shell 代理是否生效 |
|---|---|---|
| flake/GitHub 源码、求值期抓取 | `nix` 客户端进程 | ✅ 继承 shell env，无需标志 |
| 二进制替换、fixed-output fetch | **nix-daemon**（systemd 服务） | ❌ 不读 profile.d、不继承终端 env |

- `sudo -E`（nixos-rebuild 惯用）对 nix **客户端无效**——没有标志能把 shell 代理转发给 daemon fetcher。
- 可靠做法 = 给 daemon 注入环境（持久）：
```bash
sudo systemctl edit nix-daemon
# [Service] 下 8 行（大小写都要，避免 libcurl 大小写优先级赌运气）：
Environment=http_proxy=http://127.0.0.1:7890
Environment=https_proxy=http://127.0.0.1:7890
Environment=all_proxy=http://127.0.0.1:7890
Environment=no_proxy=localhost,127.0.0.1,::1
Environment=HTTP_PROXY=http://127.0.0.1:7890
Environment=HTTPS_PROXY=http://127.0.0.1:7890
Environment=ALL_PROXY=http://127.0.0.1:7890
Environment=NO_PROXY=localhost,127.0.0.1,::1
sudo systemctl restart nix-daemon
# 验证: sudo systemctl show nix-daemon -p Environment
```
- nix.conf 的 `build-extra-env` **管不到 daemon 自身 fetcher**（只管构建进程），对二进制替换/fixed-output 无效。

## clash 端口语义与双代理坑

1. **`external-controller`（默认/随机端口，clashctl ui 打印，如 9090）= Web 面板/API 端口，不是代理**。证据：源码 `scripts/lib/config.sh` 中 `_detect_proxy_port` 读 `.mixed-port/.port/.socks-port`，而 `clashui`/`_detect_ext_addr` 用 `.external-controller`；`curl -x http://127.0.0.1:9090` → 000/405（RESTful API 拒代理请求），`curl -x http://127.0.0.1:7890` → 200。
2. **真正代理端口 = `mixed-port`**（HTTP+SOCKS5 合一，如 7890）。查法：`grep mixed-port $CLASHCTL_HOME/resources/runtime.yaml` 或 `clashctl info`。
3. **`clashctl off` 只停服务/关新 shell 代理，不清当前 shell 已导出的 `http_proxy` 等**——当前 shell 直连测试是「假直连」。做直连测试先 `unset http_proxy https_proxy all_proxy HTTP_PROXY HTTPS_PROXY ALL_PROXY` + `curl --noproxy '*'`。
4. **宿主机 TUN + VM clash = 双代理链**：宿主机 TUN 透明接管 VM NAT 流量（走宿主机路由表），VM 内再开 clash 就成「目标←VM节点←VM clash←NAT←宿主机TUN←宿主机节点」链，吞吐被慢层卡死。二选一：关宿主机 TUN（VM 自管代理，推荐 VM 场景）或撤 VM 内 clash（宿主机 TUN 全管）。别两层同开。
5. 关宿主机 TUN 后 VM 直连更慢 → 佐证瓶颈是国际链路本身而非代理（见「慢速根因链」）。

## 决策

- **rust 工具链/crates 主源 = rsproxy**（VM 实测可用、一家管两件事）；nix 二进制缓存 = USTC store；SJTU 作 rust-static 备选。
- USTC rust-static 在 VM 直连 403 根因未深挖（unset 代理 + `--noproxy '*'` 仍 403，疑 IP/UA 维度拦截）——不纠缠，换镜像即可。
- 代理：nix-daemon 注入优先于 clash TUN（持久、可控）；VM 场景建议宿主机 TUN 关、VM 自管。
