@echo off
rem Run a command inside the Windows native-build environment Voxelle needs.
rem   scripts\win-env.cmd cargo build --release --features gpu-vulkan      (from src-tauri\)
rem   scripts\win-env.cmd npm run tauri dev -- --features gpu-vulkan       (from repo root)
rem Why: whisper.cpp is built by CMake. With the default Visual Studio generator the `cmake` crate
rem overwrites CMAKE_<LANG>_FLAGS_RELEASE and drops /O2 (whisper.cpp ends up unoptimized, ~10x
rem slower on CPU), and the Vulkan shader sub-build exceeds MAX_PATH under MSBuild. Ninja inside a
rem VS developer environment, with a short target dir, avoids both.
rem Needs: Visual Studio C++ tools, LLVM (libclang for bindgen); LunarG Vulkan SDK for gpu-vulkan;
rem NVIDIA CUDA Toolkit for gpu-cuda (set CMAKE_CUDA_ARCHITECTURES, e.g. 61, to build faster).
if "%~1"=="" (echo usage: win-env.cmd ^<command...^> & exit /b 2)
setlocal
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
for /f "usebackq delims=" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSDIR=%%i"
if not defined VSDIR (echo Visual Studio C++ tools not found & exit /b 1)
call "%VSDIR%\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
if not defined LIBCLANG_PATH (
	if exist "%ProgramFiles%\LLVM\bin\libclang.dll" (
		set "LIBCLANG_PATH=%ProgramFiles%\LLVM\bin"
	) else if exist "%VSDIR%\VC\Tools\Llvm\x64\bin\libclang.dll" (
		set "LIBCLANG_PATH=%VSDIR%\VC\Tools\Llvm\x64\bin"
	) else if exist "%ProgramFiles(x86)%\LLVM\bin\libclang.dll" (
		set "LIBCLANG_PATH=%ProgramFiles(x86)%\LLVM\bin"
	) else (
		set "LIBCLANG_PATH=%ProgramFiles%\LLVM\bin"
	)
)
if not defined VULKAN_SDK (
	for /f "delims=" %%i in ('dir /b /ad /o-n "%SystemDrive%\VulkanSDK" 2^>nul') do if not defined VULKAN_SDK set "VULKAN_SDK=%SystemDrive%\VulkanSDK\%%i"
)
if defined VULKAN_SDK set "PATH=%VULKAN_SDK%\Bin;%PATH%"
rem gpu-cuda: whisper-rs-sys needs CUDA_PATH (newest installed toolkit if unset).
set "CUDA_ROOT=%ProgramFiles%\NVIDIA GPU Computing Toolkit\CUDA"
if not defined CUDA_PATH if exist "%CUDA_ROOT%" (
	for /f "delims=" %%i in ('dir /b /ad /o-n "%CUDA_ROOT%" 2^>nul') do if not defined CUDA_PATH set "CUDA_PATH=%CUDA_ROOT%\%%i"
)
if defined CUDA_PATH set "PATH=%CUDA_PATH%\bin;%PATH%"
rem Target dir must be short (MAX_PATH) and contain no spaces: the Vulkan shader-generator
rem sub-build's resource-compiler step fails on paths with spaces (e.g. C:\Users\First Last).
if not defined CARGO_TARGET_DIR set "CARGO_TARGET_DIR=%USERPROFILE%\.voxelle-target"
if not "%CARGO_TARGET_DIR: =%"=="%CARGO_TARGET_DIR%" set "CARGO_TARGET_DIR=%SystemDrive%\vxt"
set "CMAKE_GENERATOR=Ninja"
%*
