// src/lib.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::uninlined_format_args)]

use base64::Engine;
use clipboard::{ClipboardContext, ClipboardProvider};
use encoding_rs::GBK;
use std::fs;
#[cfg(target_os = "android")]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::async_runtime;
use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

mod opening_book;
use opening_book::{AddEntryRequest, JieqiOpeningBook, MoveData, OpeningBookStats};

// -------------------------------------------------------------
// type definition for the engine process state
type EngineProcess = Arc<Mutex<Option<CommandChild>>>;
// -------------------------------------------------------------

/// Shell process events are arbitrary byte chunks, while UCI/JAI are line
/// protocols. Buffer each stream independently so partial or combined chunks
/// never reach the frontend protocol parser as malformed messages.
fn frame_engine_output(buffer: &mut Vec<u8>, chunk: &[u8], flush: bool) -> Vec<String> {
    buffer.extend_from_slice(chunk);
    let mut lines = Vec::new();

    while let Some(end) = buffer.iter().position(|byte| *byte == b'\n') {
        let mut line: Vec<u8> = buffer.drain(..=end).collect();
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        lines.push(decode_engine_output(&line));
    }

    if flush && !buffer.is_empty() {
        let mut line = std::mem::take(buffer);
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        lines.push(decode_engine_output(&line));
    }

    lines
}

fn decode_engine_output(bytes: &[u8]) -> String {
    if cfg!(target_os = "windows") {
        let (text, ..) = GBK.decode(bytes);
        text.into_owned()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// Check if the engine file exists and is a file on Android.
/// This is a prerequisite for setting permissions and spawning.
#[cfg(target_os = "android")]
fn check_android_engine_file(path: &str) -> Result<(), String> {
    let engine_path = Path::new(path);

    // Check if file exists at the given path
    if !engine_path.exists() {
        return Err(format!("Engine file not found: {}", path));
    }

    // Check if the path points to a file, not a directory
    if let Ok(metadata) = fs::metadata(engine_path) {
        if !metadata.is_file() {
            return Err(format!("Path is not a file: {}", path));
        }
    } else {
        // This can happen if we lack permissions to read metadata
        return Err(format!("Cannot access engine file metadata: {}", path));
    }
    Ok(())
}

/// Copy a file from a user-accessible directory to the app's internal storage.
/// Used for the legacy engine scanning mechanism.
#[cfg(target_os = "android")]
fn copy_file_to_internal_storage(
    source_path_str: &str,
    app_handle: &AppHandle,
) -> Result<String, String> {
    let source_path = Path::new(source_path_str);
    if !source_path.exists() {
        let error_msg = format!("Source file not found: {}", source_path.display());
        let _ = app_handle.emit("engine-output", format!("[DEBUG] {}", error_msg));
        return Err(error_msg);
    }

    // Use dynamic bundle identifier for a robust internal storage path
    let bundle_identifier = &app_handle.config().identifier;
    let internal_dir = format!("/data/data/{}/files/engines", bundle_identifier);
    if let Err(e) = fs::create_dir_all(&internal_dir) {
        let error_msg = format!("Failed to create internal directory: {}", e);
        let _ = app_handle.emit("engine-output", format!("[DEBUG] {}", error_msg));
        return Err(error_msg);
    }

    // Generate destination path using the original filename
    let filename = source_path
        .file_name()
        .ok_or_else(|| "Invalid source path".to_string())?
        .to_str()
        .ok_or_else(|| "Invalid filename encoding".to_string())?;
    let dest_path_str = format!("{}/{}", internal_dir, filename);
    let dest_path = Path::new(&dest_path_str);

    let _ = app_handle.emit(
        "engine-output",
        format!(
            "[DEBUG] Copying file from {} to {}",
            source_path.display(),
            dest_path.display()
        ),
    );

    // Copy the file
    if let Err(e) = fs::copy(source_path, dest_path) {
        let error_msg = format!("Failed to copy file: {}", e);
        let _ = app_handle.emit("engine-output", format!("[DEBUG] {}", error_msg));
        return Err(error_msg);
    }

    let _ = app_handle.emit("engine-output", "[DEBUG] Setting executable permission...");

    // Set executable permissions (rwxr-xr-x) which is crucial on Android/Linux
    match fs::metadata(dest_path) {
        Ok(metadata) => {
            let mut permissions = metadata.permissions();
            // Set permissions to 0o755
            permissions.set_mode(0o755);

            if let Err(e) = fs::set_permissions(dest_path, permissions) {
                let error_msg = format!("Failed to set executable permission: {}", e);
                let _ = app_handle.emit("engine-output", format!("[DEBUG] {}", error_msg));
                return Err(error_msg);
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to get metadata for setting permissions: {}", e);
            let _ = app_handle.emit("engine-output", format!("[DEBUG] {}", error_msg));
            return Err(error_msg);
        }
    }

    let _ = app_handle.emit(
        "engine-output",
        format!(
            "[DEBUG] Successfully copied and made executable: {}",
            dest_path.display()
        ),
    );
    Ok(dest_path_str)
}

/// Save game notation to Android's external, user-accessible storage.
#[tauri::command]
async fn save_game_notation(
    content: String,
    filename: String,
    app: AppHandle,
) -> Result<String, String> {
    if !cfg!(target_os = "android") {
        return Err("This function is only available on Android".to_string());
    }

    // Use dynamic bundle identifier for external storage path
    let bundle_identifier = &app.config().identifier;
    let external_dir = format!(
        "/storage/emulated/0/Android/data/{}/files/notations",
        bundle_identifier
    );

    // Create the "notations" directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&external_dir) {
        let error_msg = format!("Failed to create notations directory: {}", e);
        return Err(error_msg);
    }

    // Generate the full file path for the new notation file
    let file_path_str = format!("{}/{}", external_dir, filename);
    let file_path = Path::new(&file_path_str);

    // Write the provided content to the file
    if let Err(e) = fs::write(file_path, content) {
        let error_msg = format!("Failed to write notation file: {}", e);
        return Err(error_msg);
    }

    Ok(file_path_str)
}

/// Save chart image to Android's external, user-accessible storage.
#[tauri::command]
async fn save_chart_image(
    content: String,
    filename: String,
    app: AppHandle,
) -> Result<String, String> {
    if !cfg!(target_os = "android") {
        return Err("This function is only available on Android".to_string());
    }

    // Use dynamic bundle identifier for external storage path
    let bundle_identifier = &app.config().identifier;
    let external_dir = format!(
        "/storage/emulated/0/Android/data/{}/files/charts",
        bundle_identifier
    );

    // Create the "charts" directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&external_dir) {
        let error_msg = format!("Failed to create charts directory: {}", e);
        return Err(error_msg);
    }

    // Decode base64 content
    let cleaned_content = content.replace("data:image/png;base64,", "");
    let decoded_content = match base64::engine::general_purpose::STANDARD.decode(&cleaned_content) {
        Ok(data) => data,
        Err(e) => return Err(format!("Failed to decode image data: {}", e)),
    };

    // Generate the full file path for the new chart image file
    let file_path_str = format!("{}/{}", external_dir, filename);
    let file_path = Path::new(&file_path_str);

    // Write the provided content to the file
    if let Err(e) = fs::write(file_path, decoded_content) {
        let error_msg = format!("Failed to write chart image file: {}", e);
        return Err(error_msg);
    }

    Ok(file_path_str)
}

/// Get the path to the configuration file, which varies by platform.
fn get_config_file_path(app: &AppHandle) -> Result<String, String> {
    if cfg!(target_os = "android") {
        // On Android, use the app's private internal data directory
        let bundle_identifier = &app.config().identifier;
        Ok(format!("/data/data/{}/files/config.ini", bundle_identifier))
    } else {
        // On desktop, for simplicity, use the same directory as the executable
        Ok("config.ini".to_string())
    }
}

/// Get the path to the autosave file, which varies by platform.
fn get_autosave_file_path(app: &AppHandle) -> Result<String, String> {
    if cfg!(target_os = "android") {
        // On Android, use the app's private internal data directory
        let bundle_identifier = &app.config().identifier;
        Ok(format!(
            "/data/data/{}/files/Autosave.json",
            bundle_identifier
        ))
    } else {
        // On desktop, use the same directory as the config file
        Ok("Autosave.json".to_string())
    }
}

/// Get the path to the opening book database file, which varies by platform.
fn get_opening_book_db_path(app: &AppHandle) -> Result<String, String> {
    if cfg!(target_os = "android") {
        // On Android, use the app's private internal data directory
        let bundle_identifier = &app.config().identifier;
        Ok(format!(
            "/data/data/{}/files/jieqi_openings.jb",
            bundle_identifier
        ))
    } else {
        // On desktop, use the same directory as the config file
        Ok("jieqi_openings.jb".to_string())
    }
}

fn ensure_parent_directory(path: &Path, file_kind: &str) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {} directory: {}", file_kind, error))?;
    }
    Ok(())
}

/// Write to a sibling temporary file and replace the target only once its
/// contents have reached the OS. This keeps a crash from leaving partial INI
/// or JSON data that cannot be loaded on the next launch.
fn write_file_atomically(path: &Path, content: &[u8], file_kind: &str) -> Result<(), String> {
    ensure_parent_directory(path, file_kind)?;

    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!("{}.tmp", extension))
        .unwrap_or_else(|| "tmp".to_string());
    let temporary_path = path.with_extension(extension);

    fs::write(&temporary_path, content)
        .map_err(|error| format!("Failed to write {} file: {}", file_kind, error))?;
    if let Err(error) = fs::File::open(&temporary_path).and_then(|file| file.sync_all()) {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!("Failed to sync {} file: {}", file_kind, error));
    }

    #[cfg(not(target_os = "windows"))]
    {
        fs::rename(&temporary_path, path).map_err(|error| {
            let _ = fs::remove_file(&temporary_path);
            format!("Failed to replace {} file: {}", file_kind, error)
        })?;
    }

    // Windows does not replace an existing destination with `rename`. The
    // fallback is not atomic there, but still avoids exposing a partial file.
    #[cfg(target_os = "windows")]
    {
        if path.exists() {
            fs::remove_file(path)
                .map_err(|error| format!("Failed to replace {} file: {}", file_kind, error))?;
        }
        fs::rename(&temporary_path, path).map_err(|error| {
            let _ = fs::remove_file(&temporary_path);
            format!("Failed to replace {} file: {}", file_kind, error)
        })?;
    }

    Ok(())
}

/// Load configuration from the config file.
#[tauri::command]
async fn load_config(app: AppHandle) -> Result<String, String> {
    let config_path = get_config_file_path(&app)?;
    let path = Path::new(&config_path);

    if !path.exists() {
        // If the config file doesn't exist, return an empty string, which is valid
        return Ok(String::new());
    }

    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Failed to read config file: {}", e)),
    }
}

/// Save configuration content to the config file.
#[tauri::command]
async fn save_config(content: String, app: AppHandle) -> Result<(), String> {
    let config_path = get_config_file_path(&app)?;
    let path = Path::new(&config_path);
    write_file_atomically(path, content.as_bytes(), "config")
}

/// Clear (delete) the configuration file.
#[tauri::command]
async fn clear_config(app: AppHandle) -> Result<(), String> {
    let config_path = get_config_file_path(&app)?;
    let path = Path::new(&config_path);

    if path.exists() {
        match fs::remove_file(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to delete config file: {}", e)),
        }
    } else {
        Ok(()) // File doesn't exist, so there's nothing to do
    }
}

/// Save game notation to autosave file.
#[tauri::command]
async fn save_autosave(content: String, app: AppHandle) -> Result<(), String> {
    let autosave_path = get_autosave_file_path(&app)?;
    let path = Path::new(&autosave_path);
    write_file_atomically(path, content.as_bytes(), "autosave")
}

/// Load game notation from autosave file.
#[tauri::command]
async fn load_autosave(app: AppHandle) -> Result<String, String> {
    let autosave_path = get_autosave_file_path(&app)?;
    let path = Path::new(&autosave_path);

    if !path.exists() {
        // If the autosave file doesn't exist, return an empty string
        return Ok(String::new());
    }

    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Failed to read autosave file: {}", e)),
    }
}

/// Get the path to the user-accessible engine directory for manual placement.
#[cfg(target_os = "android")]
fn get_user_engine_directory() -> String {
    "/storage/emulated/0/jieqibox/engines".to_string()
}

/// Scans user-facing directories for engines, copies them to internal storage,
/// and then returns a list of all engines available in internal storage.
#[cfg(target_os = "android")]
fn sync_and_list_engines(app_handle: &AppHandle) -> Result<Vec<String>, String> {
    let bundle_identifier = &app_handle.config().identifier;
    let source_dirs = vec![
        get_user_engine_directory(),
        format!(
            "/storage/emulated/0/Android/data/{}/files/engines",
            bundle_identifier
        ),
    ];
    let internal_dir_str = format!("/data/data/{}/files/engines", bundle_identifier);

    let _ = app_handle.emit(
        "engine-output",
        format!(
            "[DEBUG] Syncing engines. Internal dir: {}. Source dirs: {:?}",
            internal_dir_str, source_dirs
        ),
    );

    // Ensure the internal engine directory exists
    if let Err(e) = fs::create_dir_all(&internal_dir_str) {
        let error_msg = format!(
            "Failed to create internal directory '{}': {}",
            internal_dir_str, e
        );
        let _ = app_handle.emit("engine-output", format!("[DEBUG] {}", error_msg));
        return Err(error_msg);
    } else {
        let _ = app_handle.emit(
            "engine-output",
            format!(
                "[DEBUG] Internal directory created/exists: {}",
                internal_dir_str
            ),
        );
    }

    // Iterate over all possible source directories
    for user_dir in &source_dirs {
        let _ = app_handle.emit(
            "engine-output",
            format!("[DEBUG] Checking source directory: {}", user_dir),
        );
        let user_path = Path::new(user_dir);

        if !user_path.exists() {
            let _ = app_handle.emit(
                "engine-output",
                format!(
                    "[DEBUG] Source directory does not exist, skipping: {}",
                    user_dir
                ),
            );
            continue;
        }

        if let Ok(entries) = fs::read_dir(user_path) {
            for entry_result in entries {
                if let Ok(entry) = entry_result {
                    let path = entry.path();
                    if path.is_file() {
                        if let Err(e) =
                            copy_file_to_internal_storage(path.to_str().unwrap_or(""), app_handle)
                        {
                            let _ = app_handle.emit(
                                "engine-output",
                                format!("[DEBUG] Failed to copy file {}: {}", path.display(), e),
                            );
                        }
                    }
                }
            }
        }
    }

    // List all files in the internal directory and return their full paths
    let mut available_engines = Vec::new();
    if let Ok(entries) = fs::read_dir(&internal_dir_str) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(path_str) = path.to_str() {
                    available_engines.push(path_str.to_string());
                }
            }
        }
    }

    let _ = app_handle.emit(
        "engine-output",
        format!(
            "[DEBUG] Available internal engines: {:?}",
            available_engines
        ),
    );
    Ok(available_engines)
}

/// Explicitly kills the currently running engine process, if any.
#[tauri::command]
async fn kill_engine(process_state: tauri::State<'_, EngineProcess>) -> Result<(), String> {
    if let Some(child) = process_state.lock().unwrap().take() {
        let _ = child.kill();
    }
    Ok(())
}

/// Spawns a new engine process with a given path and arguments.
#[tauri::command]
async fn spawn_engine(
    path: String,
    args: Vec<String>,
    app: AppHandle,
    process_state: tauri::State<'_, EngineProcess>,
) -> Result<(), String> {
    if cfg!(target_os = "android") {
        let _ = app.emit(
            "engine-output",
            format!("[DEBUG] Spawning engine: Path={}, Args={:?}", path, args),
        );
    }

    // The path must be an absolute, accessible file path
    let final_path = path;

    #[cfg(target_os = "android")]
    {
        if let Err(e) = check_android_engine_file(&final_path) {
            let _ = app.emit(
                "engine-output",
                format!("[DEBUG] Engine file validation failed: {}", e),
            );
            return Err(e);
        }
        let _ = app.emit("engine-output", "[DEBUG] Engine file validation passed.");
    }

    // Ensure any previous engine process is terminated before starting a new one
    kill_engine(process_state.clone()).await.ok();

    // The engine's working directory should be its parent directory
    let engine_dir = Path::new(&final_path)
        .parent()
        .ok_or_else(|| "Failed to get engine directory".to_string())?
        .to_str()
        .ok_or_else(|| "Failed to convert engine directory to string".to_string())?;

    // Spawn the new process
    let (mut rx, child) = match app
        .shell()
        .command(&final_path)
        .args(args)
        .current_dir(engine_dir)
        .spawn()
    {
        Ok(result) => result,
        Err(e) => {
            let error_msg = format!("Failed to spawn engine: {}", e);
            if cfg!(target_os = "android") {
                let _ = app.emit("engine-output", format!("[DEBUG] {}", error_msg));
            }
            return Err(error_msg);
        }
    };

    // Store the new child process in the shared state
    *process_state.lock().unwrap() = Some(child);

    // Spawn an async task to listen for the engine's stdout/stderr.
    // Keep stream buffers separate: stderr must not complete a partial stdout
    // line (or vice versa).
    let app_clone = app.clone();
    async_runtime::spawn(async move {
        let mut stdout_buffer = Vec::new();
        let mut stderr_buffer = Vec::new();
        while let Some(event) = rx.recv().await {
            let (buffer, chunk) = match event {
                CommandEvent::Stdout(buf) => (&mut stdout_buffer, buf),
                CommandEvent::Stderr(buf) => (&mut stderr_buffer, buf),
                _ => continue,
            };

            for line in frame_engine_output(buffer, &chunk, false) {
                let _ = app_clone.emit("engine-output", line);
            }
        }

        for line in frame_engine_output(&mut stdout_buffer, &[], true) {
            let _ = app_clone.emit("engine-output", line);
        }
        for line in frame_engine_output(&mut stderr_buffer, &[], true) {
            let _ = app_clone.emit("engine-output", line);
        }
    });

    Ok(())
}

/// Sends a command string to the running engine process.
#[tauri::command]
async fn send_to_engine(
    command: String,
    process_state: tauri::State<'_, EngineProcess>,
) -> Result<(), String> {
    if let Some(child) = process_state.lock().unwrap().as_mut() {
        child
            .write(format!("{}\n", command).as_bytes())
            .map_err(|e| format!("Failed to write to engine: {}", e))?;
        Ok(())
    } else {
        Err("Engine not running.".into())
    }
}

/// Get the path to a directory where users can manually place engines.
#[cfg(target_os = "android")]
#[tauri::command]
async fn get_default_android_engine_path() -> Result<String, String> {
    Ok(get_user_engine_directory())
}

/// Check file permissions on Android.
#[cfg(target_os = "android")]
#[tauri::command]
async fn check_android_file_permissions(path: String) -> Result<bool, String> {
    if let Ok(metadata) = fs::metadata(Path::new(&path)) {
        Ok(metadata.is_file())
    } else {
        Ok(false)
    }
}

/// Get the app's bundle identifier (package name).
#[cfg(target_os = "android")]
#[tauri::command]
async fn get_bundle_identifier(app: AppHandle) -> Result<String, String> {
    Ok(app.config().identifier.clone())
}

/// Scan for engines available for the app to use.
#[cfg(target_os = "android")]
#[tauri::command]
async fn scan_android_engines(app: AppHandle) -> Result<Vec<String>, String> {
    sync_and_list_engines(&app)
}

/// Emits an event to the Android native side to request a file via SAF.
#[cfg(target_os = "android")]
#[tauri::command]
async fn request_saf_file_selection(
    name: String,
    args: String,
    has_nnue: bool,
    app: AppHandle,
) -> Result<(), String> {
    // This command's only job is to forward the request to the native layer
    let _ = app.emit(
        "request-saf-file-selection",
        serde_json::json!({
            "name": name,
            "args": args,
            "has_nnue": has_nnue
        }),
    );
    Ok(())
}

/// Handles the result from the SAF file picker, after the native code has
/// copied the selected file to a temporary, accessible location.
#[cfg(target_os = "android")]
#[tauri::command]
async fn handle_saf_file_result(
    temp_file_path: String, // IMPORTANT: This must be a real file path, not a content:// URI
    filename: String,
    name: String,
    args: String,
    has_nnue: bool,
    app: AppHandle,
) -> Result<(), String> {
    let _ = app.emit(
        "engine-output",
        format!(
            "[DEBUG] SAF result for engine '{}': TempPath={}, Filename={}",
            name, temp_file_path, filename
        ),
    );

    if temp_file_path.is_empty() {
        return Err("SAF file processing failed: temporary path is empty.".to_string());
    }

    // Generate a unique identifier for this specific engine instance to be used as the directory name.
    // This prevents conflicts if an engine file and a desired directory have the same name,
    // and also allows adding the same engine multiple times.
    let engine_instance_id = format!("{}_{}", name, chrono::Utc::now().timestamp_millis());

    // Define the final destination directory for the engine using the unique ID.
    let bundle_identifier = &app.config().identifier;
    let engine_base_dir = format!(
        "/data/data/{}/files/engines/{}",
        bundle_identifier, &engine_instance_id
    );

    // Create the engine-specific directory
    if let Err(e) = fs::create_dir_all(&engine_base_dir) {
        let error_msg = format!("Failed to create final engine directory: {}", e);
        let _ = app.emit("engine-output", format!("[DEBUG] {}", error_msg));
        return Err(error_msg);
    }

    // Define the final path for the engine executable
    let final_path_str = format!("{}/{}", engine_base_dir, &filename);

    // Move the file from the temporary location to the final destination
    if let Err(e) = fs::rename(&temp_file_path, &final_path_str) {
        let error_msg = format!(
            "Failed to move engine file from temp to final destination: {}",
            e
        );
        let _ = app.emit("engine-output", format!("[DEBUG] {}", error_msg));
        // Fallback to copy if rename fails (e.g., cross-device link)
        if let Err(copy_err) = fs::copy(&temp_file_path, &final_path_str) {
            let copy_error_msg = format!("Fallback copy also failed: {}", copy_err);
            let _ = app.emit("engine-output", format!("[DEBUG] {}", copy_error_msg));
            return Err(copy_error_msg);
        } else {
            // Copy succeeded, remove the original temp file
            let _ = fs::remove_file(&temp_file_path);
        }
    }

    // Set executable permission on the final file
    let final_path = Path::new(&final_path_str);
    let mut perms = fs::metadata(final_path)
        .map_err(|e| e.to_string())?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(final_path, perms).map_err(|e| e.to_string())?;

    // Handle NNUE file if requested
    if has_nnue {
        let _ = app.emit(
            "engine-output",
            "[DEBUG] Engine requires NNUE file, requesting file selection...",
        );

        // Request NNUE file selection from the frontend
        let nnue_request_data = serde_json::json!({
            "engine_name": name,
            "engine_path": final_path_str,
            "args": args,
            "engine_instance_id": engine_instance_id
        });

        // Store the engine data temporarily and request NNUE file
        app.emit("request-nnue-file", nnue_request_data)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Create the ManagedEngine object to send back to the frontend
    let new_engine_data = serde_json::json!({
        "id": format!("engine_{}", chrono::Utc::now().timestamp_millis()),
        "name": name,
        "path": final_path_str,
        "args": args
    });

    // Notify the frontend that the engine has been successfully added
    app.emit("android-engine-added", new_engine_data)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Handle NNUE file result from SAF file selection
#[cfg(target_os = "android")]
#[tauri::command]
async fn handle_nnue_file_result(
    temp_file_path: String,
    filename: String,
    engine_name: String,
    engine_path: String,
    args: String,
    engine_instance_id: String,
    app: AppHandle,
) -> Result<(), String> {
    let _ = app.emit(
        "engine-output",
        format!(
            "[DEBUG] NNUE file result for engine '{}': TempPath={}, Filename={}",
            engine_name, temp_file_path, filename
        ),
    );

    if temp_file_path.is_empty() {
        return Err("NNUE file processing failed: temporary path is empty.".to_string());
    }

    // Get the engine directory path
    let bundle_identifier = &app.config().identifier;
    let engine_base_dir = format!(
        "/data/data/{}/files/engines/{}",
        bundle_identifier, &engine_instance_id
    );

    // Define the final path for the NNUE file in the same directory as the engine
    let final_nnue_path_str = format!("{}/{}", engine_base_dir, &filename);

    // Move the NNUE file from the temporary location to the final destination
    if let Err(e) = fs::rename(&temp_file_path, &final_nnue_path_str) {
        let error_msg = format!(
            "Failed to move NNUE file from temp to final destination: {}",
            e
        );
        let _ = app.emit("engine-output", format!("[DEBUG] {}", error_msg));
        // Fallback to copy if rename fails (e.g., cross-device link)
        if let Err(copy_err) = fs::copy(&temp_file_path, &final_nnue_path_str) {
            let copy_error_msg = format!("Fallback copy also failed: {}", copy_err);
            let _ = app.emit("engine-output", format!("[DEBUG] {}", copy_error_msg));
            return Err(copy_error_msg);
        } else {
            // Copy succeeded, remove the original temp file
            let _ = fs::remove_file(&temp_file_path);
        }
    }

    let _ = app.emit(
        "engine-output",
        format!(
            "[DEBUG] NNUE file successfully copied to: {}",
            final_nnue_path_str
        ),
    );

    // Create the ManagedEngine object to send back to the frontend
    let new_engine_data = serde_json::json!({
        "id": format!("engine_{}", chrono::Utc::now().timestamp_millis()),
        "name": engine_name,
        "path": engine_path,
        "args": args
    });

    // Notify the frontend that the engine has been successfully added
    app.emit("android-engine-added", new_engine_data)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Opens a URL in the system's default browser.
#[tauri::command]
async fn open_external_url(url: String, app: AppHandle) -> Result<(), String> {
    let result = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", &url]).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(&url).spawn()
    } else if cfg!(target_os = "android") {
        // On Android, delegate to the native layer
        let _ = app.emit("open-external-url", url);
        return Ok(());
    } else {
        // Linux
        Command::new("xdg-open").arg(&url).spawn()
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to open URL: {}", e)),
    }
}

// Opening Book Commands

/// Add an entry to the opening book
#[tauri::command]
async fn opening_book_add_entry(request: AddEntryRequest, app: AppHandle) -> Result<bool, String> {
    let db_path = get_opening_book_db_path(&app)?;
    let book = JieqiOpeningBook::new(db_path).map_err(|e| e.to_string())?;
    book.add_entry(&request).map_err(|e| e.to_string())
}

/// Add several entries in one SQLite transaction.
#[tauri::command]
async fn opening_book_add_entries(
    requests: Vec<AddEntryRequest>,
    app: AppHandle,
) -> Result<bool, String> {
    let db_path = get_opening_book_db_path(&app)?;
    let book = JieqiOpeningBook::new(db_path).map_err(|e| e.to_string())?;
    book.upsert_all(&requests).map_err(|e| e.to_string())
}

/// Delete an entry from the opening book
#[tauri::command]
async fn opening_book_delete_entry(
    fen: String,
    uci_move: String,
    app: AppHandle,
) -> Result<bool, String> {
    let db_path = get_opening_book_db_path(&app)?;
    let book = JieqiOpeningBook::new(db_path).map_err(|e| e.to_string())?;
    book.delete_entry(&fen, &uci_move)
        .map_err(|e| e.to_string())
}

/// Query moves for a given FEN position
#[tauri::command]
async fn opening_book_query_moves(fen: String, app: AppHandle) -> Result<Vec<MoveData>, String> {
    let db_path = get_opening_book_db_path(&app)?;
    let book = JieqiOpeningBook::new(db_path).map_err(|e| e.to_string())?;
    book.query_moves(&fen).map_err(|e| e.to_string())
}

/// Get opening book statistics
#[tauri::command]
async fn opening_book_get_stats(app: AppHandle) -> Result<OpeningBookStats, String> {
    let db_path = get_opening_book_db_path(&app)?;
    let book = JieqiOpeningBook::new(db_path).map_err(|e| e.to_string())?;
    book.get_stats().map_err(|e| e.to_string())
}

/// Clear all entries from the opening book
#[tauri::command]
async fn opening_book_clear_all(app: AppHandle) -> Result<(), String> {
    let db_path = get_opening_book_db_path(&app)?;
    let book = JieqiOpeningBook::new(db_path).map_err(|e| e.to_string())?;
    book.clear_all().map_err(|e| e.to_string())
}

/// Export all opening book entries
#[tauri::command]
async fn opening_book_export_all(app: AppHandle) -> Result<String, String> {
    let db_path = get_opening_book_db_path(&app)?;
    let book = JieqiOpeningBook::new(db_path).map_err(|e| e.to_string())?;
    let entries = book.export_all().map_err(|e| e.to_string())?;
    serde_json::to_string(&entries).map_err(|e| e.to_string())
}

/// Import opening book entries from JSON
#[tauri::command]
async fn opening_book_import_entries(
    json_data: String,
    app: AppHandle,
) -> Result<(i32, Vec<String>), String> {
    let db_path = get_opening_book_db_path(&app)?;
    let book = JieqiOpeningBook::new(db_path).map_err(|e| e.to_string())?;

    let entries: Vec<opening_book::OpeningBookEntry> =
        serde_json::from_str(&json_data).map_err(|e| e.to_string())?;

    let mut requests = Vec::new();
    let mut legacy_entries = Vec::new();
    let mut errors = Vec::new();

    for entry in entries {
        if entry.fen.trim().is_empty() {
            match hex::decode(&entry.key) {
                Ok(key) if key.len() == 12 => {
                    for move_data in entry.moves {
                        legacy_entries.push((key.clone(), move_data));
                    }
                }
                _ => errors.push(format!(
                    "Cannot import legacy opening-book entry with invalid key '{}'",
                    entry.key
                )),
            }
        } else {
            for move_data in entry.moves {
                requests.push(AddEntryRequest {
                    fen: entry.fen.clone(),
                    uci_move: move_data.uci_move,
                    priority: move_data.priority,
                    wins: move_data.wins,
                    draws: move_data.draws,
                    losses: move_data.losses,
                    allowed: move_data.allowed,
                    comment: move_data.comment,
                });
            }
        }
    }

    let imported = (requests.len() + legacy_entries.len()) as i32;
    book.upsert_all(&requests).map_err(|e| e.to_string())?;
    book.upsert_legacy_entries(&legacy_entries)
        .map_err(|e| e.to_string())?;

    Ok((imported, errors))
}

/// Export opening book database file to a specified path
#[tauri::command]
async fn opening_book_export_db(destination_path: String, app: AppHandle) -> Result<(), String> {
    let source_path = get_opening_book_db_path(&app)?;
    fs::copy(source_path, destination_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Import opening book database file from a specified path
#[tauri::command]
async fn opening_book_import_db(source_path: String, app: AppHandle) -> Result<(), String> {
    let dest_path = get_opening_book_db_path(&app)?;
    fs::copy(source_path, dest_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Save game notation with a file dialog (for desktop platforms)
/// On Android, this delegates to the existing save_game_notation function
#[tauri::command]
async fn save_game_notation_with_dialog(
    content: String,
    default_filename: String,
    app: AppHandle,
) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        // On Android, use the existing save_game_notation logic
        return save_game_notation(content, default_filename, app).await;
    }

    #[cfg(not(target_os = "android"))]
    {
        use tauri_plugin_dialog::{DialogExt, FilePath};

        // Show save file dialog
        let file_path = app
            .dialog()
            .file()
            .set_file_name(&default_filename)
            .add_filter("JSON files", &["json"])
            .add_filter("All files", &["*"])
            .blocking_save_file();

        match file_path {
            Some(FilePath::Path(path)) => {
                // Write content to the selected file
                fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))?;

                Ok(path.to_string_lossy().to_string())
            }
            Some(FilePath::Url(_)) => Err("URL paths are not supported".to_string()),
            None => Err("Save dialog was cancelled".to_string()),
        }
    }
}

/// Copy text to clipboard
#[tauri::command]
async fn copy_to_clipboard(text: String, _app: AppHandle) -> Result<(), String> {
    let mut ctx: ClipboardContext =
        ClipboardProvider::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    ctx.set_contents(text)
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))
}

/// Paste text from clipboard
#[tauri::command]
async fn paste_from_clipboard(_app: AppHandle) -> Result<String, String> {
    let mut ctx: ClipboardContext =
        ClipboardProvider::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    ctx.get_contents()
        .map_err(|e| format!("Failed to paste from clipboard: {}", e))
}

// The main entry point for the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(None)) as EngineProcess)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            spawn_engine,
            kill_engine,
            send_to_engine,
            open_external_url,
            save_game_notation,
            save_chart_image,
            load_config,
            save_config,
            clear_config,
            save_autosave,
            load_autosave,
            save_game_notation_with_dialog,
            copy_to_clipboard,
            paste_from_clipboard,
            // Opening book commands
            opening_book_add_entry,
            opening_book_add_entries,
            opening_book_delete_entry,
            opening_book_query_moves,
            opening_book_get_stats,
            opening_book_clear_all,
            opening_book_export_all,
            opening_book_import_entries,
            opening_book_export_db,
            opening_book_import_db,
            // Android-specific commands
            #[cfg(target_os = "android")]
            get_bundle_identifier,
            #[cfg(target_os = "android")]
            get_default_android_engine_path,
            #[cfg(target_os = "android")]
            check_android_file_permissions,
            #[cfg(target_os = "android")]
            scan_android_engines,
            #[cfg(target_os = "android")]
            request_saf_file_selection,
            #[cfg(target_os = "android")]
            handle_saf_file_result,
            #[cfg(target_os = "android")]
            handle_nnue_file_result
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{frame_engine_output, write_file_atomically};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn frames_split_and_combined_protocol_chunks() {
        let mut buffer = Vec::new();

        assert_eq!(
            frame_engine_output(&mut buffer, b"uci", false),
            Vec::<String>::new()
        );
        assert_eq!(
            frame_engine_output(&mut buffer, b"ok\nready", false),
            vec!["uciok"]
        );
        assert_eq!(
            frame_engine_output(&mut buffer, b"ok\nbestmove a0a1\n", false),
            vec!["readyok", "bestmove a0a1"]
        );
        assert!(buffer.is_empty());
    }

    #[test]
    fn flushes_final_unterminated_protocol_line() {
        let mut buffer = b"bestmove a0a1".to_vec();
        assert_eq!(
            frame_engine_output(&mut buffer, b"", true),
            vec!["bestmove a0a1"]
        );
        assert!(buffer.is_empty());
    }

    #[test]
    fn atomically_replaces_existing_file() {
        let unique_suffix = format!(
            "jieqi-box-atomic-write-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after the Unix epoch")
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(unique_suffix);
        let path = directory.join("state.json");
        let temporary_path = directory.join("state.json.tmp");

        write_file_atomically(&path, b"first", "test").expect("first write should succeed");
        write_file_atomically(&path, b"second", "test").expect("replacement should succeed");

        assert_eq!(
            fs::read(&path).expect("target should be readable"),
            b"second"
        );
        assert!(
            !temporary_path.exists(),
            "temporary file should be removed after replacement"
        );

        fs::remove_dir_all(directory).expect("temporary directory should be removable");
    }
}
