# Development build script for WASM + React hot reloading (Windows)

Write-Host "🚀 Starting development build pipeline..." -ForegroundColor Green

# Function to build WASM
function Build-Wasm {
    Write-Host "🦀 Building WASM package..." -ForegroundColor Yellow
    
    Push-Location charting
    $result = wasm-pack build --target web --out-dir ../web/pkg --dev
    $success = $LASTEXITCODE -eq 0
    Pop-Location
    
    if ($success) {
        Write-Host "✅ WASM build successful" -ForegroundColor Green
        Write-Host "📦 WASM files output to web/pkg/" -ForegroundColor Blue
        
        # Trigger Vite reload by touching a watched file
        if (-not (Test-Path "web\src\wasm-trigger.ts")) {
            New-Item -Path "web\src\wasm-trigger.ts" -ItemType File -Force | Out-Null
        }
        (Get-Item "web\src\wasm-trigger.ts").LastWriteTime = Get-Date
        Write-Host "🔄 Triggered React hot reload" -ForegroundColor Cyan
    } else {
        Write-Host "❌ WASM build failed" -ForegroundColor Red
        return $false
    }
    return $true
}

# Initial build
Build-Wasm

# Watch for Rust file changes
Write-Host "👀 Watching for Rust file changes..." -ForegroundColor Cyan
Write-Host "Press Ctrl+C to stop" -ForegroundColor Yellow

# Get initial state of files
$watcher = New-Object System.IO.FileSystemWatcher
$watcher.Path = Join-Path $PSScriptRoot "..\charting"
$watcher.Filter = "*.*"
$watcher.IncludeSubdirectories = $true
$watcher.EnableRaisingEvents = $true

# Define the action to take when files change
$action = {
    $path = $Event.SourceEventArgs.FullPath
    $changeType = $Event.SourceEventArgs.ChangeType
    
    # Filter for Rust files and Cargo files
    if ($path -match '\.(rs|toml|lock)$') {
        Write-Host "📝 File changed: $path" -ForegroundColor Yellow
        Write-Host "📝 Rust files changed, rebuilding..." -ForegroundColor Yellow
        
        # Use a small delay to batch multiple file changes
        Start-Sleep -Milliseconds 500
        
        # Build in the main thread context
        $scriptPath = $Event.MessageData.ScriptPath
        Push-Location $scriptPath
        Build-Wasm
        Pop-Location
        
        Write-Host "⏰ $(Get-Date -Format 'HH:mm:ss'): Ready for changes..." -ForegroundColor Green
    }
}

# Register event handlers
$handlers = @()
$handlers += Register-ObjectEvent -InputObject $watcher -EventName "Changed" -Action $action -MessageData @{ScriptPath = $PSScriptRoot}
$handlers += Register-ObjectEvent -InputObject $watcher -EventName "Created" -Action $action -MessageData @{ScriptPath = $PSScriptRoot}
$handlers += Register-ObjectEvent -InputObject $watcher -EventName "Deleted" -Action $action -MessageData @{ScriptPath = $PSScriptRoot}
$handlers += Register-ObjectEvent -InputObject $watcher -EventName "Renamed" -Action $action -MessageData @{ScriptPath = $PSScriptRoot}

try {
    Write-Host "⏰ $(Get-Date -Format 'HH:mm:ss'): Ready for changes..." -ForegroundColor Green
    
    # Keep the script running
    while ($true) {
        Start-Sleep -Seconds 1
    }
} finally {
    # Cleanup
    foreach ($handler in $handlers) {
        Unregister-Event -SourceIdentifier $handler.Name
    }
    $watcher.Dispose()
}