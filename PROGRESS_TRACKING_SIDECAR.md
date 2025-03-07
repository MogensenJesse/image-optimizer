# JavaScript Sidecar Cleanup

## Progress Summary

Legend:
🔄 = In Progress
⚠️ = Blocked/Has Issues
✅ = Completed

### Current Status:
Several redundancies and inconsistencies found in the JavaScript sidecar codebase.

### Next Implementation Steps:
1. Remove duplicate formatBytes function implementation
2. Standardize logging approach across the codebase
3. Organize format-specific settings into configuration files
4. Standardize error handling approach


## Implementation Plan

### 1. Remove Duplicate Code

[ ] Remove duplicate formatBytes function
   Short description: The formatBytes function is implemented identically in both optimizer.js and progress.js
   Prerequisites: None
   Files to modify:
   - sharp-sidecar/src/processing/optimizer.js
   - sharp-sidecar/src/index.js
   External dependencies: None
   Code to add/change/remove/move:
   - Remove the formatBytes function from optimizer.js (lines ~160-166)
   - Import formatBytes from progress.js where needed
   [ ] Cleanup after moving code (if applicable):
    - Update imports in optimizer.js to include formatBytes from utils/progress.js
    - Ensure all references to the formatBytes function in optimizer.js are updated

### 2. Standardize Logging

[ ] Replace direct debug import in worker-pool.js with project logger
   Short description: worker-pool.js uses a direct debug import instead of the project's logger module
   Prerequisites: None
   Files to modify:
   - sharp-sidecar/src/workers/worker-pool.js
   External dependencies: None
   Code to add/change/remove/move:
   - Replace require('debug') with the project's logger module
   - Update debug/error references throughout the file
   [ ] Cleanup after moving code (if applicable):
    - Ensure all log calls are updated to use the standardized format
    - Test to ensure logging still works correctly

[ ] Standardize error reporting in batch.js
   Short description: The error handling in batch.js combines direct console usage with the logger module
   Prerequisites: None
   Files to modify:
   - sharp-sidecar/src/processing/batch.js
   External dependencies: None
   Code to add/change/remove/move:
   - Standardize on the project's error/debug functions from utils
   - Ensure consistent error reporting format
   [ ] Cleanup after moving code (if applicable):
    - Verify all error handling paths report errors consistently

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

[ ] Move hardcoded format settings to configuration files
   Short description: Format-specific optimization settings are hardcoded in getLosslessSettings()
   Prerequisites: None
   Files to modify:
   - sharp-sidecar/src/processing/optimizer.js
   - Create new configuration files as needed
   External dependencies: None
   Code to add/change/remove/move:
   - Move the format-specific settings from getLosslessSettings to appropriate config files
   - Add imports for these configurations
   [ ] Cleanup after moving code (if applicable):
    - Update imports and references
    - Ensure backward compatibility is maintained

### 5. Standardize Error Handling

[ ] Create consistent error handling strategy
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
   [ ] Cleanup after moving code (if applicable):
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

No tasks completed yet.


## Findings

### Known Issues:
- Duplicate formatBytes function in optimizer.js and progress.js
- Inconsistent logging approach (some files use require('debug'), others use the project's logger)
- Format-specific settings are hardcoded in optimizer.js
- Inconsistent JSDoc documentation style
- Mixed error handling strategies

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