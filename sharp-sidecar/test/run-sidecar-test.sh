#!/bin/bash

# Script to run the sidecar tests in sequence

# Set the base directory to the location of this script
BASE_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SIDECAR_DIR="$(dirname "$BASE_DIR")"

echo "=========================================="
echo "Sharp Sidecar Communication Test"
echo "=========================================="
echo "Base directory: $BASE_DIR"
echo "Sidecar directory: $SIDECAR_DIR"
echo "----------------------------------------"

# Make sure we're running from the sidecar directory
cd "$SIDECAR_DIR" || { echo "Error: Failed to change to sidecar directory"; exit 1; }

# Function to run a node script and check exit code
run_script() {
  local script_path=$1
  local script_name=$(basename "$script_path")
  
  echo ""
  echo "Running: $script_name"
  echo "----------------------------------------"
  
  # Run the script
  node "$script_path"
  
  # Check exit code
  if [ $? -ne 0 ]; then
    echo "ERROR: Script $script_name failed with exit code $?"
    exit 1
  fi
  
  echo "----------------------------------------"
  echo "✓ Script $script_name completed successfully"
  echo ""
}

# Step 1: Run the data capture test
run_script "$BASE_DIR/run-test.js"

# Step 2: Analyze the results
run_script "$BASE_DIR/analyze-results.js"

# Step 3: Display the results
echo "=========================================="
echo "Test Results"
echo "=========================================="

# Display the analysis report
REPORT_FILE="$BASE_DIR/sidecar-analysis.md"
if [ -f "$REPORT_FILE" ]; then
  echo "Analysis report has been generated at:"
  echo "$REPORT_FILE"
  echo ""
  echo "Report Summary:"
  echo "----------------------------------------"
  
  # Extract the summary section from the markdown file
  # Display first 20 lines which should include summary
  head -n 20 "$REPORT_FILE"
  echo "..."
  echo "(See the full report for more details)"
else
  echo "ERROR: Analysis report not found at $REPORT_FILE"
fi

echo ""
echo "=========================================="
echo "Test Completed Successfully"
echo "==========================================" 