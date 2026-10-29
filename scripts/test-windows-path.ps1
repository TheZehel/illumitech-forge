#requires -Version 7.0
# Installer metadata/PATH regression only; this fixture is not an executable.
$ErrorActionPreference = 'Stop'
$root = Join-Path $env:TEMP ('forge path ' + [guid]::NewGuid())
$original = [Environment]::GetEnvironmentVariable('Path', 'User')
try {
    $source = Join-Path $root 'source'
    New-Item -ItemType Directory -Path $source | Out-Null
    $exe = Join-Path $source 'illumitech-forge.exe'
    Set-Content -LiteralPath $exe 'fixture, not executable'
    $digest = (Get-FileHash -LiteralPath $exe).Hash
    Set-Content -LiteralPath (Join-Path $source 'SHA256SUMS') "$digest  illumitech-forge.exe"
    $archive = Join-Path $root 'fixture.zip'
    Compress-Archive -Path "$source/*" -DestinationPath $archive
    $hash = (Get-FileHash -LiteralPath $archive).Hash
    $destination = Join-Path $root 'app'
    foreach ($noPath in @($true,$true,$false,$false)) {
        & "$PSScriptRoot/install.ps1" -Archive $archive -Sha256 $hash -InstallDir $destination -NoPath:$noPath
        $entries = @([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' | Where-Object { $_ -eq $destination })
        $marker = Get-Content -LiteralPath (Join-Path $destination '.forge-install.json') -Raw | ConvertFrom-Json
        Write-Output "NoPath=$noPath matchingEntries=$($entries.Count) markerAdded=$($marker.pathAdded)"
        if (-not $noPath -and $entries.Count -ne 1) {
            $actual = [Environment]::GetEnvironmentVariable('Path', 'User')
            $explicit = [Environment]::GetEnvironmentVariable('Path', [EnvironmentVariableTarget]::User)
            $registry = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment').GetValue('Path')
            Write-Output "explicitLength=$($explicit.Length) registryLength=$($registry.Length) explicitMatches=$($explicit.Contains($destination)) registryMatches=$($registry.Contains($destination))"
            Write-Output ('Expected fixture: ' + ($destination | ConvertTo-Json -Compress))
            Write-Output ('Managed fixture: ' + ($marker.directory | ConvertTo-Json -Compress))
            Write-Output ('Fixture PATH entries: ' + (@($actual -split ';' | Where-Object { $_ -match 'forge path|forge install' }) | ConvertTo-Json -Compress))
            $index = $actual.IndexOf($root, [StringComparison]::OrdinalIgnoreCase)
            Write-Output "PATH length=$($actual.Length) expectedLength=$($destination.Length) rootIndex=$index endsWithExpected=$($actual.EndsWith($destination))"
            if ($index -ge 0) { Write-Output ('Fixture suffix: ' + ($actual.Substring($index) | ConvertTo-Json -Compress)) }
            throw 'PATH entry count is not one'
        }
    }
    & "$PSScriptRoot/uninstall.ps1" -InstallDir $destination
    if ([Environment]::GetEnvironmentVariable('Path', 'User') -ne $original) { throw 'Original PATH was not restored' }
} finally {
    [Environment]::SetEnvironmentVariable('Path', $original, 'User')
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
