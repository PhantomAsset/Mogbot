#!/data/data/com.termux/files/usr/bin/bash
set -e
cd "$(dirname "$0")"
pkg update -y
pkg install rust clang make pkg-config openssl-tool -y
cargo build --release
echo
echo "Build successful: target/release/tgbot"
echo "Start with: ./termux_start.sh"
