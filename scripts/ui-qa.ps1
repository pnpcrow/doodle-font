param([switch]$LocalAi, [switch]$DecisionAi, [switch]$ConnectionsOnly, [switch]$FloatingOnly, [switch]$ExportOnly, [switch]$UsabilityOnly, [switch]$CompanionOnly, [switch]$SettingsOnly, [switch]$LayoutOnly, [switch]$LanguageOnly)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repo
cargo build --features ui-qa --bin doodle-font
if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
$runId = [guid]::NewGuid().ToString()
$env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId"
$views = @('theme-light','theme-light-small','theme-dark-small','appearance-write','appearance-restored','editor','preview','templates','settings','export','hangul','ai-ready','ai-busy','ai-recover','interaction','ai-candidates','ai-candidate-accept','ai-candidate-invalid','ai-typography','ai-layout','decision-settings')
$views += 'companion-inspecting','companion-drawing','companion-coloring','companion-explaining'
$views += 'ai-busy-small','ai-busy-light'
$rendererViews = @('renderer-write','renderer-restored','renderer-image','renderer-compare-small')
$views += $rendererViews
if ($LocalAi) { $views += 'ai-candidate-live','ai-typography-live' }
if ($DecisionAi) { $views += 'decision-fast-live','decision-gemma-live' }
if ($ConnectionsOnly) { $views = @() }
$views += 'connections-write','connections-restored','connections-delete','connections-empty'
if (-not $ConnectionsOnly) { $views += 'ai-floating','ai-floating-small' }
if ($FloatingOnly) { $views = @('ai-floating','ai-floating-small','ai-floating-reduced','ai-busy','ai-busy-small','ai-busy-light','ai-recover','ai-candidate-accept') }
$exportViews = @('export-ttf','export-otf','export-woff','export-woff2','export-woff2-small')
if ($ExportOnly) { $views = $exportViews }
elseif (-not $ConnectionsOnly -and -not $FloatingOnly) { $views += $exportViews }
if ($UsabilityOnly) { $views = @('theme-light','theme-light-small','theme-dark-small','appearance-write','appearance-restored','preview','templates','ai-busy') }
if ($CompanionOnly) { $views = $rendererViews }
if ($SettingsOnly) { $views = @('settings-off','settings-local','settings-api','settings-api-small','settings-api-expanded-small','settings-api-test','settings-api-error','settings-generation','settings-disable','settings-needle-auto','settings-local-auto','connections-write','connections-restored','connections-delete','connections-empty','settings-project','settings-dock-collapsed','settings-dock-transition') }
$layoutViews = @('layout-window','layout-corners','layout-corners-fallback','layout-project','layout-project-small','layout-tabs','layout-templates-light','layout-templates-dark-small','layout-sidebar','layout-sidebar-small','layout-sidebar-reduced','layout-companion-small')
$layoutViews += 'layout-companion-drag-write','layout-companion-drag-restored-small','layout-companion-drag-reset'
if ($LayoutOnly) { $views = $layoutViews }
elseif (-not ($ConnectionsOnly -or $FloatingOnly -or $ExportOnly -or $UsabilityOnly -or $CompanionOnly -or $SettingsOnly)) { $views += $layoutViews }
if ($LanguageOnly) {
    $views = @('language-en-write','language-en-restored','language-ja-write','language-ja-restored','language-ko-write','language-ko-restored','language-system-write','language-system-restored','language-en-editor-small','language-ja-editor-small','language-en-templates-small','language-ja-templates-small','language-en-export-small','language-ja-export-small','language-en-api-small','language-ja-api-small')
}
foreach ($viewName in $views) {
    $env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId-$viewName"
    if ($viewName.StartsWith('appearance-')) { $env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId-appearance" }
    if ($viewName.StartsWith('renderer-')) { $env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId-renderer" }
    if ($viewName.StartsWith('connections-')) { $env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId-connections" }
    if ($viewName.StartsWith('layout-companion-drag-')) { $env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId-companion-placement" }
    if ($viewName.StartsWith('language-')) { $env:DOODLE_DATA_DIR = Join-Path $repo "artifacts\ui-qa-$runId-language-$($viewName.Split('-')[1])" }
    if ($SettingsOnly -and $viewName.EndsWith('-auto')) {
        $packageDir = Join-Path $env:DOODLE_DATA_DIR 'models'
        New-Item -ItemType Directory -Path $packageDir -Force | Out-Null
        $cacheDir = Join-Path $env:LOCALAPPDATA 'DoodleFont\ai'
        $manifestPath = Join-Path $repo 'assets\local-ai-manifest.json'
        if ($viewName -eq 'settings-needle-auto') { $cacheDir = Join-Path $cacheDir 'decision'; $manifestPath = Join-Path $repo 'assets\needle-ai-manifest.json'; $env:DOODLE_NEEDLE_DIR = $packageDir }
        else { $env:DOODLE_AI_DIR = $packageDir }
        # Reuse verified cached payloads in an isolated install, without a multi-GB download.
        foreach ($asset in (Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json).files) {
            $cachedFile = Join-Path $cacheDir $asset.name
            if (-not (Test-Path -LiteralPath $cachedFile)) { throw "Cached model required for installation UI QA: $cachedFile" }
            New-Item -ItemType HardLink -Path (Join-Path $packageDir $asset.name) -Value $cachedFile | Out-Null
        }
    }
    $env:DOODLE_QA_VIEW = $viewName
    $env:DOODLE_QA_OUTPUT = Join-Path $repo "artifacts\qa-$viewName.png"
    & '.\target\debug\doodle-font.exe'
    if ($LASTEXITCODE -ne 0) { throw "UI QA failed: $viewName" }
    if (-not (Test-Path -LiteralPath $env:DOODLE_QA_OUTPUT)) { throw "Screenshot missing: $viewName" }
    if ($viewName -eq 'layout-window') {
        $savedProject = Get-Content -LiteralPath (Join-Path $env:DOODLE_DATA_DIR 'project.json') -Raw | ConvertFrom-Json
        if ($savedProject.family -ne 'Window Close Save Check') { throw 'Custom close button did not save the latest project edits' }
    }
    if ($SettingsOnly -and $viewName.EndsWith('-auto')) { Remove-Item Env:\DOODLE_AI_DIR,Env:\DOODLE_NEEDLE_DIR -ErrorAction SilentlyContinue }
}
Remove-Item Env:\DOODLE_QA_VIEW,Env:\DOODLE_QA_OUTPUT,Env:\DOODLE_DATA_DIR
Write-Output "Native UI QA passed: $($views.Count) scenarios."
if ($LanguageOnly) { Write-Output 'Language dropdown, immediate switching, restart, unchanged project data, and translated small-window editor/templates/export/settings passed.'; return }
if ($LayoutOnly) { Write-Output 'Native window corners (DWM and fallback), maximize/fullscreen/restore/resize/close, exit autosave, project editing, tabs, template cards, animated sidebars, companion dragging/persistence/reset, and context menu passed.'; return }
if ($SettingsOnly) { Write-Output 'Compact settings, API test success/error, separate generation window, disabled AI, isolated automatic installs, and saved credentials passed.'; return }
if ($CompanionOnly) { Write-Output 'Side-by-side comparison, selection, process restart, switching back, and small-window bounds passed.'; return }
if ($UsabilityOnly) { Write-Output 'Theme switching, preference restart, small windows, themed previews, and the busy companion passed.'; return }
if ($ExportOnly) { Write-Output 'Format selection, actual export checks, and the small-window export dialog passed.'; return }
if (-not $FloatingOnly) { Write-Output 'API key entry, automatic encryption, process restart, key deletion, and restart after deletion passed.' }
if (-not $ConnectionsOnly -and -not $FloatingOnly) {
    Write-Output 'Drawing, AI candidate review/apply, invalid responses, undo/redo, original preservation, and disk save also passed.'
    Write-Output 'Mock candidates verify UI behavior only. Live candidate screenshots also require a separate visual check of the requested glyph and style.'
}
if (-not $ConnectionsOnly) { Write-Output 'Floating dock preserves canvas size. Long responses and prompts scroll independently; collapse/expand and busy locking passed.' }
