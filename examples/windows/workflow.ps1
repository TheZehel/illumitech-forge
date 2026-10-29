#requires -Version 7.0
# Run commands individually after reviewing the external manifest.
param([Parameter(Mandatory)][string]$Manifest, [Parameter(Mandatory)][string]$StateDir)
$ErrorActionPreference = 'Stop'
# Keep both paths outside the agent-writable repository.
illumitech-forge --state-dir $StateDir release inspect $Manifest
illumitech-forge --state-dir $StateDir release run --dry-run $Manifest
# Explicit execution and controls (uncomment the selected action):
# illumitech-forge --state-dir $StateDir release run $Manifest
# illumitech-forge --state-dir $StateDir status $Manifest
# illumitech-forge --state-dir $StateDir pause $Manifest
# illumitech-forge --state-dir $StateDir resume $Manifest
# illumitech-forge --state-dir $StateDir cancel $Manifest
# illumitech-forge release import-spec --root 'C:\Work\Example repo' --spec '001-feature' --release-id 'example'
# Native executor argv example in a JSON/YAML manifest:
# ["pwsh.exe", "-NoProfile", "-NonInteractive", "-File", "C:\\Work\\Scripts\\build.ps1"]
# Memory configuration is optional; Claude remains experimental.
