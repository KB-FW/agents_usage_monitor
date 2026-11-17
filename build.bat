@echo off
chcp 65001 >nul
cd /d "%~dp0"
cargo tauri --version >nul 2>&1
if errorlevel 1 (
  echo [AgentMeter] tauri-cli 설치 중 - 처음 한 번만, 몇 분 걸려요
  cargo install tauri-cli --version "^2" --locked
)
echo [AgentMeter] 배포용 빌드 중...
cargo tauri build
if errorlevel 1 (
  pause
  exit /b 1
)
echo.
echo 완료: target\release\agentmeter.exe  (설치 파일: target\release\bundle)
explorer "target\release\bundle"
pause
