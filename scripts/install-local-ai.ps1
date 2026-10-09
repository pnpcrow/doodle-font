param([string]$Destination = (Join-Path $env:LOCALAPPDATA 'DoodleFont\ai'))
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Destination | Out-Null
$files = @(
    @{Name='runtime.zip'; Url='https://github.com/ggml-org/llama.cpp/releases/download/b11531/llama-b11531-bin-win-vulkan-x64.zip'; Hash='e6b751b1678cb2a2503a98d974e14f5a18ab534f56f16660882ab310efe75ff6'},
    @{Name='embedding.gguf'; Url='https://huggingface.co/ggml-org/embeddinggemma-2-GGUF/resolve/bfcd298762cc34d0357ece5ebdd31791a3a374d8/embeddinggemma-2-Q8_0.gguf'; Hash='2188ac1deca4b77dffefd603c2776a9d76d9d74ec01841392982ebb840b09135'},
    @{Name='embedding-vision.gguf'; Url='https://huggingface.co/ggml-org/embeddinggemma-2-GGUF/resolve/bfcd298762cc34d0357ece5ebdd31791a3a374d8/mmproj-embeddinggemma-2-Q8_0.gguf'; Hash='c4a8a52691ecef40618438928bdf9e68379b854e24166f292592353db0aab64f'},
    @{Name='assistant.gguf'; Url='https://huggingface.co/ggml-org/gemma-4-E2B-it-GGUF/resolve/b4243c156154b6dca9324415f8c7ccc098b4aed1/gemma-4-E2B-it-Q4_0.gguf'; Hash='8e30dff3ac4c8434c49a7036fa15564bdbb6044e42bf04550bf1a096ad7e6a52'},
    @{Name='assistant-vision.gguf'; Url='https://huggingface.co/ggml-org/gemma-4-E2B-it-GGUF/resolve/b4243c156154b6dca9324415f8c7ccc098b4aed1/mmproj-gemma-4-E2B-it-Q8_0.gguf'; Hash='9406f99c16d68cda4f1f0552192dcc99021ea1fc6d2fd50b1dc3ccf30d04b292'}
)
foreach ($item in $files) {
    $path = Join-Path $Destination $item.Name
    if ((Test-Path -LiteralPath $path) -and (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -eq $item.Hash) { continue }
    Write-Output ('Downloading ' + $item.Name)
    & curl.exe --fail --location --silent --show-error --retry 3 --connect-timeout 30 --speed-limit 1024 --speed-time 120 --continue-at - --output ($path + '.partial') $item.Url
    if ($LASTEXITCODE -ne 0) { throw ('Download failed: ' + $item.Name) }
    if ((Get-FileHash -LiteralPath ($path + '.partial') -Algorithm SHA256).Hash -ne $item.Hash) { throw ('SHA256 mismatch: ' + $item.Name) }
    Move-Item -LiteralPath ($path + '.partial') -Destination $path -Force
}
Expand-Archive -LiteralPath (Join-Path $Destination 'runtime.zip') -DestinationPath (Join-Path $Destination 'runtime') -Force
$notices = @(
    @{Name='LICENSE-llama.cpp.txt'; Url='https://raw.githubusercontent.com/ggml-org/llama.cpp/b11531/LICENSE'},
    @{Name='LICENSE-Apache-2.0.txt'; Url='https://www.apache.org/licenses/LICENSE-2.0.txt'},
    @{Name='EmbeddingGemma-2-model-card.md'; Url='https://huggingface.co/google/embeddinggemma-2/raw/914f7f89142e33e77833254d9c9b90c3cef7303b/README.md'},
    @{Name='Gemma-4-E2B-model-card.md'; Url='https://huggingface.co/google/gemma-4-E2B-it/raw/3e22461f65e89153144f8adb70e3b8c2cc9845a7/README.md'}
)
foreach($notice in $notices) {
    & curl.exe --fail --location --silent --show-error --max-time 60 --output (Join-Path $Destination $notice.Name) $notice.Url
    if ($LASTEXITCODE -ne 0) { throw ('License download failed: ' + $notice.Name) }
}
$files | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $Destination 'installed.json') -Encoding utf8
Write-Output ('Local AI installed and verified: ' + $Destination)
