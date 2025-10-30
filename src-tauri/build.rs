fn main() {
    // Tauri build will embed Windows resources (icons) if RC.EXE is available
    // If RC.EXE is not found, this will fail with a clear error message
    tauri_build::build()
}
