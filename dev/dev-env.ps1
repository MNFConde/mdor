# dev/dev-env.ps1 — mdor Android 工具链会话变量（全局优先、便携兜底）
# 用法（仓库根）：. .\dev\dev-env.ps1
# cargo/rust-analyzer 侧变量由 .cargo/config.toml 负责；本脚本只服务 dx/Gradle 与手工 sdkmanager/adb。
# 设计：dev\ 工具树存在才注入；否则保留已有环境变量（全局依赖机零介入）。机制见 doc/env.md §2.6。

$repoRoot = Split-Path -Parent $PSScriptRoot
$androidRoot = Join-Path $repoRoot 'dev\android'
$jdkRoot = Join-Path $repoRoot 'dev\jdk'

# Android SDK：dev 树存在才注入，否则保留全局
if (Test-Path -LiteralPath $androidRoot) {
    $env:ANDROID_HOME = $androidRoot
    $env:Path = "$(Join-Path $androidRoot 'cmdline-tools\latest\bin');$(Join-Path $androidRoot 'platform-tools');$env:Path"
}

# NDK：取 dev 树下已安装版本（最高者）；无则保留已有 ANDROID_NDK_HOME/NDK_HOME
$ndkCandidates = @(Get-ChildItem -LiteralPath (Join-Path $androidRoot 'ndk') -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending)
if ($ndkCandidates.Count -gt 0) {
    $env:ANDROID_NDK_HOME = $ndkCandidates[0].FullName
    $env:NDK_HOME = $ndkCandidates[0].FullName
}

# JDK：dev 树下有便携 JDK 才设置，否则保留已有 JAVA_HOME（Scoop 兜底）
$jdkCandidates = @(Get-ChildItem -LiteralPath $jdkRoot -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending)
if ($jdkCandidates.Count -gt 0) {
    $env:JAVA_HOME = $jdkCandidates[0].FullName
}

Write-Host 'mdor dev env:'
Write-Host "  ANDROID_HOME      = $env:ANDROID_HOME"
Write-Host "  ANDROID_NDK_HOME  = $env:ANDROID_NDK_HOME"
Write-Host "  NDK_HOME          = $env:NDK_HOME"
Write-Host "  JAVA_HOME         = $env:JAVA_HOME"
