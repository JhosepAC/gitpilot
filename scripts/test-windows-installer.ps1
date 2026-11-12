param([Parameter(Mandatory = $true)][string]$Installer)

$ErrorActionPreference = 'Stop'
$env:PATH = (Join-Path $env:ProgramFiles 'Git\usr\bin') + ';' + $env:PATH
if ($env:GITHUB_ACTIONS -ne 'true' -or -not $env:RUNNER_TEMP) {
    throw 'Run installer validation only on a disposable GitHub Actions Windows runner.'
}
$fixture = Join-Path $env:RUNNER_TEMP ('gitpilot-installer-' + [guid]::NewGuid())
$installDir = Join-Path $fixture 'installed'
$dataDir = Join-Path $env:LOCALAPPDATA 'dev.gitpilot.desktop'
if (Test-Path -LiteralPath $dataDir) { throw 'Refusing to use existing application data.' }
New-Item -ItemType Directory -Path $fixture | Out-Null
$Installer = (Resolve-Path -LiteralPath $Installer).Path
$gitConfig = Join-Path $fixture 'gitconfig'
Set-Content -LiteralPath $gitConfig -Value "[user]`nname = Installer Fixture`nemail = fixture@example.test`n" -Encoding utf8
$gitHash = (Get-FileHash -LiteralPath $gitConfig).Hash

function Install-Package([string]$Path) {
    $process = Start-Process -FilePath $Path -ArgumentList "/S /D=$installDir" -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Installer failed: $($process.ExitCode)" }
    foreach ($file in @('gitpilot-desktop.exe', 'LICENSE', 'NOTICE')) {
        if (-not (Test-Path -LiteralPath (Join-Path $installDir $file))) { throw "Missing installed file: $file" }
    }
}

function Open-InstalledApp {
    $process = Start-Process -FilePath (Join-Path $installDir 'gitpilot-desktop.exe') -WindowStyle Hidden -PassThru -Environment @{
        GIT_CONFIG_GLOBAL = $gitConfig
        GIT_CONFIG_NOSYSTEM = '1'
    }
    try {
        $ready = $false
        for ($attempt = 0; $attempt -lt 60; $attempt++) {
            Start-Sleep -Milliseconds 500
            $process.Refresh()
            if ($process.HasExited) { throw "Installed app exited: $($process.ExitCode)" }
            if ($process.MainWindowHandle -ne 0 -and (Test-Path -LiteralPath (Join-Path $dataDir 'gitpilot.sqlite3'))) {
                $ready = $true
                break
            }
        }
        if (-not $ready) { throw 'Installed app did not open its window/database.' }
        Start-Sleep -Seconds 2
    } finally {
        if (-not $process.HasExited) {
            # Let WebView2 and SQLite release their handles before the next installation.
            $closed = $process.CloseMainWindow()
            if (-not $closed -or -not $process.WaitForExit(10000)) {
                $process.Kill($true)
                $process.WaitForExit()
            }
        }
    }
}

Install-Package $Installer
Open-InstalledApp
if (-not (Test-Path -LiteralPath (Join-Path $installDir 'THIRD_PARTY_NOTICES.txt'))) { throw 'Missing third-party notices.' }
# Only move the fresh data created by this script, within the explicit application-data parent.
$cleanBackup = Join-Path $env:LOCALAPPDATA ('dev.gitpilot.desktop.clean-' + [guid]::NewGuid())
if ([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($dataDir)) -ne [IO.Path]::GetFullPath($env:LOCALAPPDATA) -or
    [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($cleanBackup)) -ne [IO.Path]::GetFullPath($env:LOCALAPPDATA)) {
    throw 'Unexpected application-data paths.'
}
for ($attempt = 0; $attempt -lt 40; $attempt++) {
    try {
        Rename-Item -LiteralPath $dataDir -NewName ([IO.Path]::GetFileName($cleanBackup)) -ErrorAction Stop
        break
    } catch [System.IO.IOException] {
        if ($attempt -eq 39) { throw }
        Start-Sleep -Milliseconds 500
    }
}
if (Test-Path -LiteralPath $dataDir) { throw 'Clean-install data still in use after shutdown.' }
$cleanUninstaller = Start-Process -FilePath (Join-Path $installDir 'uninstall.exe') -ArgumentList '/S' -WindowStyle Hidden -PassThru -Wait
if ($cleanUninstaller.ExitCode -ne 0) { throw 'Clean-install uninstallation failed.' }
for ($attempt = 0; $attempt -lt 20 -and (Test-Path -LiteralPath (Join-Path $installDir 'gitpilot-desktop.exe')); $attempt++) { Start-Sleep -Milliseconds 500 }
if (Test-Path -LiteralPath (Join-Path $installDir 'gitpilot-desktop.exe')) { throw 'Clean-install executable remained after uninstall.' }

$previous = Join-Path $fixture 'previous'
New-Item -ItemType Directory -Path $previous | Out-Null
gh release download v1.0.0 --repo $env:GITHUB_REPOSITORY --dir $previous --pattern 'GitPilot_1.0.0_x64-setup.exe' --pattern 'SHA256SUMS.txt*' --pattern 'GitPilot-release-key.asc'
if ($LASTEXITCODE -ne 0) { throw 'Could not download the previous official release.' }
$gpgHome = Join-Path $fixture 'gnupg'
New-Item -ItemType Directory -Path $gpgHome | Out-Null
$key = Join-Path $previous 'GitPilot-release-key.asc'
$fingerprints = & gpg --homedir $gpgHome --batch --with-colons --show-keys $key
if ($LASTEXITCODE -ne 0 -or -not ($fingerprints -match '^fpr:::::::::47B10C26AF8BC9615DE39C5813EF6F27474B5E0F:')) { throw 'Unexpected distribution key.' }
& gpg --homedir $gpgHome --batch --import $key
if ($LASTEXITCODE -ne 0) { throw 'Could not import the public distribution key.' }
& gpg --homedir $gpgHome --batch --verify (Join-Path $previous 'SHA256SUMS.txt.asc') (Join-Path $previous 'SHA256SUMS.txt')
if ($LASTEXITCODE -ne 0) { throw 'Invalid signature for the previous release.' }
$oldInstaller = Join-Path $previous 'GitPilot_1.0.0_x64-setup.exe'
$line = Get-Content -LiteralPath (Join-Path $previous 'SHA256SUMS.txt') | Where-Object { $_ -match '\s+\*?GitPilot_1\.0\.0_x64-setup\.exe$' }
if (@($line).Count -ne 1 -or (Get-FileHash -Algorithm SHA256 -LiteralPath $oldInstaller).Hash -ne ($line -split '\s+')[0]) { throw 'Previous installer checksum mismatch.' }
Install-Package $oldInstaller
python scripts/create-release-fixture.py --data-dir $dataDir --workspace (Join-Path $fixture 'workspace') --through 202611070004
if ($LASTEXITCODE -ne 0) { throw 'Could not prepare the 1.0.0 migration fixture.' }
Open-InstalledApp
$repoConfig = Join-Path $fixture 'workspace/personal/Proyectos/Equipo/Repositorio de prueba/.git/config'
$repoHash = (Get-FileHash -LiteralPath $repoConfig).Hash
Install-Package $Installer
Open-InstalledApp
Open-InstalledApp
$env:GITPILOT_QA_DATABASE = Join-Path $dataDir 'gitpilot.sqlite3'
@'
import os, sqlite3
with sqlite3.connect(os.environ['GITPILOT_QA_DATABASE']) as connection:
    assert connection.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
    assert connection.execute('SELECT count(*) FROM identities').fetchone()[0] == 3
    assert connection.execute('SELECT count(*) FROM identity_rules').fetchone()[0] == 4
    assert connection.execute('SELECT count(*) FROM directories').fetchone()[0] == 3
    assert connection.execute('SELECT max(version) FROM _sqlx_migrations WHERE success = 1').fetchone()[0] == 202611120001
    assert connection.execute('SELECT sum(signing_required) FROM identities').fetchone()[0] == 0
print('Installed upgrade: migration, integrity, profiles, directories and rules verified')
'@ | python -
if ($LASTEXITCODE -ne 0) { throw 'Installed upgrade data validation failed.' }
if ((Get-FileHash -LiteralPath $gitConfig).Hash -ne $gitHash -or (Get-FileHash -LiteralPath $repoConfig).Hash -ne $repoHash) { throw 'Installation changed Git configuration.' }
Install-Package $Installer
$uninstaller = Join-Path $installDir 'uninstall.exe'
if (-not (Test-Path -LiteralPath $uninstaller)) { throw 'Uninstaller missing.' }
$process = Start-Process -FilePath $uninstaller -ArgumentList '/S' -WindowStyle Hidden -PassThru -Wait
if ($process.ExitCode -ne 0) { throw 'Uninstallation failed.' }
for ($attempt = 0; $attempt -lt 20 -and (Test-Path -LiteralPath (Join-Path $installDir 'gitpilot-desktop.exe')); $attempt++) { Start-Sleep -Milliseconds 500 }
if (Test-Path -LiteralPath (Join-Path $installDir 'gitpilot-desktop.exe')) { throw 'Installed executable remained after uninstall.' }
Write-Output 'Windows installer: clean install, upgrade from 1.0.0, reopen, reinstall and uninstall passed.'
