# Worker Pool Metrics Integration

## Progress Summary

Legend:
🔄 = In Progress
⚠️ = Blocked/Has Issues
✅ = Completed

### Current Status:
- Planning implementation of worker pool metrics (total workers and tasks per worker)
- No blockers identified

### Next Implementation Steps:
1. Add worker metrics collection in Sharp sidecar
2. Implement IPC message handling for metrics
3. Extend Rust benchmarking reporter
4. Update console reporting format

## Implementation Plan

### 1. Sharp Sidecar Worker Metrics Collection

[ ] Add worker metrics tracking to WorkerPool
   Short description: Implement tracking of total workers and tasks per worker in the Sharp sidecar
   Prerequisites: None
   Files to modify: 
   - sharp-sidecar/src/workers/worker-pool.js
   External dependencies: None
   Code to add/change/remove/move:
   - Add metrics object to WorkerPool class
   - Track worker count and tasks per worker
   - Add metrics to batch completion message
   [ ] Cleanup after moving code:
    - Ensure proper initialization in constructor
    - Update worker message handling

[ ] Implement metrics message protocol
   Short description: Define and implement the metrics message structure for IPC communication
   Prerequisites: Worker metrics tracking implementation
   Files to modify:
   - sharp-sidecar/src/processing/batch.js
   External dependencies: None
   Code to add/change/remove/move:
   - Define metrics message structure
   - Add metrics to batch completion payload
   [ ] Cleanup after moving code:
    - Update message handling

### 2. Rust Backend Integration

[ ] Add metrics handling to ProcessPool
   Short description: Extend ProcessPool to receive and process worker metrics
   Prerequisites: Sidecar metrics implementation
   Files to modify:
   - src-tauri/src/processing/pool/process_pool.rs
   External dependencies: None
   Code to add/change/remove/move:
   - Add worker metrics structs
   - Implement metrics collection from sidecar messages
   [ ] Cleanup after moving code:
    - Update error handling
    - Update process pool documentation

[ ] Extend BenchmarkMetrics
   Short description: Add worker metrics to benchmarking system
   Prerequisites: ProcessPool metrics handling
   Files to modify:
   - src-tauri/src/benchmarking/metrics.rs
   External dependencies: None
   Code to add/change/remove/move:
   - Add worker metrics fields
   - Update metrics collection
   [ ] Cleanup after moving code:
    - Update struct documentation
    - Update metrics calculations

[ ] Update console reporting
   Short description: Add worker metrics to console output
   Prerequisites: BenchmarkMetrics extension
   Files to modify:
   - src-tauri/src/benchmarking/reporter.rs
   External dependencies: None
   Code to add/change/remove/move:
   - Add worker metrics to report format
   - Update console output formatting
   [ ] Cleanup after moving code:
    - Update report formatting
    - Update documentation

## Implementation Notes
- Keep metrics collection lightweight to minimize performance impact
- Ensure thread-safe metrics collection in worker pool
- Maintain consistent error handling patterns
- Use existing IPC channels for communication
- Keep metrics format simple and extensible

## Completed Tasks

## Findings

### Known Issues:
- None identified yet

### Technical Insights:
- Using existing IPC channel reduces implementation complexity
- Metrics collection should have minimal performance overhead
- Worker pool already has access to required metrics data