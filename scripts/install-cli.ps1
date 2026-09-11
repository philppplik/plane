<#
.SYNOPSIS
    Installiert die Plane-Kommandozeile aus der neuesten Veröffentlichung.

.DESCRIPTION
    Lädt `plane-cli.exe` passend zur Architektur des Rechners herunter,
    prüft die Prüfsumme gegen die veröffentlichte SHA256SUMS.txt, legt die
    Datei in %LOCALAPPDATA%\Programs\Plane ab und ergänzt den Benutzer-PATH.

    Es werden keine Administratorrechte gebraucht: geschrieben wird nur ins
    eigene Profil, geändert nur der Benutzer-PATH — nie der des Systems.

    Die Prüfsummenkontrolle ist kein Beiwerk. Plane ist ein Werkzeug mit
    Löschrechten; ein manipuliertes Binary wäre ein Totalschaden. Schlägt die
    Prüfung fehl, bricht das Skript ab.

.PARAMETER Version
    Version wie `v0.1.0`. Ohne Angabe wird die neueste Veröffentlichung
    verwendet.

.PARAMETER Ziel
    Installationsordner. Standard: %LOCALAPPDATA%\Programs\Plane

.EXAMPLE
    irm https://raw.githubusercontent.com/philppplik/plane/main/scripts/install-cli.ps1 | iex

.EXAMPLE
    .\install-cli.ps1 -Version v0.1.0
#>

[CmdletBinding()]
param(
    [string]$Version = 'latest',
    [string]$Ziel = (Join-Path $env:LOCALAPPDATA 'Programs\Plane')
)

$ErrorActionPreference = 'Stop'
$REPO = 'philppplik/plane'

function Schreibe($text, $farbe = 'Gray') { Write-Host "  $text" -ForegroundColor $farbe }

Write-Host ''
Write-Host '  Plane CLI' -ForegroundColor Green
Write-Host '  =========' -ForegroundColor Green
Write-Host ''

# --- 1. Architektur bestimmen ---------------------------------------------
# PROCESSOR_ARCHITECTURE meldet in einem emulierten x86-Prozess das Falsche;
# PROCESSOR_ARCHITEW6432 ist dann die Wahrheit.
$arch = $env:PROCESSOR_ARCHITEW6432
if (-not $arch) { $arch = $env:PROCESSOR_ARCHITECTURE }

switch ($arch) {
    'ARM64' { $datei = 'plane-cli-arm64.exe' }
    'AMD64' { $datei = 'plane-cli-x64.exe' }
    default {
        throw "Nicht unterstützte Architektur: $arch. Plane gibt es für x64 und ARM64."
    }
}
Schreibe "Architektur: $arch  ->  $datei"

# --- 2. Veröffentlichung ermitteln ----------------------------------------
if ($Version -eq 'latest') {
    $api = "https://api.github.com/repos/$REPO/releases/latest"
} else {
    $api = "https://api.github.com/repos/$REPO/releases/tags/$Version"
}

try {
    $release = Invoke-RestMethod -Uri $api -Headers @{ 'User-Agent' = 'plane-installer' }
} catch {
    throw "Veröffentlichung nicht abrufbar ($api). Gibt es schon ein Release? $_"
}

$tag = $release.tag_name
Schreibe "Version: $tag"

$assetBinary = $release.assets | Where-Object { $_.name -eq $datei }
$assetSummen = $release.assets | Where-Object { $_.name -eq 'SHA256SUMS.txt' }

if (-not $assetBinary) { throw "In $tag fehlt die Datei $datei." }

# --- 3. Herunterladen ------------------------------------------------------
$temp = Join-Path ([System.IO.Path]::GetTempPath()) "plane-cli-$([guid]::NewGuid()).exe"
Schreibe 'Lade herunter …'
Invoke-WebRequest -Uri $assetBinary.browser_download_url -OutFile $temp -UseBasicParsing

# --- 4. Prüfsumme kontrollieren -------------------------------------------
if ($assetSummen) {
    $summen = (Invoke-WebRequest -Uri $assetSummen.browser_download_url -UseBasicParsing).Content
    $erwartet = ($summen -split "`n" |
        Where-Object { $_ -match [regex]::Escape($datei) } |
        Select-Object -First 1) -split '\s+' | Select-Object -First 1

    if ($erwartet) {
        $tatsaechlich = (Get-FileHash -Path $temp -Algorithm SHA256).Hash.ToLower()
        if ($tatsaechlich -ne $erwartet.ToLower()) {
            Remove-Item $temp -Force -ErrorAction SilentlyContinue
            throw "Prüfsumme stimmt nicht!`n  erwartet:  $erwartet`n  bekommen:  $tatsaechlich`nDie Installation wurde abgebrochen."
        }
        Schreibe 'Prüfsumme stimmt.' 'Green'
    } else {
        Schreibe "WARNUNG: Für $datei steht keine Prüfsumme in SHA256SUMS.txt." 'Yellow'
    }
} else {
    Schreibe 'WARNUNG: Die Veröffentlichung enthält keine SHA256SUMS.txt.' 'Yellow'
}

# --- 5. Installieren -------------------------------------------------------
if (-not (Test-Path $Ziel)) { New-Item -ItemType Directory -Path $Ziel -Force | Out-Null }
$installiert = Join-Path $Ziel 'plane-cli.exe'

try {
    Move-Item -Path $temp -Destination $installiert -Force
} catch {
    Remove-Item $temp -Force -ErrorAction SilentlyContinue
    throw "Kopieren nach $Ziel fehlgeschlagen. Läuft plane-cli gerade? $_"
}
Schreibe "Installiert: $installiert" 'Green'

# --- 6. PATH ergänzen ------------------------------------------------------
# Bewusst über die Registry-API und nicht über `setx`: setx schneidet den PATH
# bei 1024 Zeichen ab und löst Variablen auf.
$pfad = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($null -eq $pfad) { $pfad = '' }
$teile = $pfad -split ';' | Where-Object { $_ -ne '' }

if ($teile -contains $Ziel) {
    Schreibe 'PATH enthält den Ordner bereits.'
} else {
    [Environment]::SetEnvironmentVariable('Path', ((@($teile) + $Ziel) -join ';'), 'User')
    Schreibe 'PATH ergänzt.' 'Green'
}

# Damit der Aufruf schon in dieser Sitzung klappt.
if (($env:Path -split ';') -notcontains $Ziel) { $env:Path = "$env:Path;$Ziel" }

Write-Host ''
Schreibe 'Fertig. In einem neuen Terminal:' 'Green'
Write-Host ''
Write-Host '      plane-cli            Textoberfläche'
Write-Host '      plane-cli scan       analysieren, ohne etwas zu verändern'
Write-Host '      plane-cli --help     alle Befehle'
Write-Host ''
