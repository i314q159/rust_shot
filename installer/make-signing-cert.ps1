# 生成 RustShot 的自签名代码签名证书（一次即可，之后复用同一个 .pfx）
#
# 用法：
#   pwsh -File installer\make-signing-cert.ps1 -Password "你的pfx密码"
#
# 产物（installer\certs\，已被 .gitignore 忽略）：
#   rustshot-codesign.pfx  私钥，签名用；丢了就只能重新签，所以请备份
#   rustshot-codesign.cer  公钥，随安装包分发，用户导入后发行者显示为证书里的名称
#
# 注意：自签名证书不会消除 SmartScreen，只是让“发行者”从“未知”变成证书主体名称，
# 且必须先让对方把这枚 .cer 装进“受信任的根证书颁发机构”。

param(
    [Parameter(Mandatory = $true)]
    [string]$Password,
    [string]$Subject = "CN=i314q159, O=RustShot, C=CN",
    [int]$Years = 5
)

$ErrorActionPreference = 'Stop'
$outDir = Join-Path $PSScriptRoot 'certs'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$pfx = Join-Path $outDir 'rustshot-codesign.pfx'
$cer = Join-Path $outDir 'rustshot-codesign.cer'
if ((Test-Path -LiteralPath $pfx) -and -not $args.Contains('-Force')) {
    Write-Host "已存在 $pfx，如需重新生成请先删除它（重新生成会让所有已发布版本的发行者变化）。"
    exit 1
}

$cert = New-SelfSignedCertificate `
    -Type CodeSigningCert `
    -Subject $Subject `
    -KeyAlgorithm RSA `
    -KeyLength 3072 `
    -HashAlgorithm SHA256 `
    -KeyExportPolicy Exportable `
    -KeyUsage DigitalSignature `
    -NotAfter (Get-Date).AddYears($Years) `
    -CertStoreLocation 'Cert:\CurrentUser\My'

$secure = ConvertTo-SecureString -String $Password -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath $pfx -Password $secure | Out-Null
Export-Certificate -Cert $cert -FilePath $cer -Type CERT | Out-Null

Write-Host "证书已生成"
Write-Host "  主体      $($cert.Subject)"
Write-Host "  指纹      $($cert.Thumbprint)"
Write-Host "  有效期至  $($cert.NotAfter.ToString('yyyy-MM-dd'))"
Write-Host "  pfx       $pfx"
Write-Host "  cer       $cer"
Write-Host ""
Write-Host "本地构建：把 pfx 密码放进环境变量 RUSTSHOT_PFX_PASSWORD 即可自动签名；"
Write-Host "CI 发布：把 pfx 的 base64 存进 secret RUSTSHOT_PFX_B64，密码存进 RUSTSHOT_PFX_PASSWORD。"