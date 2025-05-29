@echo off
REM Adjust the URL and the number of loops as needed.
for /l %%i in (1,1,5) do (
    start /b cmd /c "curl http://127.0.0.1:3001/data/100M_x"
    start /b cmd /c "curl http://127.0.0.1:3001/data/100M_y"
)
echo All requests launched!
pause
