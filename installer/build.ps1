# 构建 RustShot Windows 安装包：cargo build --release → 复制 exe → Inno Setup 编译 → 静默安装
# 用法：  pwsh -File installer\build.ps1        （或右键“使用 PowerShell 运行”）
# 产物：  outputs\RustShot-Setup-<版本>.exe
# 默认打包完直接安装到 C:\Program Files\RustShot；-NoInstall 只打包不安装。
# -Interactive 显示安装进度；-KeepAppRunning 不自动关闭正在运行的 RustShot。

param(
    [switch]$NoInstall,
    [switch]$Interactive,
    [switch]$KeepAppRunning
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Get-InstalledRustShotVersion {
    $subKey = 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{B7E1C0F2-6B4A-4E1D-9A3C-2F5D8C1A7E64}_is1'
    foreach ($view in @([Microsoft.Win32.RegistryView]::Registry64, [Microsoft.Win32.RegistryView]::Registry32)) {
        $base = $null
        $key = $null
        try {
            $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, $view)
            $key = $base.OpenSubKey($subKey)
            if ($key) {
                $value = $key.GetValue('DisplayVersion')
                if ($value) { return [string]$value }
            }
        } catch {
            # 32 位系统没有 Registry64 视图，属正常情况。
        } finally {
            if ($key) { $key.Close() }
            if ($base) { $base.Close() }
        }
    }
    return $null
}

# 1. 版本号只认 Cargo.toml，避免安装包与 exe 版本漂移。
$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"(.+)"$').Matches[0].Groups[1].Value
Write-Host "RustShot $version"

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

if ($NoInstall) {
    Write-Host '已指定 -NoInstall，跳过自动安装。'
    return
}

# 4. Inno Setup 的 AppMutex 不允许 RustShot 在跑；先关掉再装，避免静默安装被弹窗拦住。
$running = @(Get-Process -Name 'rust_shot' -ErrorAction SilentlyContinue)
if ($running.Count -gt 0) {
    if ($KeepAppRunning) { throw 'RustShot 正在运行；请先退出（托盘右键 → 退出 / Ctrl+Alt+Q）后重试，或去掉 -KeepAppRunning。' }
    Write-Host '检测到 RustShot 正在运行，尝试关闭…'
    $running | ForEach-Object { [void]$_.CloseMainWindow() }
    Start-Sleep -Milliseconds 800
    $running = @(Get-Process -Name 'rust_shot' -ErrorAction SilentlyContinue)
    if ($running.Count -gt 0) {
        Write-Host '进程仍未退出，强制结束。'
        Stop-Process -Id $running.Id -Force
        Start-Sleep -Milliseconds 300
    }
}

# 5. 静默安装刚打出来的包；非管理员会话会弹一次 UAC。
$installLog = Join-Path $outputs "install-$version.log"
$installerArgs = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$installLog`"")
if ($Interactive) { $installerArgs[0] = '/SILENT' }
Write-Host "正在安装 RustShot $version …"
$installer = Start-Process -FilePath $setup -ArgumentList $installerArgs -Wait -PassThru -Verb RunAs
if ($null -ne $installer.ExitCode -and $installer.ExitCode -ne 0) { throw "安装失败（exit $($installer.ExitCode)），日志：$installLog" }

$installed = Get-InstalledRustShotVersion
if ($installed -ne $version) { throw "安装结束但版本校验失败：注册表为 '$installed'，预期 '$version'。日志：$installLog" }
Write-Host "已安装 RustShot $version（C:\Program Files\RustShot）"
