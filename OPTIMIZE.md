# Optimization Plan

## Code Organization
- [x] Create `utils` module
  ```rust
  mod utils;  // New module for shared functionality
  ```
- [ ] Move shared functions to appropriate locations:
  - [x] Validation → `utils/validation.rs`
  - [x] File operations → `utils/fs.rs`
  - [x] Error types → `utils/error.rs`
  - [x] Format handling → `utils/formats.rs`

## Error Handling
- [x] Create custom error types
  ```rust
  // utils/error.rs
  #[derive(Error, Debug, Serialize)]
  pub enum OptimizerError {
      ValidationError(String),
      ProcessingError(String),
      IOError(String),
      WorkerError(String)
  }
  ```
- [x] Replace String errors with OptimizerError throughout codebase
  - [x] validation.rs
  - [x] optimizer.rs
  - [x] worker.rs
  - [x] commands/image.rs
  - [x] commands/worker.rs

## Validation Improvements
- [x] Create shared validation module
  ```rust
  // utils/validation.rs
  pub async fn validate_task(task: &ImageTask) -> OptimizerResult<()> {
      validate_paths(task)?;
      validate_settings(task)?;
      validate_format(task)?;
      Ok(())
  }
  ```
- [x] Remove duplicate validation code from `optimizer.rs` and `validation.rs`

## State Management
- [x] Implement Drop for AppState
  ```rust
  impl Drop for AppState {
      fn drop(&mut self) {
          let runtime = tokio::runtime::Runtime::new().unwrap();
          runtime.block_on(async {
              // Cleanup resources
              if let Ok(mut pool) = self.worker_pool.try_lock() {
                  pool.take();  // Remove and drop the worker pool
              }
          });
      }
  }
  ```

## Performance Optimizations
- [x] Replace Vec with HashSet for active tasks
  ```rust
  pub struct ImageOptimizer {
      active_tasks: Arc<Mutex<HashSet<String>>>
  }
  ```

- [x] Optimize Sharp communication
  ```rust
  // Batch commands instead of individual calls
  pub async fn process_batch(&self, tasks: Vec<ImageTask>) -> Result<...> {
      // Send tasks in chunks to reduce process spawning
      for chunk in tasks.chunks(10) {
          self.run_sharp_process_batch(chunk).await?;
      }
  }
  ```

## Format Handling
- [x] Create centralized format enum
  ```rust
  // utils/formats.rs
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
  pub enum ImageFormat {
      JPEG,
      PNG,
      WebP,
      AVIF,
  }
  ```
- [x] Add format-specific functionality:
  - Default quality values
  - Extension validation
  - Format conversion rules
  - Quality validation

## Implementation Priority
1. Error types (improves debugging and error handling)
2. Validation consolidation (reduces code duplication)
3. Format centralization (improves maintainability)
4. Performance optimizations (improves efficiency)
5. Code organization (improves maintainability)

## Documentation Updates
Update BACKEND.md after completing these improvements:
- [x] Error handling (new error types and flow)
- [x] Utils module structure and organization
- [x] Format handling improvements
- [ ] Performance optimizations
- [ ] State management changes

## Implementation Progress
1. ✅ Created utils module with error types
2. ✅ Implemented OptimizerError with all variants
3. ✅ Updated validation.rs to use new error types
4. ✅ Updated optimizer.rs with new error types
5. ✅ Updated worker.rs with new error types
6. ✅ Updated command handlers with new error types
7. ✅ Implemented shared validation module
8. ✅ Implemented centralized format handling
9. 🔄 Next: Implement performance optimizations

## Migration Checklist
For each code move/change, ensure:

### Code Migration
- [x] Source code is properly removed after moving (error types, validation)
- [x] All imports are updated in affected files (all modules)
- [x] All type references are updated (all modules)
- [x] All function calls are updated to new locations
- [x] No duplicate code remains

### Module Updates
- [x] Update `mod.rs` files with new modules
- [x] Update `use` statements in all affected files
- [x] Update public exports (`pub use`) where needed
- [x] Check visibility modifiers (`pub`, `pub(crate)`, etc.)

### Dependency Chain
- [x] Check for circular dependencies
  - Moved `ImageTask` to `core/types.rs`
  - Removed `worker/types.rs`
  - Updated module visibility
- [x] Update module initialization order
  - Reordered modules in `lib.rs` based on dependencies:
    1. `utils` (base utilities)
    2. `core` (types and state)
    3. `processing` (image processing)
    4. `worker` (worker pool)
    5. `commands` (command handlers)
- [x] Verify all paths in imports
  - Fixed circular dependency between utils and worker
  - Updated ImageTask imports to use core instead of worker
  - Consolidated imports in processing modules

## Notes
- Each improvement is independent and can be implemented separately
- Update documentation to reflect changes
- Consider backward compatibility
- Keep BACKEND.md in sync with code changes
