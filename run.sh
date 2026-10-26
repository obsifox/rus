#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"
BIN=target/release/rus
if [ ! -x "$BIN" ]; then
  cargo build --release
fi
exec "$BIN" "$@"
