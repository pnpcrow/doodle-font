param([switch]$LocalAi)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repo
cargo build --features ui-qa --bin doodle-font
if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
$runId = [guid]::NewGuid().ToString()
$env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId"
$views = @('editor','preview','templates','settings','export','hangul','ai-ready','ai-busy','ai-recover','interaction','ai-candidates','ai-candidate-accept','ai-candidate-invalid','ai-typography')
if ($LocalAi) { $views += 'ai-candidate-live','ai-typography-live' }
foreach ($viewName in $views) {
    $env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId-$viewName"
    $env:DOODLE_QA_VIEW = $viewName
    $env:DOODLE_QA_OUTPUT = Join-Path $repo "artifacts\qa-$viewName.png"
    & '.\target\debug\doodle-font.exe'
    if ($LASTEXITCODE -ne 0) { throw "UI QA failed: $viewName" }
    if (-not (Test-Path -LiteralPath $env:DOODLE_QA_OUTPUT)) { throw "Screenshot missing: $viewName" }
}
Remove-Item Env:\DOODLE_QA_VIEW,Env:\DOODLE_QA_OUTPUT,Env:\DOODLE_DATA_DIR
Write-Output 'Native UI QA passed, including drawing, AI candidate review/apply, invalid responses, undo/redo, original preservation, and disk save.'
