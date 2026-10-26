@echo off
cd /d "%~dp0"
if not exist target\release\rus.exe (
  cargo build --release
)
target\release\rus.exe %*
