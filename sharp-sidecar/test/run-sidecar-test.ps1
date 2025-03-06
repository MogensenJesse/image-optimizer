# PowerShell script to run the sidecar tests in sequence

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

# Step 1: Run the data capture test
Invoke-Script "$BASE_DIR\run-test.js"

# Step 2: Analyze the results
Invoke-Script "$BASE_DIR\analyze-results.js"

# Step 3: Display the results
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Test Results" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# Display the analysis report
$REPORT_FILE = "$BASE_DIR\sidecar-analysis.md"
if (Test-Path $REPORT_FILE) {
    Write-Host "Analysis report has been generated at:" -ForegroundColor Green
    Write-Host "$REPORT_FILE"
    Write-Host ""
    Write-Host "Report Summary:" -ForegroundColor Yellow
    Write-Host "------------------------------------------"
    
    # Extract the summary section from the markdown file
    # Display first 20 lines which should include summary
    Get-Content $REPORT_FILE -Head 20
    Write-Host "..."
    Write-Host "(See the full report for more details)"
} else {
    Write-Host "ERROR: Analysis report not found at $REPORT_FILE" -ForegroundColor Red
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Test Completed Successfully" -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Cyan 