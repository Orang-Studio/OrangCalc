<#
.SYNOPSIS
    Builds OrangCalc and its Inno Setup installer, signing both with OrangStudio certificate.
.EXAMPLE
    .\installer\build-installer.ps1
    .\installer\build-installer.ps1 -CertPassword "my secret"
    .\installer\build-installer.ps1 -SkipSigning
#>
[CmdletBinding()]
param(
    [string]$CertPassword = "OrangStudio!",
    [switch]$SkipSigning,
    [switch]$NoTimestamp
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot      = Split-Path -Parent $PSScriptRoot
$ProjDir       = Join-Path $RepoRoot 'OrangCalc'
$IssFile       = Join-Path $PSScriptRoot 'OrangCalc.iss'
$PfxFile       = Join-Path $PSScriptRoot 'OrangStudio.pfx'
$IsccPath      = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 7\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 7\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 7\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1

function Step { param($msg) Write-Host "==> $msg" -ForegroundColor Cyan }
function Ok   { param($msg) Write-Host "    $msg" -ForegroundColor Green }
function Warn { param($msg) Write-Host "    $msg" -ForegroundColor Yellow }

if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) { throw 'dotnet SDK not found in PATH.' }
if (-not $IsccPath) { throw 'Inno Setup (ISCC.exe) not found. Install it: winget install JRSoftware.InnoSetup' }
if (-not $SkipSigning) {
    $SignTool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.Directory.Name -eq 'x64' } |
        Sort-Object { try { [version]$_.Directory.Parent.Name } catch { [version]'0.0.0.0' } } -Descending |
        Select-Object -First 1 -ExpandProperty FullName
    if (-not $SignTool) { throw 'signtool.exe not found (Windows SDK). Sign later once installed, or skip signing with -SkipSigning.' }
}

Step 'Building OrangCalc (Release)'
Push-Location $RepoRoot
try { dotnet build 'OrangCalc' -c Release; if ($LASTEXITCODE -ne 0) { throw 'dotnet build failed.' } } finally { Pop-Location }
$AppExe = Join-Path $ProjDir 'bin\Release\net48\OrangCalc.exe'
Ok "Built: $AppExe"

$secure = ConvertTo-SecureString -String $CertPassword -AsPlainText -Force
$cert   = $null

if (-not $SkipSigning) {
    Step 'Ensuring OrangStudio code-signing certificate'

    $cert = Get-ChildItem Cert:\CurrentUser\My -ErrorAction SilentlyContinue |
        Where-Object { $_.HasPrivateKey -and ($_.Subject -match 'OrangStudio' -or $_.FriendlyName -match 'OrangStudio') -and ($_.EnhancedKeyUsageList.ObjectId -contains '1.3.6.1.5.5.7.3.3') } |
        Sort-Object NotAfter -Descending | Select-Object -First 1

    if (-not $cert) {
        if (Test-Path $PfxFile) {
            $cert = Import-PfxCertificate -FilePath $PfxFile -CertificateStoreLocation Cert:\CurrentUser\My -Password $secure
            Ok "Imported existing PFX: cert thumbprint $($cert.Thumbprint)"
        } else {
            $cert = New-SelfSignedCertificate -Type CodeSigningCert `
                -Subject 'CN=OrangStudio' `
                -FriendlyName 'OrangStudio Code Signing' `
                -CertStoreLocation Cert:\CurrentUser\My `
                -KeyUsage DigitalSignature `
                -KeyExportPolicy Exportable `
                -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3') `
                -NotAfter (Get-Date).AddYears(10)
            Ok "Created new self-signed cert $($cert.Thumbprint) (valid 10 years)"
        }
    } else {
        Ok "Using existing cert $($cert.Thumbprint) ($($cert.FriendlyName))"
    }

    Export-PfxCertificate -Cert $cert.PSPath -FilePath $PfxFile -Password $secure -Force | Out-Null
    Ok 'Exported cert to installer\OrangStudio.pfx (password: <CertPassword parameter>)'
    Export-Certificate -Cert $cert.PSPath -FilePath (Join-Path $PSScriptRoot 'OrangStudio.cer') -Force | Out-Null

    $trusted = Get-ChildItem Cert:\CurrentUser\TrustedPublisher -ErrorAction SilentlyContinue | Where-Object Thumbprint -eq $cert.Thumbprint
    if (-not $trusted) {
        try {
            $tempPfx = Join-Path ([IO.Path]::GetTempPath()) "orangstudio-$($cert.Thumbprint).pfx"
            Export-PfxCertificate -Cert $cert.PSPath -FilePath $tempPfx -Password $secure -Force | Out-Null
            Import-PfxCertificate -FilePath $tempPfx -CertStoreLocation Cert:\CurrentUser\TrustedPublisher -Password $secure | Out-Null
            Remove-Item $tempPfx -ErrorAction SilentlyContinue
            Ok 'Added cert to CurrentUser TrustedPublisher (local trust)'
        } catch {
            Warn "Could not add to TrustedPublisher: $($_.Exception.Message)"
        }
    }
}


function Invoke-Sign {
    param([string]$File, [bool]$TryTimestamp = $true)
    if ($SkipSigning) { return }
    if ($TryTimestamp -and -not $NoTimestamp) {
        & $SignTool sign /sha1 $cert.Thumbprint /fd sha256 @('/tr', 'http://timestamp.digicert.com', '/td', 'sha256') $File 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) { Ok "Signed (timestamped): $(Split-Path -Leaf $File)"; return }
        Warn 'Timestamping failed (offline?), signing without timestamp...'
    }
    & $SignTool sign /sha1 $cert.Thumbprint /fd sha256 $File | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "signtool failed for $File." }
    Ok "Signed: $(Split-Path -Leaf $File)"
}

if (-not $SkipSigning) { Step 'Signing application executable'; Invoke-Sign -File $AppExe }

Step 'Compiling Inno Setup installer'
& $IsccPath $IssFile
if ($LASTEXITCODE -ne 0) { throw 'ISCC compilation failed.' }

$OutputDir = Join-Path $PSScriptRoot 'Output'
$Installer = Join-Path $OutputDir 'OrangCalc-4.0.0-setup.exe'
Ok "Created: $Installer"

if (-not $SkipSigning) { Step 'Signing installer'; Invoke-Sign -File $Installer }

if (-not $SkipSigning -and (Test-Path $Installer)) {
    Step 'Verifying signatures'
    foreach ($f in @($AppExe, $Installer)) {
        $sig = Get-AuthenticodeSignature $f
        Ok "$(Split-Path -Leaf $f): $($sig.Status)"
        if ($sig.Status -ne 'Valid') {
            Warn 'Signature is present but does not chain to a trusted root (normal for a self-signed cert until OrangStudio.cer is imported into Trusted Root Certification Authorities on this machine).'
        }
    }
}

Write-Host ''
Write-Host 'Done.' -ForegroundColor Green
if (-not $SkipSigning) {
    Write-Host 'Reminder: self-signed installers show a SmartScreen warning on other machines.'
    Write-Host 'To trust the signatures: double-click installer\OrangStudio.cer and install it into "Trusted Root Certification Authorities" (does not remove the SmartScreen reputation warning).'
}