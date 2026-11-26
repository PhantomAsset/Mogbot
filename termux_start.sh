#!/data/data/com.termux/files/usr/bin/bash
set -e
cd "$(dirname "$0")"
if [ ! -x target/release/tgbot ]; then
  echo "Binary not found. Run: bash termux_setup.sh"
  exit 1
fi
termux-wake-lock 2>/dev/null || true
if [ -z "${TG_TOKEN:-}" ]; then
  read -rsp "Paste your Telegram bot token: " TG_TOKEN
  echo
  export TG_TOKEN
fi
exec ./target/release/tgbot
