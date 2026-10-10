param([string]$Destination = (Join-Path $env:LOCALAPPDATA 'DoodleFont\ai'))
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Destination | Out-Null
$manifestPath = Join-Path (Split-Path -Parent $PSScriptRoot) 'assets/local-ai-manifest.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$files = $manifest.files
foreach ($item in $files) {
    $path = Join-Path $Destination $item.Name
    if ((Test-Path -LiteralPath $path) -and (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -eq $item.sha256) { continue }
    Write-Output ('Downloading ' + $item.Name)
    & curl.exe --fail --location --silent --show-error --retry 3 --connect-timeout 30 --speed-limit 1024 --speed-time 120 --continue-at - --output ($path + '.partial') $item.Url
    if ($LASTEXITCODE -ne 0) { throw ('Download failed: ' + $item.Name) }
    if ((Get-FileHash -LiteralPath ($path + '.partial') -Algorithm SHA256).Hash -ne $item.sha256) { throw ('SHA256 mismatch: ' + $item.Name) }
    Move-Item -LiteralPath ($path + '.partial') -Destination $path -Force
}
Expand-Archive -LiteralPath (Join-Path $Destination 'runtime.zip') -DestinationPath (Join-Path $Destination 'runtime') -Force
$notices = @(
    @{Name='LICENSE-llama.cpp.txt'; Url='https://raw.githubusercontent.com/ggml-org/llama.cpp/b11531/LICENSE'},
    @{Name='LICENSE-Apache-2.0.txt'; Url='https://www.apache.org/licenses/LICENSE-2.0.txt'},
    @{Name='Gemma-4-E2B-model-card.md'; Url='https://huggingface.co/google/gemma-4-E2B-it/raw/3e22461f65e89153144f8adb70e3b8c2cc9845a7/README.md'}
)
foreach($notice in $notices) {
    & curl.exe --fail --location --silent --show-error --max-time 60 --output (Join-Path $Destination $notice.Name) $notice.Url
    if ($LASTEXITCODE -ne 0) { throw ('License download failed: ' + $notice.Name) }
}
$files | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $Destination 'installed.json') -Encoding utf8
Write-Output ('Local AI installed and verified: ' + $Destination)
