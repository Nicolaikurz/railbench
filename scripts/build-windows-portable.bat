@echo off
echo === Building Windows Portable Executable ===
cd /d "%~dp0\.."

call npm run build:all
if %errorlevel% neq 0 (
    echo [ERROR] Build failed!
    exit /b %errorlevel%
)

call npx electron-builder --win portable
if %errorlevel% neq 0 (
    echo [ERROR] electron-builder failed!
    exit /b %errorlevel%
)

echo.
echo ===========================================
echo Windows Portable build complete!
echo Executable located in release\
echo ===========================================
pause

