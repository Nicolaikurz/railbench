@echo off
setlocal enabledelayedexpansion
cd /d "%~dp0\.."

set "ARCH=%~1"
if "%ARCH%"=="" set "ARCH=all"

for /f "delims=" %%i in ('node -p "require('./package.json').build?.productName || require('./package.json').name || 'railbench'"') do set "PRODUCT_NAME=%%i"
for /f "delims=" %%i in ('node -p "require('./package.json').version || '0.6.1'"') do set "APP_VERSION=%%i"

echo === Building Windows Portable (ZIP) ===
echo Application: %PRODUCT_NAME% v%APP_VERSION%
echo Target Architecture: %ARCH%

if not exist "release\old" mkdir "release\old"
for %%f in (release\*-win-*.zip release\*-portable-*.exe) do (
    if exist "%%f" (
        echo * Archiving old binary -^> release\old\%%~nxf
        move /y "%%f" "release\old\%%~nxf" >nul
    )
)

call npm run build:all
if %errorlevel% neq 0 (
    echo [ERROR] Build failed!
    exit /b %errorlevel%
)

if /i "%ARCH%"=="x64" (
    call :package_target x64
) else if /i "%ARCH%"=="arm64" (
    call :package_target arm64
) else if /i "%ARCH%"=="all" (
    call :package_target x64
    if !errorlevel! neq 0 exit /b !errorlevel!
    call :package_target arm64
) else (
    echo [ERROR] Unknown architecture: %ARCH% ^(choose x64, arm64, or all^)
    exit /b 1
)

if %errorlevel% neq 0 exit /b %errorlevel%

echo * Cleaning intermediate build artifacts...
if exist "release\win-unpacked" rmdir /s /q "release\win-unpacked"
if exist "release\win-arm64-unpacked" rmdir /s /q "release\win-arm64-unpacked"
if exist "release\builder-debug.yml" del /f /q "release\builder-debug.yml"
if exist "release\builder-effective-config.yaml" del /f /q "release\builder-effective-config.yaml"
for %%f in (release\latest*.yml release\*.blockmap) do if exist "%%f" del /f /q "%%f"

echo.
echo ===========================================
echo Windows Portable build complete!
echo ZIP Archives:
echo   x64:   release\%PRODUCT_NAME%-%APP_VERSION%-win-x64.zip
echo   arm64: release\%PRODUCT_NAME%-%APP_VERSION%-win-arm64.zip
echo ===========================================
pause
exit /b 0

:package_target
set "TARGET_ARCH=%~1"
set "UNPACKED_DIR=release\win-unpacked"
if /i "%TARGET_ARCH%"=="arm64" set "UNPACKED_DIR=release\win-arm64-unpacked"
set "FOLDER_NAME=%PRODUCT_NAME%-%APP_VERSION%-win-%TARGET_ARCH%"
set "STAGED_DIR=release\%FOLDER_NAME%"
set "ZIP_OUTPUT=release\%FOLDER_NAME%.zip"

echo.
echo * Packaging Windows Portable directory for %TARGET_ARCH%...
call npx electron-builder --win dir --%TARGET_ARCH%
if %errorlevel% neq 0 (
    echo [ERROR] electron-builder failed for %TARGET_ARCH%!
    exit /b %errorlevel%
)

if exist "%UNPACKED_DIR%" (
    echo * Staging folder with architecture badge: %STAGED_DIR%
    if exist "%STAGED_DIR%" rmdir /s /q "%STAGED_DIR%"
    if exist "%ZIP_OUTPUT%" del /f /q "%ZIP_OUTPUT%"
    move /y "%UNPACKED_DIR%" "%STAGED_DIR%" >nul

    echo * Creating portable ZIP archive: %ZIP_OUTPUT%
    powershell -NoProfile -Command "Compress-Archive -Path '%STAGED_DIR%' -DestinationPath '%ZIP_OUTPUT%' -Force"
    if !errorlevel! neq 0 (
        echo [ERROR] Failed to create %ZIP_OUTPUT%!
        exit /b !errorlevel!
    )
    echo [OK] Created %ZIP_OUTPUT%
    rmdir /s /q "%STAGED_DIR%"
) else (
    echo [WARN] Directory %UNPACKED_DIR% was not generated.
)
exit /b 0
