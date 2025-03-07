# PowerShell script to run the sidecar tests

# Set the base directory to the location of this script
$BASE_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path
$SIDECAR_DIR = Split-Path -Parent $BASE_DIR

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Sharp Sidecar Communication Test" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Base directory: $BASE_DIR"
Write-Host "Sidecar directory: $SIDECAR_DIR"
Write-Host "------------------------------------------"

# Make sure we're running from the sidecar directory
Set-Location $SIDECAR_DIR
if (-not $?) {
    Write-Host "Error: Failed to change to sidecar directory" -ForegroundColor Red
    exit 1
}

# Function to run a node script and check exit code
function Invoke-Script {
    param (
        [string]$ScriptPath
    )
    
    $ScriptName = Split-Path -Leaf $ScriptPath
    
    Write-Host ""
    Write-Host "Running: $ScriptName" -ForegroundColor Yellow
    Write-Host "------------------------------------------"
    
    # Run the script
    node $ScriptPath
    
    # Check exit code
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Script $ScriptName failed with exit code $LASTEXITCODE" -ForegroundColor Red
        exit 1
    }
    
    Write-Host "------------------------------------------"
    Write-Host "✓ Script $ScriptName completed successfully" -ForegroundColor Green
    Write-Host ""
}

# Run the test
Invoke-Script "$BASE_DIR\run-test.js"

# Display the log file
$LOG_FILE = "$BASE_DIR\sidecar-output.log"
if (Test-Path $LOG_FILE) {
    Write-Host "==========================================" -ForegroundColor Cyan
    Write-Host "Sidecar Output Log" -ForegroundColor Cyan
    Write-Host "==========================================" -ForegroundColor Cyan
    Write-Host "Log file: $LOG_FILE"
    Write-Host ""
    Write-Host "Log Contents:" -ForegroundColor Yellow
    Write-Host "------------------------------------------"
    
    # Display the log file
    Get-Content $LOG_FILE
} else {
    Write-Host "ERROR: Log file not found at $LOG_FILE" -ForegroundColor Red
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Test Completed Successfully" -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Cyan 