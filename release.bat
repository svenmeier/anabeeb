@echo off

cargo build --release
if errorlevel 1 (
    echo Build failed.
    exit /b 1
)

set EXE=
for %%f in (target\release\*.exe) do set EXE=%%f
if "%EXE%"=="" (
    echo No .exe found in target\release.
    exit /b 1
)
set EXAMPLES="examples"
set SCHEMAS="schemas"
set JSON="*.json"
set DLLS="target\fluidsynth\bin\*.dll"
set TEMP="target\anabeeb-windows"
set ZIP="target\anabeeb-windows.zip"

if exist %TEMP% rmdir /s /q %TEMP%
mkdir %TEMP%
copy "%EXE%" "%TEMP%\" >nul
copy "%JSON%" "%TEMP%\" >nul
xcopy /E /I /Y "%EXAMPLES%" "%TEMP%\%EXAMPLES%\" >nul
xcopy /E /I /Y "%SCHEMAS%" "%TEMP%\%SCHEMAS%\" >nul
xcopy /E /I /Y "%DLLS%" "%TEMP%\" >nul

if exist "%ZIP%" del "%ZIP%"
tar -caf "%ZIP%" -C "target" "anabeeb-windows"
rmdir /s /q %TEMP%

echo ✅ Windows release created: %ZIP%
