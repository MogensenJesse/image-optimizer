# JavaScript Sidecar Cleanup

## Progress Summary

Legend:
🔄 = In Progress
⚠️ = Blocked/Has Issues
✅ = Completed

### Current Status:
Several redundancies and inconsistencies found in the JavaScript sidecar codebase.

### Next Implementation Steps:
1. ✅ Remove duplicate formatBytes function implementation
2. ✅ Standardize logging approach across the codebase
3. ✅ Organize format-specific settings into configuration files
4. ✅ Standardize error handling approach
5. ✅ Fix test script output and directory creation issues


## Implementation Plan

### 1. Remove Duplicate Code

[✅] Remove duplicate formatBytes function
   Short description: The formatBytes function is implemented identically in both optimizer.js and progress.js
   Prerequisites: None
   Files to modify:
   - sharp-sidecar/src/processing/optimizer.js
   - sharp-sidecar/src/index.js
   External dependencies: None
   Code to add/change/remove/move:
   - Remove the formatBytes function from optimizer.js (lines ~160-166)
   - Import formatBytes from progress.js where needed
   [✅] Cleanup after moving code (if applicable):
    - Update imports in optimizer.js to include formatBytes from utils/progress.js
    - Ensure all references to the formatBytes function in optimizer.js are updated

### 2. Standardize Logging

[✅] Replace direct debug import in worker-pool.js with project logger
   Short description: worker-pool.js uses a direct debug import instead of the project's logger module
   Prerequisites: None
   Files to modify:
   - sharp-sidecar/src/workers/worker-pool.js
   External dependencies: None
   Code to add/change/remove/move:
   - Replace require('debug') with the project's logger module
   - Update debug/error references throughout the file
   [✅] Cleanup after moving code (if applicable):
    - Ensure all log calls are updated to use the standardized format
    - Test to ensure logging still works correctly

[✅] Standardize error reporting in batch.js
   Short description: The error handling in batch.js combines direct console usage with the logger module
   Prerequisites: None
   Files to modify:
   - sharp-sidecar/src/processing/batch.js
   External dependencies: None
   Code to add/change/remove/move:
   - Standardize on the project's error/debug functions from utils
   - Ensure consistent error reporting format
   [✅] Cleanup after moving code (if applicable):
    - Verify all error handling paths report errors consistently

[✅] Fix test script output capture
   Short description: The test script doesn't properly capture all output sent to the Rust backend
   Prerequisites: None
   Files to modify:
   - sharp-sidecar/test/capture-sidecar-output.js
   - sharp-sidecar/src/workers/worker-pool.js
   - sharp-sidecar/src/utils/logger.js
   External dependencies: None
   Code to add/change/remove/move:
   - Improve console.log and console.error capture to handle multiple arguments
   - Add capturing of direct process.stdout.write calls
   - Fix task completion logging to show correct completion counts
   - Fix double logging issue in test script
   - Ensure debug messages use console.log instead of console.error
   - Create the optimized directory in the test script
   - Optimize log formatting by removing excessive empty lines and properly handling newlines
   [✅] Cleanup after moving code (if applicable):
    - Ensure all output to the Rust backend is properly captured in the test log
    - Verify that debug messages appear without ERROR prefix
    - Confirm that task completion counter increments properly
    - Verify log file has clean, consistent formatting without excessive blank lines

### 3. Improve Code Organization

[ ] Standardize JSDoc module documentation
   Short description: Module documentation style is inconsistent across files
   Prerequisites: None
   Files to modify:
   - All JavaScript files in the sidecar project
   External dependencies: None
   Code to add/change/remove/move:
   - Add consistent JSDoc @module tags to files missing proper documentation
   - Standardize format of existing documentation
   [ ] Cleanup after moving code (if applicable):
    - Check that all public functions have appropriate JSDoc comments

### 4. Refactor Configuration Management

[✅] Move hardcoded format settings to configuration files
   Short description: Format-specific optimization settings are hardcoded in getLosslessSettings()
   Prerequisites: None
   Files to modify:
   - sharp-sidecar/src/processing/optimizer.js
   - Create new configuration files as needed
   External dependencies: None
   Code to add/change/remove/move:
   - Move the format-specific settings from getLosslessSettings to appropriate config files
   - Add imports for these configurations
   [✅] Cleanup after moving code (if applicable):
    - Update imports and references
    - Ensure backward compatibility is maintained

### 5. Standardize Error Handling

[✅] Create consistent error handling strategy
   Short description: Different files use different approaches to error handling
   Prerequisites: None
   Files to modify:
   - All JavaScript files that handle errors, particularly:
     - sharp-sidecar/src/index.js
     - sharp-sidecar/src/processing/optimizer.js
     - sharp-sidecar/src/processing/batch.js
   External dependencies: None
   Code to add/change/remove/move:
   - Standardize try/catch blocks
   - Ensure consistent error reporting format
   - Add meaningful error messages
   [✅] Cleanup after moving code (if applicable):
    - Test error scenarios to ensure proper reporting


## Implementation Notes
- Make changes incrementally, one file at a time
- Keep error messages consistent with existing ones
- Maintain existing API contracts
- Don't overcomplicate things, keep it simple and functional
- Ensure proper cleanup after each code move
- Test each change to ensure it doesn't break existing functionality
- The sharp-sidecar project is a Node.js application, so ensure compatibility with CommonJS module system


## Completed Tasks

1. ✅ Remove duplicate formatBytes function
   - Removed the duplicate formatBytes function from optimizer.js
   - Added import for formatBytes from utils/progress.js
   - Verified all references to formatBytes in optimizer.js now use the imported function

2. ✅ Replace direct debug import in worker-pool.js
   - Replaced require('debug') with the project's logger module
   - Updated all debug/error references throughout the file to use the project's logger
   - Replaced direct console.error call with error() from the utils module

3. ✅ Standardize error reporting in batch.js
   - Replaced direct process.exit(1) calls with proper error throwing
   - Added JSDoc @throws annotation to document the error behavior
   - Ensured consistent error message format with variable declaration before logging

4. ✅ Fix test script output capture
   - Improved console.log and console.error capture to handle multiple arguments
   - Added capturing of direct process.stdout.write calls
   - Added immediate task completion logging in worker-pool.js
   - Fixed bug where JSON output was not properly captured in the test log
   - Fixed double logging issue by removing original console call
   - Updated logger.js to use console.log for debug messages instead of console.error
   - Added code to create the optimized directory in the test script
   - Verified that task completion counter now shows the correct incremental progress
   - Optimized log formatting by trimming whitespace and properly handling newlines
   - Implemented proper handling of multiline log messages to avoid excessive empty lines

5. ✅ Move hardcoded format settings to configuration files
   - Removed the duplicate getLosslessSettings function from optimizer.js
   - Updated the import in optimizer.js to include getLosslessSettings from the config/formats module
   - Updated module.exports to remove the now non-existent local getLosslessSettings function
   - Verified all imports in other files are still correct
   - Maintained backward compatibility by using the same function signature and return values

6. ✅ Standardize error handling strategy
   - Updated error handling in index.js, optimizer.js, and batch.js
   - Standardized the format of error messages with more descriptive information
   - Added pre-validation of inputs to catch errors earlier
   - Improved try/catch blocks to catch specific errors at appropriate points
   - Added consistent error message formatting with variable declaration before logging
   - Replaced direct process.exit() calls with throw statements for better error propagation
   - Added specific error messages for each potential failure point
   - Added additional error handling for cleanup operations that shouldn't interrupt the main flow


## Findings

### Known Issues:
- Duplicate formatBytes function in optimizer.js and progress.js ✅ Fixed
- Inconsistent logging approach (some files use require('debug'), others use the project's logger) ✅ Fixed
- Format-specific settings are hardcoded in optimizer.js ✅ Fixed
- Inconsistent JSDoc documentation style
- Mixed error handling strategies ✅ Fixed
- Test script not properly capturing all output sent to the Rust backend ✅ Fixed
- Task completion counter not being reflected in the logs at the correct time ✅ Fixed
- Test script failing due to missing optimized directory ✅ Fixed
- Debug messages being incorrectly prefixed with ERROR due to use of console.error ✅ Fixed
- Double logging in test script output ✅ Fixed

### Technical Insights:
- The project uses a worker thread model for parallelization
- Sharp is the primary image processing library
- The codebase is organized into clear modules:
  - processing: Image optimization logic
  - utils: Helper functions and logging
  - workers: Thread management
  - config: Format-specific settings
- The sidecar communicates with the main Tauri application through stdout/stderr
- Progress reporting is handled through a specialized messaging system
- Test scripts now properly capture both stdout and stderr, with debug messages going to stdout
- Communication between workers and the main thread happens via messages with specific types ("progress", "results", etc.)
- Task completion is now correctly tracked and reported in real-time
- Format-specific settings are now centralized in configuration files, eliminating duplication
- Error handling is now more robust with specific error messages for each potential failure point
- Input validation is now performed before processing to prevent errors during optimization
- Error propagation is improved by using throw statements instead of process.exit() calls