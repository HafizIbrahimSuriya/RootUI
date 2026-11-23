@echo off
setlocal
title RootUI Launcher
cd /d "%~dp0"

where cargo >nul 2>nul
if errorlevel 1 (
    echo.
    echo  [RootUI] Rust is not installed ^(cargo not found^).
    echo  Install it from https://rustup.rs  ^(choose the default MSVC toolchain^),
    echo  then close this window and run run-rootui.bat again.
    echo.
    pause
    exit /b 1
)

echo.
echo  [RootUI] Building and starting... ^(the first run downloads and compiles
echo  dependencies, this can take a few minutes; later runs start instantly^)
echo.

cargo run --release -- %*
set EXITCODE=%ERRORLEVEL%

if not "%EXITCODE%"=="0" (
    echo.
    echo  [RootUI] Exited with code %EXITCODE%. See the messages above.
    pause
)
endlocal
