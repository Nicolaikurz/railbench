@echo off
setlocal enabledelayedexpansion
cd /d "%~dp0\.."

set "ARCH=%~1"
if "%ARCH%"=="" set "ARCH=x64"

set "PKG_PATCH=1"
for /f "tokens=3 delims=." %%p in ('powershell -NoProfile -Command "(Select-String -Path 'rust\Cargo.toml' -Pattern '^\s*version\s*=\s*\"([^\"]+)\"').Matches.Groups[1].Value"') do set "PKG_PATCH=%%p"
for /f %%v in ('powershell -NoProfile -Command "[DateTime]::UtcNow.ToString('yy.MM') + '.' + '%PKG_PATCH%'"') do set "APP_VERSION=%%v"
if not "%RAILBENCH_VERSION%"=="" set "APP_VERSION=%RAILBENCH_VERSION%"

echo === Building Native Rust Windows Portable (ZIP) ===
echo Application        : railbench v%APP_VERSION%
echo Target Architecture: %ARCH%

if not exist "release\old" mkdir "release\old"
for %%f in (release\*-win-*.zip release\*-portable-*.exe) do (
    if exist "%%f" (
        echo * Archiving old binary -^> release\old\%%~nxf
        move /y "%%f" "release\old\%%~nxf" >nul
    )
)

if /i "%ARCH%"=="x64" (
    call :package_target x64 x86_64-pc-windows-msvc
) else if /i "%ARCH%"=="arm64" (
    call :package_target arm64 aarch64-pc-windows-msvc
) else if /i "%ARCH%"=="all" (
    call :package_target x64 x86_64-pc-windows-msvc
    if !errorlevel! neq 0 exit /b !errorlevel!
    call :package_target arm64 aarch64-pc-windows-msvc
) else (
    echo [ERROR] Unknown architecture: %ARCH% ^(choose x64, arm64, or all^)
    exit /b 1
)

if %errorlevel% neq 0 exit /b %errorlevel%

echo.
echo ===========================================
echo Windows Portable build complete!
echo ZIP Archives in: release\
echo ===========================================
pause
exit /b 0

:package_target
set "TARGET_ARCH=%~1"
set "RUST_TARGET=%~2"
set "FOLDER_NAME=railbench-%APP_VERSION%-win-%TARGET_ARCH%"
set "STAGE_ROOT=release\win-stage-%TARGET_ARCH%"
set "STAGE_DIR=%STAGE_ROOT%\%FOLDER_NAME%"
set "ZIP_OUTPUT=release\%FOLDER_NAME%.zip"
set "EXE_SRC=rust\target\%RUST_TARGET%\release\railbench-app.exe"

echo.
echo * Compiling Rust release binary (%RUST_TARGET%)...
call cargo build --release --manifest-path rust\Cargo.toml -p railbench-app --target %RUST_TARGET%
if %errorlevel% neq 0 (
    echo [ERROR] cargo build failed for %RUST_TARGET%!
    exit /b %errorlevel%
)

if not exist "%EXE_SRC%" (
    echo [ERROR] Expected binary not found: %EXE_SRC%
    exit /b 1
)

if exist "%STAGE_ROOT%" rmdir /s /q "%STAGE_ROOT%"
if exist "%ZIP_OUTPUT%" del /f /q "%ZIP_OUTPUT%"
mkdir "%STAGE_DIR%"
copy /y "%EXE_SRC%" "%STAGE_DIR%\railbench.exe" >nul
if exist "LICENSE" copy /y "LICENSE" "%STAGE_DIR%\LICENSE" >nul

echo * Creating portable ZIP archive: %ZIP_OUTPUT%
powershell -NoProfile -Command "Compress-Archive -Path '%STAGE_DIR%' -DestinationPath '%ZIP_OUTPUT%' -Force"
if !errorlevel! neq 0 (
    echo [ERROR] Failed to create %ZIP_OUTPUT%!
    exit /b !errorlevel!
)
echo [OK] Created %ZIP_OUTPUT%
rmdir /s /q "%STAGE_ROOT%"
exit /b 0
