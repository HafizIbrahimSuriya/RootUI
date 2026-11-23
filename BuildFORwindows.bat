@echo off
setlocal EnableExtensions EnableDelayedExpansion
title RootUI - Build for Windows (64-bit + 32-bit)
cd /d "%~dp0"

set "VERSION=1.0V"

echo.
echo  ==========================================
echo    RootUI %VERSION% - Windows build (x64 + x86)
echo  ==========================================
echo.

where cargo >nul 2>nul
if errorlevel 1 (
    echo  [RootUI] Rust is not installed ^(cargo not found^).
    echo  Install it from https://rustup.rs ^(default MSVC toolchain^), then run this file again.
    pause
    exit /b 1
)

if exist dist rmdir /s /q dist
mkdir dist

set "OK64=0"
set "OK86=0"

rem ------------------------------------------------------------------
rem  1) EXE  (64-bit and 32-bit)
rem ------------------------------------------------------------------
echo  [RootUI] Adding the Rust targets ^(only needed the first time^)...
rustup target add x86_64-pc-windows-msvc i686-pc-windows-msvc

call :winbuild x86_64-pc-windows-msvc windows-x64
if not errorlevel 1 set "OK64=1"

call :winbuild i686-pc-windows-msvc windows-x86
if not errorlevel 1 set "OK86=1"

if "%OK64%"=="0" if "%OK86%"=="0" goto :fail

rem ------------------------------------------------------------------
rem  2) SETUP FILES  (Inno Setup 6) - one script per architecture
rem ------------------------------------------------------------------
set "ISCC="
for %%P in ("%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe" "%ProgramFiles%\Inno Setup 6\ISCC.exe" "%LocalAppData%\Programs\Inno Setup 6\ISCC.exe") do if exist %%P set "ISCC=%%~P"
if not defined ISCC (
    for /f "delims=" %%P in ('where ISCC.exe 2^>nul') do set "ISCC=%%P"
)

if defined ISCC (
    if "%OK64%"=="1" call :setup x64
    if "%OK86%"=="1" call :setup x86
) else (
    echo.
    echo  [RootUI] Inno Setup not found - the setup files were skipped ^(the exe files are ready^).
    echo           Install Inno Setup 6 from https://jrsoftware.org/isdl.php and run this file again.
)

echo.
echo  ==========================================
echo    RootUI %VERSION% - finished
echo  ==========================================
echo.
if "%OK64%"=="1" echo   64-bit exe : dist\windows-x64\RootUI.exe
if "%OK64%"=="0" echo   64-bit exe : FAILED
if "%OK86%"=="1" echo   32-bit exe : dist\windows-x86\RootUI.exe
if "%OK86%"=="0" echo   32-bit exe : FAILED - install the x86 MSVC build tools in Visual Studio Installer
echo.
if exist "dist\installers" (
    echo   Setup files:
    dir /b "dist\installers"
    echo.
    explorer "%~dp0dist\installers"
)
pause
endlocal
exit /b 0


rem ==================================================================
:winbuild
echo.
echo  [RootUI] Building %2 ^(%1^)...
cargo build --release --target %1
if errorlevel 1 (
    echo  [RootUI] Build FAILED for %2.
    exit /b 1
)
mkdir "dist\%2" >nul 2>nul
copy /y "target\%1\release\rootui.exe" "dist\%2\RootUI.exe" >nul
xcopy /e /i /y /q "ui" "dist\%2\ui" >nul
exit /b 0


rem ==================================================================
:setup
echo.
echo  [RootUI] Making the %1 setup file...
"%ISCC%" /Q "packaging\RootUI-%1.iss"
if errorlevel 1 (
    echo  [RootUI] Inno Setup FAILED for %1 - see the message above.
) else (
    echo  [RootUI] Ready: dist\installers\RootUI-%VERSION%-Setup-%1.exe
)
exit /b 0


rem ==================================================================
:fail
echo.
echo  [RootUI] Build failed. See the messages above.
pause
exit /b 1
