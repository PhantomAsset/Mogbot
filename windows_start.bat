@echo off
setlocal
cd /d "%~dp0"
if not exist "target\release\tgbot.exe" (
  echo tgbot.exe not found. Run windows_build.bat first.
  pause
  exit /b 1
)
if "%TG_TOKEN%"=="" (
  set /p TG_TOKEN=Paste your Telegram bot token:
)
if "%TG_TOKEN%"=="" (
  echo TG_TOKEN is empty.
  pause
  exit /b 1
)
target\release\tgbot.exe
pause
