@echo off
chcp 65001 >nul
cd /d "%~dp0"
echo [AgentPets] 브라우저 미리보기 (데모 데이터) - ui\ 파일을 저장하면 바로 반영돼요. 끝내려면 Ctrl+C
cargo run -p agentpets-devserver
pause
