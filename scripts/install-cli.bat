@echo off
rem ===========================================================================
rem  Plane CLI installieren
rem ---------------------------------------------------------------------------
rem  Kopiert plane-cli.exe nach %LOCALAPPDATA%\Programs\Plane und nimmt den
rem  Ordner in den Benutzer-PATH auf. Danach laesst sich `plane-cli` aus jedem
rem  Terminal aufrufen.
rem
rem  Braucht KEINE Administratorrechte: es wird nur ins eigene Benutzerprofil
rem  geschrieben und nur der Benutzer-PATH geaendert, nie der des Systems.
rem ===========================================================================

rem UTF-8, damit Umlaute in der Ausgabe korrekt erscheinen.
chcp 65001 >nul
setlocal EnableDelayedExpansion

set "PROJEKT=%~dp0.."
set "QUELLE=%PROJEKT%\src-tauri\target\release\plane-cli.exe"
set "ZIEL=%LOCALAPPDATA%\Programs\Plane"

echo.
echo   Plane CLI - Installation
echo   ========================
echo.

rem --- 1. Binary vorhanden? --------------------------------------------------
if not exist "%QUELLE%" (
    echo   plane-cli.exe wurde nicht gefunden. Es wird jetzt gebaut.
    echo   Das dauert beim ersten Mal einige Minuten.
    echo.
    where cargo >nul 2>&1
    if errorlevel 1 (
        echo   FEHLER: cargo wurde nicht gefunden.
        echo   Bitte Rust installieren: https://rustup.rs/
        echo.
        exit /b 2
    )
    pushd "%PROJEKT%\src-tauri"
    cargo build --release --bin plane-cli
    set BAUFEHLER=!errorlevel!
    popd
    if not "!BAUFEHLER!"=="0" (
        echo.
        echo   FEHLER: Der Build ist fehlgeschlagen.
        exit /b 1
    )
)

if not exist "%QUELLE%" (
    echo   FEHLER: %QUELLE% fehlt auch nach dem Build.
    exit /b 1
)

rem --- 2. Kopieren -----------------------------------------------------------
if not exist "%ZIEL%" mkdir "%ZIEL%" 2>nul
copy /Y "%QUELLE%" "%ZIEL%\plane-cli.exe" >nul
if errorlevel 1 (
    echo   FEHLER: Kopieren nach %ZIEL% fehlgeschlagen.
    echo   Laeuft plane-cli gerade? Bitte schliessen und erneut versuchen.
    exit /b 1
)
echo   Installiert nach: %ZIEL%\plane-cli.exe

rem --- 3. PATH ergaenzen -----------------------------------------------------
rem  Bewusst ueber PowerShell und nicht ueber `setx`: setx schneidet den PATH
rem  bei 1024 Zeichen ab und loest Variablen auf - das hat schon manchem die
rem  Umgebung zerschossen.
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$ziel = $env:LOCALAPPDATA + '\Programs\Plane';" ^
  "$pfad = [Environment]::GetEnvironmentVariable('Path','User');" ^
  "if ($null -eq $pfad) { $pfad = '' };" ^
  "$teile = $pfad -split ';' | Where-Object { $_ -ne '' };" ^
  "if ($teile -contains $ziel) {" ^
  "  Write-Host '  PATH enthaelt den Ordner bereits.'" ^
  "} else {" ^
  "  $neu = (@($teile) + $ziel) -join ';';" ^
  "  [Environment]::SetEnvironmentVariable('Path', $neu, 'User');" ^
  "  Write-Host '  PATH ergaenzt.'" ^
  "}"

echo.
echo   Fertig. Neues Terminal oeffnen, dann:
echo.
echo       plane-cli            Textoberflaeche
echo       plane-cli scan       analysieren, ohne etwas zu veraendern
echo       plane-cli --help     alle Befehle
echo.
echo   Deinstallation: scripts\uninstall-cli.bat
echo.

endlocal
exit /b 0
