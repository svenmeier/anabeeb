@echo off

echo Downloading FluidSynth 2.3.3 for Windows x64...

mkdir target >nul 2>nul

curl -L -o target\fluidsynth.zip ^
  https://github.com/FluidSynth/fluidsynth/releases/download/v2.4.6/fluidsynth-2.4.6-win10-x64.zip

if errorlevel 1 (
    echo ❌ Failed to download FluidSynth.
    exit /b 1
)

echo Extracting...
mkdir target\fluidsynth
cd target\fluidsynth
tar -xf ..\fluidsynth.zip
cd ..\..

if errorlevel 1 (
    echo ❌ Failed to extract FluidSynth.
    exit /b 1
)

echo Cleaning up...
del target\fluidsynth.zip

echo ✅ FluidSynth is ready
