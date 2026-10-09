$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repo
cargo test --locked
if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
cargo build --release --locked --bin doodle-font
if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
New-Item -ItemType Directory -Force -Path 'release' | Out-Null
Copy-Item -LiteralPath 'target\release\doodle-font.exe' -Destination 'release\Doodle-Font.exe'
Copy-Item -LiteralPath 'README.md' -Destination 'release\README.md'
Copy-Item -LiteralPath 'THIRD_PARTY_NOTICES.md' -Destination 'release\THIRD_PARTY_NOTICES.md'
(Get-FileHash -LiteralPath 'release\Doodle-Font.exe' -Algorithm SHA256).Hash | Set-Content -LiteralPath 'release\SHA256.txt' -Encoding utf8
Write-Output 'Created release\Doodle-Font.exe'
