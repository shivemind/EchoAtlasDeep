@echo off
cd /d "%~dp0"
set CARGO_TARGET_DIR=C:\ProgramData\rmtide\target
"%USERPROFILE%\.cargo\bin\cargo.exe" run
echo.
echo Exit code: %ERRORLEVEL%
