#requires -Version 7.0
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Root, [string]$Forge = 'illumitech-forge', [switch]$Run)
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Run this pilot on native Windows with PowerShell 7.' }
$Root = [IO.Path]::GetFullPath($Root)
if (Test-Path -LiteralPath $Root) { throw 'Use a new pilot directory; existing files are preserved.' }
& $Forge --version
if ($LASTEXITCODE -ne 0) { throw 'Forge executable unavailable' }
$name = & git config --get user.name
$email = & git config --get user.email
if (-not $name -or -not $email) { throw 'Configure your human Git identity before the pilot.' }
if ($name -match 'codex' -or $email -match 'codex') { throw 'Use a human Git identity.' }
$repo = Join-Path $Root 'repo espaço 日本'
$evaluators = Join-Path $Root 'evaluators'
$state = Join-Path $Root 'state'
New-Item -ItemType Directory -Path $repo,$evaluators | Out-Null
& git -C $repo init -q
if ($LASTEXITCODE -ne 0) { throw 'Git init failed' }
Set-Content -LiteralPath (Join-Path $repo 'README.md') -Value 'preserve-pilot-sentinel' -NoNewline -Encoding utf8
& git -C $repo add README.md
& git -C $repo commit -qm 'test: prepare native Windows acceptance fixture'
if ($LASTEXITCODE -ne 0) { throw 'Human fixture commit failed' }
$evaluator = Join-Path $evaluators 'verify.ps1'
@'
param([Parameter(Mandatory)][string]$Repo)
$ErrorActionPreference = 'Stop'
if ([IO.File]::ReadAllText((Join-Path $Repo 'README.md')) -cne 'preserve-pilot-sentinel') { throw 'Sentinel changed' }
foreach ($case in @(@(2,3,5), @(-4,1,-3), @(1.5,2.25,3.75))) {
    $value = & pwsh.exe -NoProfile -NonInteractive -File (Join-Path $Repo 'sum.ps1') -A $case[0] -B $case[1]
    if ($LASTEXITCODE -ne 0 -or [double]$value -ne $case[2]) { throw 'Addition behavior failed' }
}
$changed = @(& git -C $Repo diff HEAD --name-only)
$untracked = @(& git -C $Repo ls-files --others --exclude-standard)
if ($LASTEXITCODE -ne 0 -or $changed.Count -ne 0 -or $untracked.Count -ne 1 -or $untracked[0] -ne 'sum.ps1') { throw 'Unexpected files changed' }
Write-Output 'Native behavior and file preservation passed'
'@ | Set-Content -LiteralPath $evaluator -Encoding utf8
$hash = (Get-FileHash -LiteralPath $evaluator -Algorithm SHA256).Hash.ToLowerInvariant()
$acceptance = @(@{kind='gate';id='behavior'})
$manifest = Join-Path $Root 'native-pilot.json'
@{
    schema_version=1; release_id='windows-native-pilot'
    repos=@{app=@{path=$repo}}
    tasks=@(
        @{id='implement';repo='app';description='Create only sum.ps1 with double parameters A and B and print their sum. Preserve README.md exactly. Do not stage, commit, push, change configuration, write memory, or modify evaluator/state files. The external evaluator determines acceptance.';executor=@{kind='codex';permission='workspace_write'};acceptance=$acceptance;max_attempts=1;timeout_seconds=300},
        @{id='review';repo='app';dependencies=@('implement');description='Read sum.ps1 and report whether it adds double parameters A and B. Do not edit any files, commit, push, change configuration, delegate or write memory. External gates determine acceptance.';executor=@{kind='codex';permission='read_only'};acceptance=$acceptance;max_attempts=1;timeout_seconds=180}
    )
    gates=@(@{id='behavior';argv=@('pwsh.exe','-NoProfile','-NonInteractive','-File',$evaluator,'{repo}');inputs=@{$evaluator=$hash}})
} | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $manifest -Encoding utf8
& $Forge --state-dir $state release inspect $manifest
if ($LASTEXITCODE -ne 0) { throw 'Inspect failed' }
& $Forge --state-dir $state release run $manifest --dry-run
if ($LASTEXITCODE -ne 0) { throw 'Dry-run failed' }
if ($Run) {
    # Explicit opt-in: uses the already authenticated native client and its configured model.
    & $Forge --state-dir $state release run $manifest
    if ($LASTEXITCODE -ne 0) { throw "Pilot did not pass. Inspect report using manifest $manifest and state $state." }
    & $Forge --state-dir $state report $manifest
    if ($LASTEXITCODE -ne 0) { throw 'Report failed' }
} else {
    Write-Output "Prepared only; no agent invoked. Review $manifest, then run Forge release run with state directory $state."
}
