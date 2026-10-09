$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repo
cargo build --release --locked --features ui-qa --bin doodle-font
if($LASTEXITCODE -ne 0){throw 'UI build failed'}
$fixture = Join-Path $repo ('artifacts/install-ui-' + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $fixture | Out-Null
$qaExe = Join-Path $fixture 'Doodle-Font-QA.exe'
Copy-Item -LiteralPath 'target/release/doodle-font.exe' -Destination $qaExe
$previousAi = $env:DOODLE_AI_DIR
$previousData = $env:DOODLE_DATA_DIR
try {
    $env:DOODLE_AI_DIR = $fixture
    $env:DOODLE_DATA_DIR = Join-Path $fixture 'project'
    $env:DOODLE_QA_VIEW = 'install-missing'
    $env:DOODLE_QA_OUTPUT = Join-Path $repo 'artifacts/qa-install-missing.png'
    $qaProcess = Start-Process -FilePath $qaExe -WindowStyle Hidden -Wait -PassThru
    if($qaProcess.ExitCode -ne 0){throw 'Missing-install view failed'}
    $cache = Join-Path $env:LOCALAPPDATA 'DoodleFont/ai'
    $manifest = Get-Content -LiteralPath 'assets/local-ai-manifest.json' -Raw | ConvertFrom-Json
    foreach($asset in $manifest.files){
        # Share only already-downloaded, read-only input files with the isolated fixture.
        New-Item -ItemType HardLink -Path (Join-Path $fixture $asset.name) -Target (Join-Path $cache $asset.name) | Out-Null
    }
    $env:DOODLE_QA_VIEW = 'install-flow'
    $env:DOODLE_QA_OUTPUT = Join-Path $repo 'artifacts/qa-install-complete.png'
    $qaProcess = Start-Process -FilePath $qaExe -WindowStyle Hidden -Wait -PassThru
    if($qaProcess.ExitCode -ne 0){throw 'Install button flow failed'}
    if(-not(Test-Path -LiteralPath (Join-Path $fixture 'installed.json'))){throw 'Install marker missing'}
    if(-not(Test-Path -LiteralPath (Join-Path $fixture 'runtime/llama-server.exe'))){throw 'Installed runtime missing'}
    Write-Output ('Native install button, cache verification, extraction, completion and AI availability passed: ' + $fixture)
} finally {
    $env:DOODLE_AI_DIR=$previousAi
    $env:DOODLE_DATA_DIR=$previousData
    Remove-Item Env:\DOODLE_QA_VIEW,Env:\DOODLE_QA_OUTPUT -ErrorAction SilentlyContinue
}
