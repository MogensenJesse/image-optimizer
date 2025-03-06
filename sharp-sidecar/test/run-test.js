const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

// Get path to the test script
const testScriptPath = path.join(__dirname, 'capture-sidecar-output.js');
const sidecarRootPath = path.join(__dirname, '..');

// Function to run the test
function runTest() {
  console.log('Running sidecar output capture test...');
  console.log(`Test script: ${testScriptPath}`);
  console.log(`Working directory: ${sidecarRootPath}`);
  
  try {
    // Use execSync to run the test script from the sidecar root directory
    const output = execSync(`node "${testScriptPath}"`, {
      cwd: sidecarRootPath,
      stdio: 'inherit',
      encoding: 'utf8'
    });
    
    console.log('Test completed successfully!');
    
    // Check if output files were created
    const logFile = path.join(__dirname, 'sidecar-output.log');
    const jsonFile = path.join(__dirname, 'sidecar-output.json');
    
    if (fs.existsSync(logFile)) {
      console.log(`Log file created: ${logFile}`);
      const stats = fs.statSync(logFile);
      console.log(`Log file size: ${stats.size} bytes`);
    } else {
      console.error(`ERROR: Log file not created: ${logFile}`);
    }
    
    if (fs.existsSync(jsonFile)) {
      console.log(`JSON file created: ${jsonFile}`);
      const stats = fs.statSync(jsonFile);
      console.log(`JSON file size: ${stats.size} bytes`);
      
      // Parse the JSON file to count message types
      const jsonData = JSON.parse(fs.readFileSync(jsonFile, 'utf8'));
      console.log(`Total captured messages: ${jsonData.length}`);
    } else {
      console.error(`ERROR: JSON file not created: ${jsonFile}`);
    }
    
    return 0;
  } catch (error) {
    console.error('Error running test:', error.message);
    return 1;
  }
}

// Run the test and exit with appropriate code
process.exit(runTest()); 