@echo off
setlocal enabledelayedexpansion

echo Visual Regression Baseline Update Tool
echo ======================================

REM Check if we're updating from CI artifacts
if "%1"=="--ci-artifacts" (
    echo Updating baselines from CI artifacts...
    echo Please download the 'cypress-screenshots' artifact from the failed GitHub Actions run
    echo Extract it to: web\cypress\screenshots\
    pause
    
    REM Copy screenshots to baselines
    if exist "cypress\screenshots" (
        echo Copying screenshots to baselines...
        
        REM Create backup of current baselines
        if exist "cypress\fixtures\visual-baselines" (
            echo Creating backup of current baselines...
            for /f "tokens=2-4 delims=/ " %%a in ('date /t') do set date=%%c%%a%%b
            for /f "tokens=1-2 delims=: " %%a in ('time /t') do set time=%%a%%b
            move cypress\fixtures\visual-baselines cypress\fixtures\visual-baselines.backup.!date!!time!
        )
        
        REM Create new baselines directory
        if not exist "cypress\fixtures\visual-baselines" mkdir cypress\fixtures\visual-baselines
        
        REM Copy all non-failed screenshots to baselines
        for /r cypress\screenshots %%f in (*.png) do (
            echo %%f | findstr /v "failed" >nul && copy "%%f" cypress\fixtures\visual-baselines\
        )
        
        echo Baselines updated successfully!
    ) else (
        echo Error: No screenshots found in cypress\screenshots\
        exit /b 1
    )
) else (
    echo Generating new baselines locally...
    
    REM Check if servers are running
    echo Checking if required servers are running...
    curl -s http://localhost:3000 >nul 2>&1
    if errorlevel 1 (
        echo Error: React dev server is not running on port 3000
        echo Please run 'npm run dev:suite' in another terminal
        exit /b 1
    )
    
    REM Run Cypress to generate baselines
    echo Running tests in baseline generation mode...
    npx cypress run --env visualRegressionType=base --spec "cypress/e2e/visual-regression-*.cy.ts"
    
    echo New baselines generated successfully!
)

echo.
echo Next steps:
echo 1. Review the new baseline images in cypress\fixtures\visual-baselines\
echo 2. Commit the updated baselines to git
echo 3. Push to your branch to re-run CI tests