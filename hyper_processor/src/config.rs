use serde::Deserialize;
use std::path::Path;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Settings {
    #[serde(default)]
    pub whitelisted_filenames: Vec<String>,
    #[serde(default)]
    pub audit_mode: bool,
    #[serde(default)]
    pub learning_mode: bool,
    #[serde(default)]
    pub learning_output: Option<String>,
}

impl Settings {
    /// Loads configuration from file (default: rasp_config.yaml) and environment variables.
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_path_str = std::env::var("HYPER_RASP_CONFIG")
            .unwrap_or_else(|_| "rasp_config.yaml".to_string());
        let config_path = Path::new(&config_path_str);

        let builder = config::Config::builder()
            // Defaults are now handled entirely by `serde(default)` and `Default` trait
            
            // Load config file (optional)
            .add_source(config::File::with_name(&config_path_str).required(false))
            
            // Load environment variables (HYPER_RASP_AUDIT_MODE, HYPER_RASP_WHITELISTED_FILENAMES)
            .add_source(config::Environment::with_prefix("HYPER_RASP").separator("__"));
            
        // Build and deserialize
        let config = builder.build()?;
        let mut settings: Self = config.try_deserialize()?;

        // Handle HYPER_RASP_WHITELIST environment variable explicitly
        // The config crate expects HYPER_RASP_WHITELISTED_FILENAMES but users might use HYPER_RASP_WHITELIST
        if let Ok(whitelist_str) = std::env::var("HYPER_RASP_WHITELIST") {
            let whitelist_items: Vec<String> = whitelist_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            
            if !whitelist_items.is_empty() {
                settings.whitelisted_filenames = whitelist_items;
            }
        }
        
        // Handle HYPER_RASP_LEARNING_MODE environment variable
        if let Ok(learning) = std::env::var("HYPER_RASP_LEARNING_MODE") {
            if learning.to_lowercase() == "true" {
                settings.learning_mode = true;
                // If learning mode is enabled, also check for output file
                if let Ok(output) = std::env::var("HYPER_RASP_LEARNING_OUTPUT") {
                    settings.learning_output = Some(output);
                }
            }
        }
        
        // In learning mode, force audit mode to be true
        if settings.learning_mode {
            settings.audit_mode = true;
        }

        // --- Check config file permissions after attempting to load it ---
        if config_path.exists() {
             match fs::metadata(config_path) {
                Ok(metadata) => {
                    let perms = metadata.permissions();
                    let mode = perms.mode();
                    // Check if 'group' or 'other' has write permissions (0o020 or 0o002)
                    if mode & 0o022 != 0 {
                         // Use eprintln here as logger might not be initialized yet
                         // when Settings::load is called early in init_library.
                         // We need the PID/Name prefix for context.
                         // Let's try getting it again, though not ideal.
                         let pid = std::process::id();
                         let comm = fs::read_to_string("/proc/self/comm")
                             .map(|s| s.trim().to_string())
                             .unwrap_or_else(|_| "<unknown>".to_string());
                         let log_prefix = format!("[{pid} {}]", comm);

                         eprintln!(
                            "{} [Config] WARNING: Configuration file '{}' has insecure permissions ({:#o}). Others or group members may be able to modify the whitelist. Recommend setting permissions to 644 or 600.",
                            log_prefix,
                            config_path.display(),
                            mode & 0o777 // Display standard permission bits
                         );
                         // We don't use log::warn here because the logger initialization
                         // might happen *after* config loading in src/lib.rs.
                         // eprintln ensures the message is seen during startup.
                    }
                }
                Err(e) => {
                     // Failed to get metadata, log an error (again, use eprintln)
                     let pid = std::process::id();
                     let comm = fs::read_to_string("/proc/self/comm")
                         .map(|s| s.trim().to_string())
                         .unwrap_or_else(|_| "<unknown>".to_string());
                     let log_prefix = format!("[{pid} {}]", comm);
                     eprintln!(
                        "{} [Config] ERROR: Could not read metadata for config file '{}': {}. Cannot verify permissions.", 
                        log_prefix, config_path.display(), e
                     );
                }
            }
        } // else: config file doesn't exist, no permissions to check.

        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use std::sync::Mutex;
    
    // Global mutex to ensure tests don't run in parallel
    static TEST_MUTEX: Mutex<()> = Mutex::new(());
    
    // Helper to clear all HYPER_RASP env vars
    fn clear_env_vars() {
        std::env::remove_var("HYPER_RASP_CONFIG");
        std::env::remove_var("HYPER_RASP_AUDIT_MODE");
        std::env::remove_var("HYPER_RASP_WHITELIST");
        std::env::remove_var("HYPER_RASP_WHITELISTED_FILENAMES");
        std::env::remove_var("HYPER_RASP_LEARNING_MODE");
        std::env::remove_var("HYPER_RASP_LEARNING_OUTPUT");
    }
    
    #[test]
    fn test_default_settings() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let settings = Settings::default();
        assert_eq!(settings.audit_mode, false);
        assert_eq!(settings.whitelisted_filenames.len(), 0);
    }
    
    #[test]
    fn test_load_from_yaml() {
        let _guard = TEST_MUTEX.lock().unwrap();
        clear_env_vars(); // Clean start
        
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test_config.yaml");
        
        let yaml_content = r#"
audit_mode: true
whitelisted_filenames:
  - custom_lib.so
  - another_lib.so.1
"#;
        
        fs::write(&config_path, yaml_content).unwrap();
        
        // Set environment variable to point to our test config
        std::env::set_var("HYPER_RASP_CONFIG", config_path.to_str().unwrap());
        
        let settings = Settings::load().unwrap();
        
        assert_eq!(settings.audit_mode, true);
        assert_eq!(settings.whitelisted_filenames.len(), 2);
        assert!(settings.whitelisted_filenames.contains(&"custom_lib.so".to_string()));
        assert!(settings.whitelisted_filenames.contains(&"another_lib.so.1".to_string()));
        
        // Clean up
        clear_env_vars();
    }
    
    #[test]
    fn test_env_var_override() {
        let _guard = TEST_MUTEX.lock().unwrap();
        clear_env_vars(); // Clean start
        
        // Create a config file with audit_mode: false
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test_config.yaml");
        
        let yaml_content = r#"
audit_mode: false
whitelisted_filenames:
  - from_file.so
"#;
        
        fs::write(&config_path, yaml_content).unwrap();
        
        // Set environment variables
        std::env::set_var("HYPER_RASP_CONFIG", config_path.to_str().unwrap());
        std::env::set_var("HYPER_RASP_AUDIT_MODE", "true");
        std::env::set_var("HYPER_RASP_WHITELIST", "from_env1.so,from_env2.so");
        
        // Note: Settings::load() in lib.rs has custom handling for HYPER_RASP_AUDIT_MODE
        // But here we're testing just the config loading part
        let settings = Settings::load().unwrap();
        
        // HYPER_RASP_WHITELIST should override the file
        assert_eq!(settings.whitelisted_filenames.len(), 2);
        assert!(settings.whitelisted_filenames.contains(&"from_env1.so".to_string()));
        assert!(settings.whitelisted_filenames.contains(&"from_env2.so".to_string()));
        
        // Clean up
        clear_env_vars();
    }
    
    #[test]
    fn test_missing_config_file() {
        let _guard = TEST_MUTEX.lock().unwrap();
        clear_env_vars(); // Clean start
        
        // Set path to non-existent file
        let dir = tempdir().unwrap();
        let non_existent = dir.path().join("nonexistent.yaml");
        std::env::set_var("HYPER_RASP_CONFIG", non_existent.to_str().unwrap());
        
        // Should still load with defaults (not fail)
        let settings = Settings::load().unwrap();
        
        assert_eq!(settings.audit_mode, false);
        assert_eq!(settings.whitelisted_filenames.len(), 0);
        
        // Clean up
        clear_env_vars();
    }
    
    #[test]
    fn test_whitelist_parsing() {
        let _guard = TEST_MUTEX.lock().unwrap();
        clear_env_vars(); // Clean start
        
        // Point to non-existent config so it doesn't load any file
        let dir = tempdir().unwrap();
        let non_existent = dir.path().join("nonexistent.yaml");
        std::env::set_var("HYPER_RASP_CONFIG", non_existent.to_str().unwrap());
        std::env::set_var("HYPER_RASP_WHITELIST", " lib1.so , lib2.so,  ,lib3.so ");
        
        let settings = Settings::load().unwrap();
        
        // Should parse correctly and ignore empty entries
        assert_eq!(settings.whitelisted_filenames.len(), 3);
        assert!(settings.whitelisted_filenames.contains(&"lib1.so".to_string()));
        assert!(settings.whitelisted_filenames.contains(&"lib2.so".to_string()));
        assert!(settings.whitelisted_filenames.contains(&"lib3.so".to_string()));
        
        // Clean up
        clear_env_vars();
    }
} 