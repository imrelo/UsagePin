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
$runtimeVersion = '155.0.4283.45'
# Fixed downloads from Microsoft's WebView2 download page. Refresh these for releases.
$runtimeUrls = @{
    x64 = 'https://msedge.sf.dl.delivery.mp.microsoft.com/filestreamingservice/files/01b0fbd9-6ea3-4e12-818a-a71f73f3855b/Microsoft.WebView2.FixedVersionRuntime.155.0.4283.45.x64.cab'
    x86 = 'https://msedge.sf.dl.delivery.mp.microsoft.com/filestreamingservice/files/bdb15e94-cebc-476e-a5d5-ed3cbb4b3d89/Microsoft.WebView2.FixedVersionRuntime.155.0.4283.45.x86.cab'
}
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
$bundleStage = Join-Path $backendRoot ("target\msix-bundles\" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $bundleStage -Force | Out-Null
foreach ($architecture in $Architectures) {
$target = $targets[$architecture]
$releaseRoot = Join-Path $backendRoot "target\$target\release"
$runtimeRoot = Join-Path $backendRoot "webview2-runtime\$architecture"
$runtimeUrl = $runtimeUrls[$architecture]
if (!(Test-Path -LiteralPath (Join-Path $runtimeRoot 'msedgewebview2.exe'))) {
    $downloadRoot = Join-Path $backendRoot 'target\msix-downloads'
    New-Item -ItemType Directory -Path $downloadRoot -Force | Out-Null
    $cabPath = Join-Path $downloadRoot "WebView2-$runtimeVersion-$architecture.cab"
    if (!(Test-Path -LiteralPath $cabPath)) {
        Write-Host "Downloading Microsoft WebView2 $runtimeVersion $architecture..."
        $ProgressPreference = 'SilentlyContinue'
        Invoke-WebRequest -UseBasicParsing -Uri $runtimeUrl -OutFile $cabPath -TimeoutSec 600
    }
    $verification = & $signTool verify /pa /v $cabPath 2>&1
    if ($LASTEXITCODE -ne 0 -or ($verification -join "`n") -notmatch 'Issued to: Microsoft Corporation') {
        throw 'The downloaded WebView2 archive must have a valid Microsoft signature.'
    }
    $extractRoot = Join-Path $downloadRoot ([guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $extractRoot | Out-Null
    & expand.exe $cabPath '-F:*' $extractRoot | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "WebView2 extraction failed: $LASTEXITCODE" }
    $runtimeExe = Get-ChildItem -LiteralPath $extractRoot -Filter 'msedgewebview2.exe' -Recurse |
        Select-Object -First 1
    if (!$runtimeExe) { throw 'The WebView2 archive did not contain msedgewebview2.exe.' }
    New-Item -ItemType Directory -Path $runtimeRoot -Force | Out-Null
    Get-ChildItem -LiteralPath $runtimeExe.DirectoryName -Force |
        Copy-Item -Destination $runtimeRoot -Recurse -Force
}
$actualRuntime = (Get-Item -LiteralPath (Join-Path $runtimeRoot 'msedgewebview2.exe')).VersionInfo.FileVersion
if ($actualRuntime -ne $runtimeVersion) {
    throw "Expected WebView2 $runtimeVersion, found $actualRuntime. Refresh the runtime cache."
}

Push-Location $projectRoot
try {
    if (!$SkipBuild) {
        $buildConfig = Get-Content -LiteralPath (Join-Path $backendRoot 'tauri.msix.conf.json') -Raw | ConvertFrom-Json
        $buildConfig.bundle.windows.webviewInstallMode.path = "webview2-runtime/$architecture"
        $buildConfigPath = Join-Path $backendRoot "target\msix-config-$architecture.json"
        [IO.File]::WriteAllText($buildConfigPath, ($buildConfig | ConvertTo-Json -Depth 20), (New-Object Text.UTF8Encoding($false)))
        # Keep the C runtime inside the executable instead of requiring VC++ Redistributable.
        $savedRustFlags = [Environment]::GetEnvironmentVariable('RUSTFLAGS', 'Process')
        try {
            $env:RUSTFLAGS = "$savedRustFlags -C target-feature=+crt-static".Trim()
            & npm.cmd run tauri -- build --no-bundle --ci --target $target --config $buildConfigPath -- --locked
            if ($LASTEXITCODE -ne 0) { throw "Tauri release build failed: $LASTEXITCODE" }
        } finally {
            [Environment]::SetEnvironmentVariable('RUSTFLAGS', $savedRustFlags, 'Process')
        }
    }
    $executable = Join-Path $releaseRoot 'usagepin.exe'
    if (!(Test-Path -LiteralPath $executable)) { throw 'Release executable not found.' }
    $builtRuntime = Join-Path $releaseRoot "webview2-runtime\$architecture"
    if (!(Test-Path -LiteralPath (Join-Path $builtRuntime 'msedgewebview2.exe'))) {
        throw 'The Tauri release build must include the fixed WebView2 runtime.'
    }

    # Use a fresh staging directory so a rebuild cannot include stale package files.
    $stage = Join-Path $backendRoot ("target\msix-staging\" + [guid]::NewGuid().ToString('N'))
    $assets = Join-Path $stage 'Assets'
    New-Item -ItemType Directory -Path $assets -Force | Out-Null
    $manifest.Package.Identity.ProcessorArchitecture = $architecture
    $manifest.Save((Join-Path $stage 'AppxManifest.xml'))
    Copy-Item -LiteralPath $executable -Destination $stage
    $stageRuntime = Join-Path $stage 'webview2-runtime'
    New-Item -ItemType Directory -Path $stageRuntime | Out-Null
    Copy-Item -LiteralPath $builtRuntime -Destination (Join-Path $stageRuntime $architecture) -Recurse
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
