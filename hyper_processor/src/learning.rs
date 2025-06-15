use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use chrono::Local;

static LEARNING_FILE: Mutex<Option<File>> = Mutex::new(None);

pub fn init(output_path: String) -> Result<(), String> {
    let mut file_guard = LEARNING_FILE.lock()
        .map_err(|e| format!("Failed to lock learning file: {}", e))?;
    
    // Create or truncate the file
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&output_path)
        .map_err(|e| format!("Failed to open learning output file {}: {}", output_path, e))?;
    
    // Write header
    writeln!(file, "# HyperProcessor Learning Mode Results").map_err(|e| e.to_string())?;
    writeln!(file, "# Started: {}", Local::now().format("%Y-%m-%d %H:%M:%S")).map_err(|e| e.to_string())?;
    writeln!(file).map_err(|e| e.to_string())?;
    
    *file_guard = Some(file);
    Ok(())
}

pub fn record_library(library_name: &str) {
    if let Ok(mut file_guard) = LEARNING_FILE.lock() {
        if let Some(ref mut file) = *file_guard {
            // Write as JSON line for easy parsing
            let _ = writeln!(file, r#"{{"library": "{}"}}"#, library_name);
            let _ = file.flush(); // Ensure it's written immediately
        }
    }
}

pub fn save_and_cleanup() {
    if let Ok(mut file_guard) = LEARNING_FILE.lock() {
        if let Some(mut file) = file_guard.take() {
            // Write footer
            let _ = writeln!(file);
            let _ = writeln!(file, "# Ended: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = file.flush();
        }
    }
} 