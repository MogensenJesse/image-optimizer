const { execSync } = require("node:child_process");
const path = require("node:path");
const fs = require("node:fs");

// Get path to the test script
const testScriptPath = path.join(__dirname, "capture-sidecar-output.js");
const sidecarRootPath = path.join(__dirname, "..");

// Function to run the test
function runTest() {
  console.log("Running sidecar output capture test...");
  console.log(`Test script: ${testScriptPath}`);
  console.log(`Working directory: ${sidecarRootPath}`);

  try {
    // Use execSync to run the test script from the sidecar root directory
    const _output = execSync(`node "${testScriptPath}"`, {
      cwd: sidecarRootPath,
      stdio: "inherit",
      encoding: "utf8",
    });

    console.log("Test completed successfully!");

    // Check if output file was created
    const logFile = path.join(__dirname, "sidecar-output.log");

    if (fs.existsSync(logFile)) {
      console.log(`Log file created: ${logFile}`);
      const stats = fs.statSync(logFile);
      console.log(`Log file size: ${stats.size} bytes`);
    } else {
      console.error(`ERROR: Log file not created: ${logFile}`);
      return 1;
    }

    return 0;
  } catch (error) {
    console.error("Error running test:", error.message);
    return 1;
  }
}

// Run the test and exit with appropriate code
process.exit(runTest());
