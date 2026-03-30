@echo off
where cargo >> "%~dp0cargo_found.txt" 2>&1
echo USERPROFILE=%USERPROFILE% >> "%~dp0cargo_found.txt"
dir "%USERPROFILE%\.cargo\bin" >> "%~dp0cargo_found.txt" 2>&1
