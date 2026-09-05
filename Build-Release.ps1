[CmdletBinding()]
param(
    [switch]$NoStart
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'

$projectRoot = [System.IO.Path]::GetFullPath($PSScriptRoot)
$localAppData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
if ([string]::IsNullOrWhiteSpace($localAppData)) {
    throw 'Windows neposkytl cestu LocalApplicationData.'
}

$installDirectory = [System.IO.Path]::GetFullPath(
    [System.IO.Path]::Combine($localAppData, 'BrowserLauncher')
)
$installedExecutable = [System.IO.Path]::GetFullPath(
    [System.IO.Path]::Combine($installDirectory, 'BrowserLauncher.exe')
)
$releaseExecutable = [System.IO.Path]::GetFullPath(
    [System.IO.Path]::Combine($projectRoot, 'target', 'release', 'browser-launcher.exe')
)

if ([System.IO.Path]::GetDirectoryName($installedExecutable) -ne $installDirectory) {
    throw "Neplatná cílová cesta: $installedExecutable"
}

$managedPaths = @($releaseExecutable, $installedExecutable)
$runningLaunchers = @(Get-Process -ErrorAction SilentlyContinue | ForEach-Object {
    try {
        $processPath = [System.IO.Path]::GetFullPath($_.Path)
        if ($managedPaths -contains $processPath) {
            $_
        }
    }
    catch {
        # Některé systémové procesy neumožňují přečíst Path; nejsou naším cílem.
    }
})

foreach ($process in $runningLaunchers) {
    Write-Host "Ukončuji starou instanci PID $($process.Id): $($process.Path)"
    Stop-Process -Id $process.Id
}
if ($runningLaunchers.Count -gt 0) {
    Wait-Process -Id @($runningLaunchers.Id) -Timeout 10 -ErrorAction SilentlyContinue
}

Push-Location $projectRoot
try {
    Write-Host 'Sestavuji release...'
    & cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release skončil s kódem $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $releaseExecutable -PathType Leaf)) {
    throw "Release EXE nebyl vytvořen: $releaseExecutable"
}

[System.IO.Directory]::CreateDirectory($installDirectory) | Out-Null
Copy-Item -LiteralPath $releaseExecutable -Destination $installedExecutable -Force

Write-Host 'Aktualizuji per-user registraci pro HTTP a HTTPS...'
& $installedExecutable --install-register
if ($LASTEXITCODE -ne 0) {
    throw "Registrace skončila s kódem $LASTEXITCODE."
}

$deployed = Get-Item -LiteralPath $installedExecutable
Write-Host "Nasazeno: $($deployed.FullName) ($($deployed.Length) bajtů)"

if (-not $NoStart) {
    $started = Start-Process -FilePath $installedExecutable -PassThru
    Write-Host "Spuštěna nová instance PID $($started.Id)."
}

