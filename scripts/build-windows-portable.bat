@echo off
echo === Building Windows Portable Directory ===
cd /d "%~dp0\.."

call npm run build:all
if %errorlevel% neq 0 (
    echo [ERROR] Build failed!
    exit /b %errorlevel%
)

echo.
echo • Packaging Windows Portable directory (win-unpacked)...
call npx electron-builder --win dir
if %errorlevel% neq 0 (
    echo [ERROR] electron-builder failed!
    exit /b %errorlevel%
)

echo.
echo • Creating portable ZIP archive...
powershell -NoProfile -Command "if (Test-Path release\win-unpacked) { Compress-Archive -Path release\win-unpacked\* -DestinationPath release\railbench-0.6.0-portable-x64.zip -Force }" 2>nul || true

echo.
echo ===========================================
echo Windows Portable build complete!
echo Portable folder: release\win-unpacked\
echo Executable:     release\win-unpacked\railbench.exe
echo ZIP Archive:    release\railbench-0.6.0-portable-x64.zip
echo ===========================================
pause
