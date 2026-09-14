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

$runningReleaseLaunchers = @(Get-Process -ErrorAction SilentlyContinue | ForEach-Object {
    try {
        $processPath = [System.IO.Path]::GetFullPath($_.Path)
        if ($processPath -eq $releaseExecutable) {
            $_
        }
    }
    catch {
        # Některé systémové procesy neumožňují přečíst Path; nejsou naším cílem.
    }
})

foreach ($process in $runningReleaseLaunchers) {
    Write-Host "Ukončuji starou instanci PID $($process.Id): $($process.Path)"
    Stop-Process -Id $process.Id
}
if ($runningReleaseLaunchers.Count -gt 0) {
    Wait-Process -Id @($runningReleaseLaunchers.Id) -Timeout 10 -ErrorAction SilentlyContinue
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

$runningInstalledLaunchers = @(Get-Process -ErrorAction SilentlyContinue | ForEach-Object {
    try {
        $processPath = [System.IO.Path]::GetFullPath($_.Path)
        if ($processPath -eq $installedExecutable) {
            $_
        }
    }
    catch {
        # Některé systémové procesy neumožňují přečíst Path; nejsou naším cílem.
    }
})
foreach ($process in $runningInstalledLaunchers) {
    Write-Host "Ukončuji starou nainstalovanou instanci PID $($process.Id): $($process.Path)"
    Stop-Process -Id $process.Id
}
if ($runningInstalledLaunchers.Count -gt 0) {
    Wait-Process -Id @($runningInstalledLaunchers.Id) -Timeout 10 -ErrorAction SilentlyContinue
}

Write-Host 'Instaluji release a aktualizuji per-user registraci pro HTTP a HTTPS...'
$registration = $null
$retryDelays = @(250, 500, 1000)
for ($attempt = 0; $attempt -lt $retryDelays.Count; $attempt++) {
    $registration = Start-Process -FilePath $releaseExecutable -ArgumentList '--install-register' -Wait -PassThru -WindowStyle Hidden
    if ($registration.ExitCode -eq 0) {
        break
    }
    if ($attempt -lt ($retryDelays.Count - 1)) {
        Write-Warning "Instalace skončila s kódem $($registration.ExitCode), opakuji pokus..."
        Start-Sleep -Milliseconds $retryDelays[$attempt]
    }
}
if ($null -eq $registration -or $registration.ExitCode -ne 0) {
    throw "Instalace a registrace skončila s kódem $($registration.ExitCode)."
}

$deployed = Get-Item -LiteralPath $installedExecutable
Write-Host "Nasazeno: $($deployed.FullName) ($($deployed.Length) bajtů)"

if (-not $NoStart) {
    $started = Start-Process -FilePath $installedExecutable -PassThru -WindowStyle Hidden
    Write-Host "Spuštěna nová instance PID $($started.Id)."
}

