@echo off
chcp 65001 >nul
cd /d "%~dp0"
cargo tauri --version >nul 2>&1
if errorlevel 1 (
  echo [AgentPets] tauri-cli 설치 중 - 처음 한 번만, 몇 분 걸려요
  cargo install tauri-cli --version "^2" --locked
)
echo [AgentPets] 개발 모드 - 실제 사용량으로 앱 실행. ui\ 저장 = 창 새로고침, Rust 저장 = 자동 재빌드
cargo tauri dev
pause
