# 构建 RustShot Windows 安装包：cargo build --release → 复制 exe → Inno Setup 编译
# 用法：  pwsh -File installer\build.ps1        （或右键“使用 PowerShell 运行”）
# 产物：  outputs\RustShot-Setup-<版本>.exe

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

# 1. 版本号只认 Cargo.toml，避免安装包与 exe 版本漂移。
$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"(.+)"$').Matches[0].Groups[1].Value
Write-Host "RustShot $version"

# 1b. v.txt 记录最近一次构建用的版本号（Cargo.toml 仍是唯一来源，这里只留档）。
Set-Content -LiteralPath (Join-Path $root 'v.txt') -Value $version -Encoding UTF8

# 2. Release 构建（质量门由调用方保证；这里至少确保编译通过）。
Push-Location $root
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release 失败" }
} finally {
    Pop-Location
}

$exe = Join-Path $root 'target\release\rust_shot.exe'
$outputs = Join-Path $root 'outputs'
New-Item -ItemType Directory -Force -Path $outputs | Out-Null
Copy-Item -LiteralPath $exe -Destination (Join-Path $outputs 'rust_shot.exe') -Force
Write-Host "已复制 $(Join-Path $outputs 'rust_shot.exe')"

# 3. 找 ISCC（Inno Setup 编译器）。
$candidates = @(
    'ISCC.exe',
    (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
    (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
    (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
)
$iscc = $candidates | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
if (-not $iscc) { throw '找不到 ISCC.exe：请先安装 Inno Setup 6（winget install JRSoftware.InnoSetup）' }

& $iscc (Join-Path $PSScriptRoot 'rust_shot.iss') "/DMyAppVersion=$version"
if ($LASTEXITCODE -ne 0) { throw "ISCC 编译失败（exit $LASTEXITCODE）" }

$setup = Join-Path $outputs "RustShot-Setup-$version.exe"
if (-not (Test-Path -LiteralPath $setup)) { throw "未生成 $setup" }
$mb = [math]::Round((Get-Item -LiteralPath $setup).Length / 1MB, 2)
Write-Host "安装包：$setup ($mb MB)"