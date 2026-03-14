/// Converts EnvlyError into a String suitable for Tauri command results.
/// Commands use `Result<T, String>` which Tauri automatically maps to InvokeError.
#[allow(dead_code)]
pub fn to_cmd_err(e: crate::core::error::EnvlyError) -> String {
    e.to_string()
}
