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
New-Item -ItemType Directory -Force -Path 'release\scripts','release\licenses' | Out-Null
Copy-Item -LiteralPath 'scripts\install-local-ai.ps1' -Destination 'release\scripts\install-local-ai.ps1'
$metadata = cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Dependency license inventory failed' }
$inventory = foreach ($package in $metadata.packages) {
    if (-not $package.source) { continue }
    $packageRoot = Split-Path -Parent $package.manifest_path
    $licenseTarget = Join-Path 'release\licenses' ($package.name + '-' + $package.version)
    New-Item -ItemType Directory -Force -Path $licenseTarget | Out-Null
    foreach ($license in (Get-ChildItem -LiteralPath $packageRoot -Recurse -File | Where-Object Name -Match '^(LICENSE|COPYING|OFL|UFL|NOTICE)')) {
        $relative = [IO.Path]::GetRelativePath($packageRoot, $license.FullName)
        $target = Join-Path $licenseTarget $relative
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target) | Out-Null
        Copy-Item -LiteralPath $license.FullName -Destination $target
    }
    @{name=$package.name;version=$package.version;license=$package.license;repository=$package.repository}
}
$inventory | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath 'release\licenses\crates.json' -Encoding utf8
(Get-FileHash -LiteralPath 'release\Doodle-Font.exe' -Algorithm SHA256).Hash | Set-Content -LiteralPath 'release\SHA256.txt' -Encoding utf8
Write-Output 'Created release\Doodle-Font.exe'
