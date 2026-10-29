#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$archive = (Get-ChildItem dist/*windows*.zip | Select-Object -First 1).FullName
$hash = (Get-FileHash -LiteralPath $archive).Hash
$originalPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$root = Join-Path $env:TEMP ('forge install café ' + [guid]::NewGuid())
$install = Join-Path $root 'app'
try {
    try { & "$PSScriptRoot/install.ps1" -Archive $archive -Sha256 ('0' * 64) -InstallDir $install -NoPath; throw 'Accepted bad checksum' }
    catch { if ($_.Exception.Message -ne 'Archive SHA256 mismatch') { throw } }
    if (Test-Path $install) { throw 'Checksum failure changed destination' }
    & "$PSScriptRoot/install.ps1" -Archive $archive -Sha256 $hash -InstallDir $install -NoPath
    & "$PSScriptRoot/install.ps1" -Archive $archive -Sha256 $hash -InstallDir $install -NoPath
    & (Join-Path $install 'illumitech-forge.exe') --version
    if ($LASTEXITCODE -ne 0) { throw 'Installed executable failed' }
    & "$PSScriptRoot/install.ps1" -Archive $archive -Sha256 $hash -InstallDir $install
    & "$PSScriptRoot/install.ps1" -Archive $archive -Sha256 $hash -InstallDir $install
    if (([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' | Where-Object { $_ -eq $install }).Count -ne 1) { throw 'PATH is not idempotent' }
    Set-Content -LiteralPath (Join-Path $install 'unrelated.txt') 'preserve'
    $license = Join-Path $install 'LICENSE'
    $original = [IO.File]::ReadAllBytes($license)
    Add-Content -LiteralPath $license 'local edit'
    try { & "$PSScriptRoot/uninstall.ps1" -InstallDir $install; throw 'Removed modified file' }
    catch { if ($_.Exception.Message -notlike 'Modified file preserved:*') { throw } }
    [IO.File]::WriteAllBytes($license, $original)
    & "$PSScriptRoot/uninstall.ps1" -InstallDir $install
    & "$PSScriptRoot/uninstall.ps1" -InstallDir $install
    if ((Get-Content -LiteralPath (Join-Path $install 'unrelated.txt')) -ne 'preserve') { throw 'Lost unrelated file' }
    if (Test-Path (Join-Path $install 'illumitech-forge.exe')) { throw 'Uninstall left managed executable' }
    if ([Environment]::GetEnvironmentVariable('Path', 'User') -ne $originalPath) { throw 'PATH not restored' }
} finally { [Environment]::SetEnvironmentVariable('Path', $originalPath, 'User'); if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force } }
