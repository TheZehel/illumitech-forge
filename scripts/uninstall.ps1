#requires -Version 7.0
[CmdletBinding()]
param([string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs/IllumitechForge'))
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'This uninstaller requires Windows and PowerShell 7.' }
$InstallDir = [IO.Path]::GetFullPath($InstallDir)
function Assert-PlainPath([string]$Path) {
    while ($Path) {
        if ((Test-Path -LiteralPath $Path) -and ((Get-Item -Force -LiteralPath $Path).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Reparse-point destination refused' }
        $Path = [IO.Path]::GetDirectoryName($Path)
    }
}
Assert-PlainPath $InstallDir
$marker = Join-Path $InstallDir '.forge-install.json'
Assert-PlainPath $marker
if (-not (Test-Path -LiteralPath $marker)) { Write-Output 'No managed installation; no changes.'; return }
$state = Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json -AsHashtable
if ($state.schema -ne 1 -or $state.directory -ne $InstallDir) { throw 'Invalid installation marker' }
foreach ($name in $state.files.Keys) {
    if ($name -match '(^/|\\|:|(^|/)\.\.(/|$))') { throw 'Invalid managed path' }
    $file = Join-Path $InstallDir $name
    Assert-PlainPath $file
    if ((Test-Path -LiteralPath $file) -and (Get-FileHash -LiteralPath $file).Hash -ne $state.files[$name]) { throw "Modified file preserved: $name" }
}
foreach ($name in $state.files.Keys) {
    $file = Join-Path $InstallDir $name
    if (Test-Path -LiteralPath $file) { Remove-Item -LiteralPath $file }
}
if ($state.pathAdded) {
    $path = [Environment]::GetEnvironmentVariable('Path', 'User')
    [Environment]::SetEnvironmentVariable('Path', (($path -split ';' | Where-Object { $_ -ne $InstallDir }) -join ';'), 'User')
}
Remove-Item -LiteralPath $marker
# Only remove empty directories. Unrelated files remain untouched.
$parents = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($name in $state.files.Keys) {
    $parent = [IO.Path]::GetDirectoryName((Join-Path $InstallDir $name))
    while ($parent -and $parent -ne $InstallDir) { $null = $parents.Add($parent); $parent = [IO.Path]::GetDirectoryName($parent) }
}
$parents | Sort-Object Length -Descending | ForEach-Object {
    if ((Test-Path -LiteralPath $_) -and -not (Get-ChildItem -LiteralPath $_ -Force)) { Remove-Item -LiteralPath $_ }
}
if (-not (Get-ChildItem -LiteralPath $InstallDir -Force)) { Remove-Item -LiteralPath $InstallDir }
