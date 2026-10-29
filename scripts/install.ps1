#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Archive,
    [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$Sha256,
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs/IllumitechForge'),
    [switch]$NoPath
)
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'This installer requires Windows and PowerShell 7.' }
$Archive = (Resolve-Path -LiteralPath $Archive).Path
if ((Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash -ne $Sha256) { throw 'Archive SHA256 mismatch' }
$InstallDir = [IO.Path]::GetFullPath($InstallDir)
# Reject redirected destination ancestors before comparing or writing managed files.
function Assert-PlainPath([string]$Path) {
    while ($Path) {
        if ((Test-Path -LiteralPath $Path) -and ((Get-Item -Force -LiteralPath $Path).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Reparse-point destination refused' }
        $Path = [IO.Path]::GetDirectoryName($Path)
    }
}
Assert-PlainPath $InstallDir
$marker = Join-Path $InstallDir '.forge-install.json'
Assert-PlainPath $marker
if (Test-Path -LiteralPath $InstallDir) {
    if (-not (Test-Path -LiteralPath $marker)) { throw 'Unmanaged destination; preserving existing files' }
}
$temp = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
try {
    # Reject traversal, duplicate paths and non-file ZIP entries before extraction.
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        foreach ($entry in $zip.Entries) {
            if ($entry.FullName -match '(^/|\\|:|(^|/)\.\.(/|$))' -or -not $seen.Add($entry.FullName)) { throw 'Unsafe ZIP path' }
        }
    } finally { $zip.Dispose() }
    Expand-Archive -LiteralPath $Archive -DestinationPath $temp
    $files = @{}
    foreach ($line in Get-Content -LiteralPath (Join-Path $temp 'SHA256SUMS')) {
        if ($line -notmatch '^([a-fA-F0-9]{64})  (.+)$') { throw 'Invalid SHA256SUMS' }
        $hash, $name = $Matches[1], $Matches[2]
        if ($name -match '(^/|\\|:|(^|/)\.\.(/|$))' -or $files.ContainsKey($name) -or $name -eq '.forge-install.json') { throw 'Unsafe checksum path' }
        $source = Join-Path $temp $name
        if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ne $hash) { throw "File checksum mismatch: $name" }
        $files[$name] = $hash
    }
    if (-not $files.ContainsKey('illumitech-forge.exe')) { throw 'Missing executable' }
    foreach ($file in Get-ChildItem -LiteralPath $temp -File -Recurse) {
        $name = [IO.Path]::GetRelativePath($temp, $file.FullName).Replace('\','/')
        if ($name -ne 'SHA256SUMS' -and -not $files.ContainsKey($name)) { throw "Unlisted file: $name" }
    }
    # Preflight every destination. Upgrades require explicit uninstall; never overwrite user files.
    $files['SHA256SUMS'] = (Get-FileHash -LiteralPath (Join-Path $temp 'SHA256SUMS')).Hash
    foreach ($name in $files.Keys) {
        $dest = Join-Path $InstallDir $name
        Assert-PlainPath $dest
        if ((Test-Path -LiteralPath $dest) -and (Get-FileHash -LiteralPath $dest).Hash -ne $files[$name]) { throw "Existing file differs: $name" }
    }
    $old = if (Test-Path -LiteralPath $marker) { Get-Content -Raw -LiteralPath $marker | ConvertFrom-Json } else { $null }
    if ($old -and ($old.schema -ne 1 -or $old.directory -ne $InstallDir)) { throw 'Invalid installation marker' }
    foreach ($name in $files.Keys) {
        $dest = Join-Path $InstallDir $name
        [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($dest)) | Out-Null
        if (-not (Test-Path -LiteralPath $dest)) { Copy-Item -LiteralPath (Join-Path $temp $name) -Destination $dest }
    }
    $path = [string][Environment]::GetEnvironmentVariable('Path', 'User')
    $added = $old -and $old.pathAdded
    if (-not $NoPath -and $InstallDir -notin ($path -split ';')) {
        [Environment]::SetEnvironmentVariable('Path', (($path.TrimEnd(';') + ';' + $InstallDir).TrimStart(';')), 'User')
        $added = $true
    }
    @{ schema = 1; directory = $InstallDir; pathAdded = [bool]$added; files = $files } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $marker
    Write-Output "Installed in $InstallDir. Open a new terminal if PATH was updated."
} finally { if (Test-Path -LiteralPath $temp) { Remove-Item -LiteralPath $temp -Recurse -Force } }
