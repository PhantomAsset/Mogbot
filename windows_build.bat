@echo off
setlocal
cd /d "%~dp0"
where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust is not installed or Cargo is not in PATH.
  echo Install Rust from https://rustup.rs/ and restart this terminal.
  pause
  exit /b 1
)
cargo build --release
if errorlevel 1 (
  echo Build failed. Read the error above.
  pause
  exit /b 1
)
echo.
echo Build successful: target\release\tgbot.exe
pause
