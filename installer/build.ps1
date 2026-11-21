param(
    [switch]$NoInstall,
    [switch]$Interactive,
    [switch]$KeepAppRunning
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Get-InstalledRustShotVersion {
    $subKey = 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{B7E1C0F2-6B4A-4E1D-9A3C-2F5D8C1A7E64}_is1'
    $views = @(
        [Microsoft.Win32.RegistryView]::Registry64,
        [Microsoft.Win32.RegistryView]::Registry32
    )

    foreach ($view in $views) {
        $base = $null
        $key = $null
        try {
            $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
                [Microsoft.Win32.RegistryHive]::LocalMachine,
                $view
            )
            $key = $base.OpenSubKey($subKey)
            if ($key) {
                $value = $key.GetValue('DisplayVersion')
                if ($value) {
                    return [string]$value
                }
            }
        }
        catch {
        }
        finally {
            if ($key) {
                $key.Close()
            }
            if ($base) {
                $base.Close()
            }
        }
    }

    return $null
}

$manifest = Get-Content -LiteralPath (Join-Path $root 'Cargo.toml') -Raw
$package = [regex]::Match($manifest, '(?ms)^\[package\]\s*(?<body>.*?)(?=^\[|\z)')
if (-not $package.Success) {
    throw 'Cargo.toml 缺少 [package] 段'
}

$versionMatch = [regex]::Match($package.Groups['body'].Value, '(?m)^version\s*=\s*"(.+)"\s*$')
if (-not $versionMatch.Success) {
    throw 'Cargo.toml 的 [package] 段缺少 version'
}

$version = $versionMatch.Groups[1].Value
Write-Host "RustShot $version"

Push-Location $root
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw 'cargo build --release 失败'
    }
}
finally {
    Pop-Location
}

$exe = Join-Path $root 'target\release\rust_shot.exe'
$outputs = Join-Path $root 'outputs'
New-Item -ItemType Directory -Force -Path $outputs | Out-Null
Copy-Item -LiteralPath $exe -Destination (Join-Path $outputs 'rust_shot.exe') -Force
Write-Host "已复制 $(Join-Path $outputs 'rust_shot.exe')"

$candidates = @(
    'ISCC.exe',
    (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
    (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
    (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
)
$iscc = $candidates |
    Where-Object { $_ -and (Test-Path -LiteralPath $_) } |
    Select-Object -First 1
if (-not $iscc) {
    throw '找不到 ISCC.exe：请先安装 Inno Setup 6（winget install JRSoftware.InnoSetup）'
}

& $iscc (Join-Path $PSScriptRoot 'rust_shot.iss') "/DMyAppVersion=$version"
if ($LASTEXITCODE -ne 0) {
    throw "ISCC 编译失败（exit $LASTEXITCODE）"
}

$setup = Join-Path $outputs "RustShot-Setup-$version.exe"
if (-not (Test-Path -LiteralPath $setup)) {
    throw "未生成 $setup"
}

$mb = [math]::Round((Get-Item -LiteralPath $setup).Length / 1MB, 2)
Write-Host "安装包：$setup ($mb MB)"

if ($NoInstall) {
    Write-Host '已指定 -NoInstall，跳过自动安装。'
    return
}

$running = @(Get-Process -Name 'rust_shot' -ErrorAction SilentlyContinue)
if ($running.Count -gt 0) {
    if ($KeepAppRunning) {
        throw 'RustShot 正在运行；请先退出（托盘右键 → 退出 / Ctrl+Alt+Q）后重试，或去掉 -KeepAppRunning。'
    }

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

$installLog = Join-Path $outputs "install-$version.log"
$installerArgs = @(
    '/VERYSILENT',
    '/SUPPRESSMSGBOXES',
    '/NORESTART',
    "/LOG=`"$installLog`""
)
if ($Interactive) {
    $installerArgs[0] = '/SILENT'
}

Write-Host "正在安装 RustShot $version …"
$installer = Start-Process -FilePath $setup -ArgumentList $installerArgs -Wait -PassThru -Verb RunAs
if ($null -ne $installer.ExitCode -and $installer.ExitCode -ne 0) {
    throw "安装失败（exit $($installer.ExitCode)），日志：$installLog"
}

$installed = Get-InstalledRustShotVersion
if ($installed -ne $version) {
    throw "安装结束但版本校验失败：注册表为 '$installed'，预期 '$version'。日志：$installLog"
}

Write-Host "已安装 RustShot $version（C:\Program Files\RustShot）"

$installedExe = 'C:\Program Files\RustShot\rust_shot.exe'
if (-not (Test-Path -LiteralPath $installedExe)) {
    throw "找不到已安装的程序：$installedExe"
}
Start-Process -FilePath $installedExe -ArgumentList '--tray'
Write-Host "已启动 $installedExe（--tray）"
