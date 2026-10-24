use std::path::PathBuf;

/// Return existing local paths supplied when the application was launched.
/// The executable name is omitted; non-path arguments are ignored.
#[tauri::command]
pub fn get_cli_files() -> Vec<String> {
    std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}
