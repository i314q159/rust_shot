# 构建 RustShot Windows 安装包：cargo build --release → 签名 exe → 复制 → Inno Setup 编译 → 签名安装包
# 用法：  pwsh -File installer\build.ps1        （或右键“使用 PowerShell 运行”）
# 产物：  outputs\RustShot-Setup-<版本>.exe
#
# 代码签名（可选，二者其一即可）：
#   $env:RUSTSHOT_PFX           pfx 路径（默认找 installer\certs\rustshot-codesign.pfx）
#   $env:RUSTSHOT_PFX_PASSWORD  pfx 密码
# 没有证书时照常构建，只是产物未签名。

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

# 3. 取签名证书（没配置就跳过整段签名）。
$cert = $null
$pfxPath = if ($env:RUSTSHOT_PFX) { $env:RUSTSHOT_PFX } else { Join-Path $PSScriptRoot 'certs\rustshot-codesign.pfx' }
if (Test-Path -LiteralPath $pfxPath) {
    if (-not $env:RUSTSHOT_PFX_PASSWORD) {
        throw "找到 $pfxPath 但没有 RUSTSHOT_PFX_PASSWORD。先设置该环境变量（密码见 installer\certs\pfx-password.txt）再重新构建"
    }
    $secure = ConvertTo-SecureString -String $env:RUSTSHOT_PFX_PASSWORD -Force -AsPlainText
    $imported = Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation 'Cert:\CurrentUser\My' -Password $secure
    $cert = Get-Item -LiteralPath "Cert:\CurrentUser\My\$($imported.Thumbprint)"
    Write-Host "签名证书：$($cert.Subject)"
} else {
    Write-Host '未配置签名证书，本次构建不签名'
}

# Authenticode 签名：优先带 RFC3161 时间戳，时间戳服务不可达时退回无时间戳签名（仍然有效，只是过期后不再有效）。
function Sign-File {
    param([string]$Path, $Certificate)
    $ts = 'http://timestamp.digicert.com'
    try {
        $res = Set-AuthenticodeSignature -FilePath $Path -Certificate $Certificate -TimestampServer $ts -HashAlgorithm SHA256
    } catch {
        Write-Warning "时间戳服务不可用（$ts），改签不带时间戳的签名"
        $res = Set-AuthenticodeSignature -FilePath $Path -Certificate $Certificate -HashAlgorithm SHA256
    }
    if (-not $res.SignerCertificate) {
        throw "签名失败：$Path — $($res.StatusMessage)"
    }
    # 自签名证书在本机未受信时 Status 是 UnknownError，这里只看是否真的写入了签名者。
    Write-Host ("已签名 {0}（{1}；{2}）" -f (Split-Path -Leaf $Path), $res.SignerCertificate.Subject, $res.Status)
}

if ($cert) {
    Sign-File -Path $exe -Certificate $cert
}
Copy-Item -LiteralPath $exe -Destination (Join-Path $outputs 'rust_shot.exe') -Force
Write-Host "已复制 $(Join-Path $outputs 'rust_shot.exe')"

# 4. 找 ISCC（Inno Setup 编译器）。
$candidates = @(
    'ISCC.exe',
    (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
    (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
    (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
)
$iscc = $candidates | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
if (-not $iscc) { throw '找不到 ISCC.exe：请先安装 Inno Setup 6（winget install JRSoftware.InnoSetup）' }

$defines = @("/DMyAppVersion=$version")
# 只有自签名证书才需要在安装包里附带证书、并让用户选择是否导入信任；
# 正规 CA 签发的证书走系统信任链，什么都不用发。
$selfSigned = $cert -and ($cert.Subject -eq $cert.Issuer)
if ($selfSigned) {
    # 安装包内附上公钥证书，供用户自行导入信任；指纹用于卸载时把证书摘掉。
    $cerSource = [IO.Path]::ChangeExtension($pfxPath, '.cer')
    if (-not (Test-Path -LiteralPath $cerSource)) {
        throw "缺少公钥证书 $cerSource（用 installer\make-signing-cert.ps1 生成时会一并导出）"
    }
    $cer = Join-Path $outputs 'rustshot-codesign.cer'
    Copy-Item -LiteralPath $cerSource -Destination $cer -Force
    $defines += "/DSignCert=$cer"
    $defines += "/DCertThumbprint=$($cert.Thumbprint)"
}

& $iscc (Join-Path $PSScriptRoot 'rust_shot.iss') @defines
if ($LASTEXITCODE -ne 0) { throw "ISCC 编译失败（exit $LASTEXITCODE）" }

$setup = Join-Path $outputs "RustShot-Setup-$version.exe"
if (-not (Test-Path -LiteralPath $setup)) { throw "未生成 $setup" }
if ($cert) {
    Sign-File -Path $setup -Certificate $cert
}
$mb = [math]::Round((Get-Item -LiteralPath $setup).Length / 1MB, 2)
Write-Host "安装包：$setup ($mb MB)"