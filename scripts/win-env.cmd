@echo off
rem Run a command inside the Windows native-build environment Voxelle needs.
rem   scripts\win-env.cmd cargo build --release --features gpu-vulkan      (from src-tauri\)
rem   scripts\win-env.cmd npm run tauri dev -- --features gpu-vulkan       (from repo root)
rem Why: whisper.cpp is built by CMake. With the default Visual Studio generator the `cmake` crate
rem overwrites CMAKE_<LANG>_FLAGS_RELEASE and drops /O2 (whisper.cpp ends up unoptimized, ~10x
rem slower on CPU), and the Vulkan shader sub-build exceeds MAX_PATH under MSBuild. Ninja inside a
rem VS developer environment, with a short target dir, avoids both.
rem Needs: Visual Studio C++ tools, LLVM (libclang for bindgen); LunarG Vulkan SDK for gpu-vulkan.
if "%~1"=="" (echo usage: win-env.cmd ^<command...^> & exit /b 2)
setlocal
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
for /f "usebackq delims=" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSDIR=%%i"
if not defined VSDIR (echo Visual Studio C++ tools not found & exit /b 1)
call "%VSDIR%\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
if not defined LIBCLANG_PATH set "LIBCLANG_PATH=%ProgramFiles%\LLVM\bin"
if defined VULKAN_SDK set "PATH=%VULKAN_SDK%\Bin;%PATH%"
rem Target dir must be short (MAX_PATH) and contain no spaces: the Vulkan shader-generator
rem sub-build's resource-compiler step fails on paths with spaces (e.g. C:\Users\First Last).
if not defined CARGO_TARGET_DIR set "CARGO_TARGET_DIR=%USERPROFILE%\.voxelle-target"
if not "%CARGO_TARGET_DIR: =%"=="%CARGO_TARGET_DIR%" set "CARGO_TARGET_DIR=%SystemDrive%\vxt"
set "CMAKE_GENERATOR=Ninja"
%*
