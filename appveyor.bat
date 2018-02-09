echo on
SetLocal EnableDelayedExpansion

REM This is the recommended way to choose the toolchain version, according to
REM Appveyor's documentation.
SET PATH=C:\Program Files (x86)\MSBuild\%TOOLCHAIN_VERSION%\Bin;%PYTHON%;%PATH%

set VCVARSALL="C:\Program Files (x86)\Microsoft Visual Studio %TOOLCHAIN_VERSION%\VC\vcvarsall.bat"

if [%Platform%] NEQ [x64] goto win32
set TARGET_ARCH=x86_64
set TARGET_PROGRAM_FILES=%ProgramFiles%
call %VCVARSALL% amd64
if %ERRORLEVEL% NEQ 0 exit 1
goto download

:win32
echo on
if [%Platform%] NEQ [Win32] exit 1
set TARGET_ARCH=i686
set TARGET_PROGRAM_FILES=%ProgramFiles(x86)%
call %VCVARSALL% amd64_x86
if %ERRORLEVEL% NEQ 0 exit 1
goto download

:download
REM vcvarsall turns echo off
echo on

mkdir windows_build_tools
mkdir windows_build_tools\
echo Downloading Yasm...
powershell -Command "(New-Object Net.WebClient).DownloadFile('http://www.tortall.net/projects/yasm/releases/yasm-1.3.0-win64.exe', 'windows_build_tools\yasm.exe')"
if %ERRORLEVEL% NEQ 0 (
  echo ...downloading Yasm failed.
  exit 1
)

set RUST_URL=https://static.rust-lang.org/dist/rust-%RUST%-%TARGET_ARCH%-pc-windows-msvc.msi
echo Downloading %RUST_URL%...
mkdir build
powershell -Command "(New-Object Net.WebClient).DownloadFile('%RUST_URL%', 'build\rust-%RUST%-%TARGET_ARCH%-pc-windows-msvc.msi')"
if %ERRORLEVEL% NEQ 0 (
  echo ...downloading Rust failed.
  exit 1
)

start /wait msiexec /i build\rust-%RUST%-%TARGET_ARCH%-pc-windows-msvc.msi INSTALLDIR="%TARGET_PROGRAM_FILES%\Rust %RUST%" /quiet /qn /norestart
if %ERRORLEVEL% NEQ 0 exit 1

set PATH=%TARGET_PROGRAM_FILES%\Rust %RUST%\bin;%cd%\windows_build_tools;%PATH%
set TARGET=nightly-%TARGET_ARCH%-pc-windows-msvc

link /?
cl /?
rustc --version
cargo --version

cd rust
cargo test %CARGO_MODE%
if %ERRORLEVEL% NEQ 0 exit 1
cd ..

dir rust\target\release

python -m pip install --upgrade pip setuptools wheel

python -m pip install git+https://github.com/getsentry/milksnake.git

python setup.py build
if %ERRORLEVEL% NEQ 0 exit 1

set
python setup.py test
if %ERRORLEVEL% NEQ 0 exit 1
