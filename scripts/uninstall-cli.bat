@echo off
rem ===========================================================================
rem  Plane CLI entfernen
rem ---------------------------------------------------------------------------
rem  Loescht plane-cli.exe aus %LOCALAPPDATA%\Programs\Plane und nimmt den
rem  Ordner wieder aus dem Benutzer-PATH. Ruehrt nichts sonst an.
rem
rem  Die Einstellungen unter %APPDATA%\com.ppaul.plane bleiben erhalten - sie
rem  gehoeren zur grafischen Anwendung, nicht zur CLI.
rem ===========================================================================

chcp 65001 >nul
setlocal

set "ZIEL=%LOCALAPPDATA%\Programs\Plane"

echo.
echo   Plane CLI - Deinstallation
echo   =========================
echo.

if exist "%ZIEL%\plane-cli.exe" (
    del /Q "%ZIEL%\plane-cli.exe"
    if errorlevel 1 (
        echo   FEHLER: Loeschen fehlgeschlagen. Laeuft plane-cli gerade?
        exit /b 1
    )
    echo   Entfernt: %ZIEL%\plane-cli.exe
) else (
    echo   plane-cli.exe war nicht installiert.
)

rem Ordner nur entfernen, wenn er jetzt leer ist.
rd "%ZIEL%" 2>nul

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$ziel = $env:LOCALAPPDATA + '\Programs\Plane';" ^
  "$pfad = [Environment]::GetEnvironmentVariable('Path','User');" ^
  "if ($null -eq $pfad) { $pfad = '' };" ^
  "$teile = $pfad -split ';' | Where-Object { $_ -ne '' -and $_ -ne $ziel };" ^
  "[Environment]::SetEnvironmentVariable('Path', ($teile -join ';'), 'User');" ^
  "Write-Host '  PATH bereinigt.'"

echo.
echo   Fertig. Die Aenderung am PATH gilt in neuen Terminals.
echo.

endlocal
exit /b 0
