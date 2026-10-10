param([switch]$LocalAi, [switch]$DecisionAi, [switch]$ConnectionsOnly, [switch]$FloatingOnly, [switch]$ExportOnly, [switch]$UsabilityOnly)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repo
cargo build --features ui-qa --bin doodle-font
if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
$runId = [guid]::NewGuid().ToString()
$env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId"
$views = @('theme-light','theme-light-small','theme-dark-small','appearance-write','appearance-restored','editor','preview','templates','settings','export','hangul','ai-ready','ai-busy','ai-recover','interaction','ai-candidates','ai-candidate-accept','ai-candidate-invalid','ai-typography','ai-layout','decision-settings')
$views += 'companion-inspecting','companion-drawing','companion-coloring','companion-explaining'
if ($LocalAi) { $views += 'ai-candidate-live','ai-typography-live' }
if ($DecisionAi) { $views += 'decision-fast-live','decision-gemma-live' }
if ($ConnectionsOnly) { $views = @() }
$views += 'connections-write','connections-restored','connections-delete','connections-empty'
if (-not $ConnectionsOnly) { $views += 'ai-floating','ai-floating-small' }
if ($FloatingOnly) { $views = @('ai-floating','ai-floating-small','ai-busy','ai-recover','ai-candidate-accept') }
$exportViews = @('export-ttf','export-otf','export-woff','export-woff2','export-woff2-small')
if ($ExportOnly) { $views = $exportViews }
elseif (-not $ConnectionsOnly -and -not $FloatingOnly) { $views += $exportViews }
if ($UsabilityOnly) { $views = @('theme-light','theme-light-small','theme-dark-small','appearance-write','appearance-restored','preview','templates','ai-busy') }
foreach ($viewName in $views) {
    $env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId-$viewName"
    if ($viewName.StartsWith('appearance-')) { $env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId-appearance" }
    if ($viewName.StartsWith('connections-')) { $env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId-connections" }
    $env:DOODLE_QA_VIEW = $viewName
    $env:DOODLE_QA_OUTPUT = Join-Path $repo "artifacts\qa-$viewName.png"
    & '.\target\debug\doodle-font.exe'
    if ($LASTEXITCODE -ne 0) { throw "UI QA failed: $viewName" }
    if (-not (Test-Path -LiteralPath $env:DOODLE_QA_OUTPUT)) { throw "Screenshot missing: $viewName" }
}
Remove-Item Env:\DOODLE_QA_VIEW,Env:\DOODLE_QA_OUTPUT,Env:\DOODLE_DATA_DIR
Write-Output "Native UI QA passed: $($views.Count) scenarios."
if ($UsabilityOnly) { Write-Output 'Theme switching, preference restart, small windows, themed previews, and the busy companion passed.'; return }
if ($ExportOnly) { Write-Output 'Format selection, actual export checks, and the small-window export dialog passed.'; return }
if (-not $FloatingOnly) { Write-Output 'API key entry, automatic encryption, process restart, key deletion, and restart after deletion passed.' }
if (-not $ConnectionsOnly -and -not $FloatingOnly) {
    Write-Output 'Drawing, AI candidate review/apply, invalid responses, undo/redo, original preservation, and disk save also passed.'
    Write-Output 'Mock candidates verify UI behavior only. Live candidate screenshots also require a separate visual check of the requested glyph and style.'
}
if (-not $ConnectionsOnly) { Write-Output 'Floating dock preserves canvas size. Long responses and prompts scroll independently; collapse/expand and busy locking passed.' }
