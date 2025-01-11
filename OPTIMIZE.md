# Optimization Plan

## Code Organization
- [ ] Create `utils` module
  ```rust
  mod utils;  // New module for shared functionality
  ```
- [ ] Move shared functions to appropriate locations:
  - Validation → `utils/validation.rs`
  - File operations → `utils/fs.rs`
  - Error types → `utils/error.rs`
  - Format handling → `utils/formats.rs`

## Error Handling
- [ ] Create custom error types
  ```rust
  // utils/error.rs
  pub enum OptimizerError {
      ValidationError(String),
      ProcessingError(String),
      IOError(std::io::Error),
      WorkerError(String)
  }
  ```
- [ ] Replace String errors with OptimizerError throughout codebase

## Validation Improvements
- [ ] Create shared validation module
  ```rust
  // utils/validation.rs
  pub async fn validate_task(task: &ImageTask) -> Result<(), OptimizerError> {
      validate_paths(task)?;
      validate_settings(task)?;
      validate_format(task)?;
      Ok(())
  }
  ```
- [ ] Remove duplicate validation code from `optimizer.rs` and `validation.rs`

## State Management
- [ ] Implement Drop for AppState
  ```rust
  impl Drop for AppState {
      fn drop(&mut self) {
          if let Some(pool) = self.worker_pool.try_lock() {
              // Cleanup resources
          }
      }
  }
  ```

## Performance Optimizations
- [ ] Replace Vec with HashSet for active tasks
  ```rust
  pub struct ImageOptimizer {
      active_tasks: Arc<Mutex<HashSet<String>>>
  }
  ```

- [ ] Optimize Sharp communication
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
- [ ] Create centralized format enum
  ```rust
  // utils/formats.rs
  pub enum ImageFormat {
      JPEG,
      PNG,
      WebP,
      AVIF,
  }
  
  impl ImageFormat {
      pub fn validate_quality(&self, quality: u32) -> bool {
          // Format-specific validation
      }
  }
  ```

## Implementation Priority
1. Error types (improves debugging and error handling)
2. Validation consolidation (reduces code duplication)
3. Format centralization (improves maintainability)
4. Performance optimizations (improves efficiency)
5. Code organization (improves maintainability)

## Documentation Updates
Update BACKEND.md after completing these improvements:
- [ ] Error handling (new error types and flow)
- [ ] Utils module structure and organization
- [ ] Format handling improvements
- [ ] Performance optimizations
- [ ] State management changes

## Migration Checklist
For each code move/change, ensure:

### Code Migration
- [ ] Source code is properly removed after moving
- [ ] All imports are updated in affected files
- [ ] All type references are updated
- [ ] All function calls are updated to new locations
- [ ] No duplicate code remains

### Module Updates
- [ ] Update `mod.rs` files with new modules
- [ ] Update `use` statements in all affected files
- [ ] Update public exports (`pub use`) where needed
- [ ] Check visibility modifiers (`pub`, `pub(crate)`, etc.)

### Dependency Chain
- [ ] Check for circular dependencies
- [ ] Update module initialization order
- [ ] Verify all paths in imports
- [ ] Test affected module integration

### Testing
- [ ] Unit tests are moved with their code
- [ ] Integration tests are updated
- [ ] All tests pass after migration
- [ ] No orphaned test files remain

## Notes
- Each improvement is independent and can be implemented separately
- Test thoroughly after each change
- Update documentation to reflect changes
- Consider backward compatibility
- Keep BACKEND.md in sync with code changes
