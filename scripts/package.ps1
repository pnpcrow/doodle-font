param([string]$OutputDirectory = 'release')
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repo
cargo test --locked
if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
cargo build --release --locked --bin doodle-font
if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
$destination = [IO.Path]::GetFullPath((Join-Path $repo $OutputDirectory))
New-Item -ItemType Directory -Force -Path $destination | Out-Null
Copy-Item -LiteralPath 'target\release\doodle-font.exe' -Destination (Join-Path $destination 'Doodle-Font.exe')
Copy-Item -LiteralPath 'README.md' -Destination (Join-Path $destination 'README.md')
Copy-Item -LiteralPath 'THIRD_PARTY_NOTICES.md' -Destination (Join-Path $destination 'THIRD_PARTY_NOTICES.md')
New-Item -ItemType Directory -Force -Path (Join-Path $destination 'scripts'),(Join-Path $destination 'licenses'),(Join-Path $destination 'assets') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $destination 'licenses\typefaces') | Out-Null
Copy-Item -LiteralPath 'assets\fonts\Noto-OFL.txt','assets\fonts\SingleDay-OFL.txt','assets\fonts\manifest.json' -Destination (Join-Path $destination 'licenses\typefaces')
Copy-Item -LiteralPath 'assets/local-ai-manifest.json' -Destination (Join-Path $destination 'assets/local-ai-manifest.json')
Copy-Item -LiteralPath 'assets/needle-ai-manifest.json' -Destination (Join-Path $destination 'assets/needle-ai-manifest.json')
Copy-Item -LiteralPath 'scripts\install-local-ai.ps1' -Destination (Join-Path $destination 'scripts\install-local-ai.ps1')
$metadata = cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Dependency license inventory failed' }
$inventory = foreach ($package in $metadata.packages) {
    if (-not $package.source) { continue }
    $packageRoot = Split-Path -Parent $package.manifest_path
    $licenseTarget = Join-Path (Join-Path $destination 'licenses') ($package.name + '-' + $package.version)
    New-Item -ItemType Directory -Force -Path $licenseTarget | Out-Null
    foreach ($license in (Get-ChildItem -LiteralPath $packageRoot -Recurse -File | Where-Object Name -Match '^(LICENSE|COPYING|OFL|UFL|NOTICE)')) {
        $relative = [IO.Path]::GetRelativePath($packageRoot, $license.FullName)
        $target = Join-Path $licenseTarget $relative
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target) | Out-Null
        Copy-Item -LiteralPath $license.FullName -Destination $target
    }
    @{name=$package.name;version=$package.version;license=$package.license;repository=$package.repository}
}
$inventory | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $destination 'licenses\crates.json') -Encoding utf8
(Get-FileHash -LiteralPath (Join-Path $destination 'Doodle-Font.exe') -Algorithm SHA256).Hash | Set-Content -LiteralPath (Join-Path $destination 'SHA256.txt') -Encoding utf8
Write-Output ('Created ' + (Join-Path $destination 'Doodle-Font.exe'))
