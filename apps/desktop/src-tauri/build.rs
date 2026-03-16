// =============================================================================
// Tauri build script
// =============================================================================
// This runs at compile time before the main code is compiled.
// Tauri uses it to generate platform-specific resources (app icon,
// Windows resource file, etc.).
// =============================================================================

fn main() {
    tauri_build::build()
}
