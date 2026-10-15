param(
    [switch]$SkipBuild,
    [ValidateSet('x64', 'x86')]
    [string[]]$Architectures = @('x64', 'x86')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = Split-Path -Parent $PSScriptRoot
$backendRoot = Join-Path $projectRoot 'backend'
$outputRoot = Join-Path $backendRoot 'target\release\bundle\msix'
$manifestSource = Join-Path $projectRoot 'packaging\msix\AppxManifest.xml'
# Microsoft's Evergreen bootstrapper selects the runtime architecture on the user's PC.
$bootstrapperUrl = 'https://go.microsoft.com/fwlink/p/?LinkId=2124703'
$downloadRoot = Join-Path $backendRoot 'target\msix-downloads'
$bootstrapperPath = Join-Path $downloadRoot 'MicrosoftEdgeWebview2Setup.exe'
$targets = @{ x64 = 'x86_64-pc-windows-msvc'; x86 = 'i686-pc-windows-msvc' }

function Get-Sha256([string]$Path) {
    $algorithm = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($Path)
    try {
        return [BitConverter]::ToString($algorithm.ComputeHash($stream)).Replace('-', '')
    } finally {
        $stream.Dispose()
        $algorithm.Dispose()
    }
}

$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$makeAppx = Get-ChildItem -LiteralPath $sdkRoot -Directory |
    Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
    Sort-Object { [version]$_.Name } -Descending |
    ForEach-Object { Join-Path $_.FullName 'x64\makeappx.exe' } |
    Where-Object { Test-Path -LiteralPath $_ } |
    Select-Object -First 1
if (!$makeAppx) { throw 'Install the Windows SDK with MakeAppx before building MSIX.' }
$signTool = Join-Path (Split-Path -Parent $makeAppx) 'signtool.exe'
if (!(Test-Path -LiteralPath $signTool)) { throw 'Install the Windows SDK with SignTool.' }

[xml]$manifest = Get-Content -LiteralPath $manifestSource -Raw
$packageJson = Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json
$expectedVersion = "$($packageJson.version).0"
if ($manifest.Package.Identity.Version -ne $expectedVersion) {
    throw "Manifest version must match package.json: $expectedVersion"
}

New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
New-Item -ItemType Directory -Path $downloadRoot -Force | Out-Null
if (!(Test-Path -LiteralPath $bootstrapperPath)) {
    Write-Host 'Downloading the Microsoft WebView2 Evergreen bootstrapper...'
    $ProgressPreference = 'SilentlyContinue'
    $downloadPath = Join-Path $downloadRoot ([guid]::NewGuid().ToString('N') + '.exe')
    Invoke-WebRequest -UseBasicParsing -Uri $bootstrapperUrl -OutFile $downloadPath -TimeoutSec 120
    Move-Item -LiteralPath $downloadPath -Destination $bootstrapperPath
}
$verification = & $signTool verify /pa /v $bootstrapperPath 2>&1
if ($LASTEXITCODE -ne 0 -or ($verification -join "`n") -notmatch 'Issued to: Microsoft Corporation') {
    throw 'The WebView2 bootstrapper must have a valid Microsoft Corporation signature.'
}
$bundleStage = Join-Path $backendRoot ("target\msix-bundles\" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $bundleStage -Force | Out-Null
foreach ($architecture in $Architectures) {
$target = $targets[$architecture]
$releaseRoot = Join-Path $backendRoot "target\$target\release"
$executable = Join-Path $releaseRoot 'usagepin.exe'
$evergreenStamp = Join-Path $releaseRoot 'usagepin-evergreen.sha256'

Push-Location $projectRoot
try {
    if (!$SkipBuild) {
        # Keep the C runtime inside the executable instead of requiring VC++ Redistributable.
        $savedRustFlags = [Environment]::GetEnvironmentVariable('RUSTFLAGS', 'Process')
        try {
            $env:RUSTFLAGS = "$savedRustFlags -C target-feature=+crt-static".Trim()
            & npm.cmd run tauri -- build --no-bundle --ci --target $target --config backend/tauri.msix.conf.json -- --locked
            if ($LASTEXITCODE -ne 0) { throw "Tauri release build failed: $LASTEXITCODE" }
            Get-Sha256 $executable | Set-Content -LiteralPath $evergreenStamp -Encoding ASCII
        } finally {
            [Environment]::SetEnvironmentVariable('RUSTFLAGS', $savedRustFlags, 'Process')
        }
    }
    if (!(Test-Path -LiteralPath $executable)) { throw 'Release executable not found.' }
    if (!(Test-Path -LiteralPath $evergreenStamp) -or
        (Get-Content -LiteralPath $evergreenStamp -Raw).Trim() -ne (Get-Sha256 $executable)) {
        throw 'Run a full Store build before using -SkipBuild; this executable is not a verified Evergreen build.'
    }

    # Use a fresh staging directory so a rebuild cannot include stale package files.
    $stage = Join-Path $backendRoot ("target\msix-staging\" + [guid]::NewGuid().ToString('N'))
    $assets = Join-Path $stage 'Assets'
    New-Item -ItemType Directory -Path $assets -Force | Out-Null
    $manifest.Package.Identity.ProcessorArchitecture = $architecture
    $manifest.Save((Join-Path $stage 'AppxManifest.xml'))
    Copy-Item -LiteralPath $executable -Destination $stage
    Copy-Item -LiteralPath $bootstrapperPath -Destination $stage
    foreach ($icon in @('StoreLogo.png', 'Square150x150Logo.png', 'Square44x44Logo.png')) {
        Copy-Item -LiteralPath (Join-Path $backendRoot "icons\$icon") -Destination $assets
    }
    $packagePath = Join-Path $outputRoot "UsagePin_$expectedVersion`_$architecture.msix"
    $logPath = Join-Path $outputRoot "makeappx-$architecture.log"
    # Semantic validation is enabled. Do not use /nv for Store builds.
    & $makeAppx pack /d $stage /p $packagePath /h SHA256 /o 2>&1 |
        Out-File -FilePath $logPath
    Get-Content -LiteralPath $logPath | Select-Object -Last 8 | Write-Host
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx validation/packaging failed: $LASTEXITCODE" }
    $hash = Get-Sha256 $packagePath
    "$hash  $([IO.Path]::GetFileName($packagePath))" |
        Set-Content -LiteralPath "$packagePath.sha256" -Encoding ASCII
    Write-Host "Store upload package: $packagePath"
    Write-Host "SHA256: $hash"
    Copy-Item -LiteralPath $packagePath -Destination $bundleStage
} finally {
    Pop-Location
}
}
$bundlePath = Join-Path $outputRoot "UsagePin_$expectedVersion`_$($Architectures -join '_').msixbundle"
$bundleLog = Join-Path $outputRoot 'makeappx-bundle.log'
& $makeAppx bundle /d $bundleStage /p $bundlePath /bv $expectedVersion /o 2>&1 |
    Out-File -FilePath $bundleLog
Get-Content -LiteralPath $bundleLog | Select-Object -Last 8 | Write-Host
if ($LASTEXITCODE -ne 0) { throw "MakeAppx bundle failed: $LASTEXITCODE" }
$bundleHash = Get-Sha256 $bundlePath
"$bundleHash  $([IO.Path]::GetFileName($bundlePath))" |
    Set-Content -LiteralPath "$bundlePath.sha256" -Encoding ASCII
Write-Host "Store upload bundle: $bundlePath"
Write-Host "SHA256: $bundleHash"
