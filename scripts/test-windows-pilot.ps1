#requires -Version 7.0
# Preparation smoke only: never invokes an authenticated agent.
$ErrorActionPreference = 'Stop'
$root = Join-Path $env:TEMP ('forge pilot smoke ' + [guid]::NewGuid())
$keys = @('GIT_CONFIG_COUNT','GIT_CONFIG_KEY_0','GIT_CONFIG_VALUE_0','GIT_CONFIG_KEY_1','GIT_CONFIG_VALUE_1')
$saved = @{}
foreach ($key in $keys) { $saved[$key] = [Environment]::GetEnvironmentVariable($key, 'Process') }
try {
    $env:GIT_CONFIG_COUNT = '2'
    $env:GIT_CONFIG_KEY_0 = 'user.name'
    $env:GIT_CONFIG_VALUE_0 = 'Human Fixture'
    $env:GIT_CONFIG_KEY_1 = 'user.email'
    $env:GIT_CONFIG_VALUE_1 = 'human@example.invalid'
    $forge = (Resolve-Path 'target/x86_64-pc-windows-msvc/release/illumitech-forge.exe').Path
    & "$PSScriptRoot/windows-native-pilot.ps1" -Root $root -Forge $forge
    if (-not (Test-Path -LiteralPath (Join-Path $root 'native-pilot.json'))) { throw 'Missing pilot manifest' }
    if (Test-Path -LiteralPath (Join-Path $root 'state')) { throw 'Preparation created execution state' }
    $repo = Join-Path $root 'repo espaço 日本'
    if (Test-Path -LiteralPath (Join-Path $repo 'sum.ps1')) { throw 'Preparation executed a task' }
    Set-Content -LiteralPath (Join-Path $repo 'sum.ps1') -Value 'param([double]$A,[double]$B); $A + $B' -Encoding utf8
    & pwsh.exe -NoProfile -NonInteractive -File (Join-Path $root 'evaluators/verify.ps1') -Repo $repo
    if ($LASTEXITCODE -ne 0) { throw 'Prepared evaluator rejected valid fixture' }
    Set-Content -LiteralPath (Join-Path $repo 'sum.ps1') -Value 'param([double]$A,[double]$B); 0' -Encoding utf8
    & pwsh.exe -NoProfile -NonInteractive -File (Join-Path $root 'evaluators/verify.ps1') -Repo $repo 2>$null
    if ($LASTEXITCODE -eq 0) { throw 'Prepared evaluator accepted false implementation' }
    $global:LASTEXITCODE = 0
} finally {
    foreach ($key in $keys) { [Environment]::SetEnvironmentVariable($key, $saved[$key], 'Process') }
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
