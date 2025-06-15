// Module for checking loaded libraries via /proc/self/maps

use std::collections::HashSet;
use std::path::Path;
use anyhow::{Result};
use crate::config::Settings; // Import Settings
use tracing::{debug, event, Level as TracingLevel}; // Removed warn, error as event! is used for them
use std::fs;
use sha2::{Sha256, Digest};
use std::io::Read;

// Minimal default system whitelist (Basenames or common versions)
// Users should add specific system/app libs to rasp_config.yaml
static DEFAULT_SYSTEM_WHITELIST: &[&str] = &[
    // Base essentials
    "libc.so.6",
    "ld-linux-x86-64.so.2", // Note: Arch specific!
    "libdl.so.2",
    "libm.so.6",
    "libpthread.so.0",
    "libgcc_s.so.1", // Common version suffix
    "librt.so.1",
    "libresolv.so.2",
    // SELinux/Security (common on RHEL/Fedora)
    "libselinux.so.1",
    "libcap.so.2.73",  // Capability library - version specific
    // Regex library (used by many tools like ls)
    "libpcre2-8.so.0.14.0", // Version specific
    // GCC runtime - additional version format
    "libgcc_s-15-20250521.so.1", // Fedora specific versioning
    // Maybe remove NSS libs from default? They load dynamically.
    // "libnss_files.so.2",   
    // "libnss_dns.so.2",
];

/// Gets file size and SHA256 hash of a library file
fn get_file_info(path: &Path) -> (u64, String) {
    let mut size = 0u64;
    let mut hash = String::from("<error>");
    
    if let Ok(metadata) = fs::metadata(path) {
        size = metadata.len();
    }
    
    if let Ok(mut file) = fs::File::open(path) {
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        
        loop {
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => hasher.update(&buffer[..n]),
                Err(_) => break,
            }
        }
        
        hash = format!("{:x}", hasher.finalize());
    }
    
    (size, hash)
}

/// Checks loaded libraries parsed from maps_content against a combined whitelist.
/// Returns Ok((found_unauthorized, audit_mode)) or Err on internal failure.
pub fn perform_check(settings: &Settings, maps_content: &str) -> Result<(bool, bool)> {
    debug!("[Check] Starting preload check...");
    let mut found_unauthorized = false;
    
    // Build the effective whitelist:
    // 1. Start with the hardcoded default system libraries.
    // 2. Add libraries specified in the config file.
    // 3. Always add our own library.
    let mut effective_whitelist: HashSet<String> = DEFAULT_SYSTEM_WHITELIST.iter()
                                                        .map(|s| s.to_string())
                                                        .collect();
    for filename in &settings.whitelisted_filenames {
        effective_whitelist.insert(filename.clone());
    }
    effective_whitelist.insert("libhyper_processor.so".to_string()); // Add self
    
    debug!("[Check] Effective Whitelist Filenames: {:?}", effective_whitelist);

    // Process the provided maps_content
    for line in maps_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Need at least 6 parts: address perms offset dev inode path
        if parts.len() >= 6 {
            let perms = parts[1];
            
            // Find the first part starting with '/', which should be the path
            let path_str_opt = parts.get(5..).and_then(|potential_paths| {
                potential_paths.iter().find(|&&p| p.starts_with('/')).copied()
            });

            if let Some(path_str) = path_str_opt {
                 let path = Path::new(path_str);
                
                 // Check for executable permission and if it's an absolute path
                 if perms.contains('x') && path.is_absolute() {
                     if let Some(filename_osstr) = path.file_name() {
                         if let Some(filename) = filename_osstr.to_str() {
                            // Check if the filename itself contains .so before proceeding
                            if filename.contains(".so") { 
                                // Record in learning mode
                                #[cfg(feature = "learning")]
                                {
                                    if settings.learning_mode {
                                        crate::learning::record_library(filename);
                                    }
                                }
                                
                                let is_whitelisted = effective_whitelist.contains(filename);
                                debug!(
                                    "[Check] Checking filename: '{}' from path '{}'. Whitelisted: {}",
                                    filename,
                                    path_str,
                                    is_whitelisted
                                );
                                if !is_whitelisted {
                                    let (file_size, file_hash) = get_file_info(path);
                                    
                                    // Record metrics
                                    #[cfg(feature = "metrics")]
                                    crate::metrics::record_unauthorized_library(filename, settings.audit_mode);
                                    
                                    if settings.audit_mode { 
                                        event!(TracingLevel::WARN,
                                            unauthorized_library_filename = filename,
                                            unauthorized_library_path = path_str,
                                            file_size = file_size,
                                            file_hash = file_hash.as_str(),
                                            alert_type = "AUDIT",
                                            "Unauthorized library detected (Audit Mode)"
                                        );
                                    } else { 
                                        event!(TracingLevel::ERROR,
                                            unauthorized_library_filename = filename,
                                            unauthorized_library_path = path_str,
                                            file_size = file_size,
                                            file_hash = file_hash.as_str(),
                                            alert_type = "SECURITY",
                                            "Unauthorized library detected (Blocking Mode)"
                                        );
                                    }
                                    found_unauthorized = true;
                                } else {
                                    // Record authorized library
                                    #[cfg(feature = "metrics")]
                                    crate::metrics::record_authorized_library(filename);
                                }
                            } // else: filename doesn't contain .so, ignore
                         } else { 
                             event!(TracingLevel::WARN, path_osstr = ?filename_osstr, "[Check] Filename from path is not valid UTF-8");
                         }
                     } else { 
                         event!(TracingLevel::WARN, path_str = path_str, "[Check] Could not extract filename from path component");
                     }
                 }
            } // else: Path not found or doesn't start with '/', ignore line for path check
        }
    }

    // Restore debug log for final state
    debug!(
        "[Check] Final check state: found_unauthorized = {}, audit_mode = {}",
        found_unauthorized,
        settings.audit_mode
    );

    // Return the findings and the audit mode status
    Ok((found_unauthorized, settings.audit_mode))
}

#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module
    use crate::config::Settings;

    // Helper to create settings for tests
    fn create_settings(audit_mode: bool, user_whitelist: Vec<&str>) -> Settings {
        Settings {
            audit_mode,
            whitelisted_filenames: user_whitelist.into_iter().map(String::from).collect(),
            learning_mode: false,
            learning_output: None,
            // system_whitelist is handled internally by perform_check using DEFAULT_SYSTEM_WHITELIST
        }
    }

    // Example /proc/self/maps content
    const MAPS_LEGIT_ONLY: &str = r#"
7f0000000000-7f1000000000 r-xp 00000000 fd:01 1234 /usr/lib64/ld-linux-x86-64.so.2
7f2000000000-7f3000000000 r-xp 00000000 fd:01 5678 /home/user/hyper_processor/target/release/libhyper_processor.so
7f4000000000-7f5000000000 r-xp 00000000 fd:01 9012 /usr/lib64/libc.so.6
7f6000000000-7f7000000000 r--p 001b1000 fd:01 9012 /usr/lib64/libc.so.6
7f8000000000-7f9000000000 r-xp 00000000 fd:01 3456 /usr/lib64/libpthread.so.0
7fa000000000-7fb000000000 ---p 00000000 00:00 0 
7fc000000000-7fd000000000 r-xp 00000000 fd:01 7890 /usr/lib64/libdl.so.2
    "#;

    const MAPS_WITH_UNAUTHORIZED: &str = r#"
7f0000000000-7f1000000000 r-xp 00000000 fd:01 1234 /usr/lib64/ld-linux-x86-64.so.2
7f2000000000-7f3000000000 r-xp 00000000 fd:01 5678 /home/user/hyper_processor/target/release/libhyper_processor.so
7f4000000000-7f5000000000 r-xp 00000000 fd:01 9012 /usr/lib64/libc.so.6
7f6000000000-7f7000000000 r-xp 00000000 fd:01 1122 /usr/lib64/libevil.so.1
7f8000000000-7f9000000000 r-xp 00000000 fd:01 3456 /usr/lib64/libpthread.so.0
    "#;
    
    const MAPS_WITH_NON_EXEC: &str = r#"
7f0000000000-7f1000000000 r--p 00000000 fd:01 1234 /usr/lib64/libnotexec.so.1
7f2000000000-7f3000000000 rw-p 00000000 fd:01 5678 /usr/lib64/libdataonly.so
7f4000000000-7f5000000000 r-xp 00000000 fd:01 9012 /usr/lib64/libc.so.6
    "#;

    const MAPS_MALFORMED: &str = r#"
7f0000000000-7f1000000000 r-xp path/missing
just some garbage line
7f4000000000-7f5000000000 r-xp 00000000 fd:01 9012 /usr/lib64/libc.so.6
    "#;

    #[test]
    fn test_all_whitelisted() {
        let settings = create_settings(false, vec![]);
        let result = perform_check(&settings, MAPS_LEGIT_ONLY);
        assert_eq!(result.unwrap(), (false, false));
    }

    #[test]
    fn test_unauthorized_block() {
        let settings = create_settings(false, vec![]); // Audit off
        let result = perform_check(&settings, MAPS_WITH_UNAUTHORIZED);
        assert_eq!(result.unwrap(), (true, false));
    }

    #[test]
    fn test_unauthorized_audit() {
        let settings = create_settings(true, vec![]); // Audit ON
        let result = perform_check(&settings, MAPS_WITH_UNAUTHORIZED);
        assert_eq!(result.unwrap(), (true, true));
    }

    #[test]
    fn test_user_whitelisted() {
        let settings = create_settings(false, vec!["libevil.so.1"]);
        let result = perform_check(&settings, MAPS_WITH_UNAUTHORIZED);
        assert_eq!(result.unwrap(), (false, false));
    }
    
    #[test]
    fn test_non_executable_ignored() {
        let settings = create_settings(false, vec![]);
        let result = perform_check(&settings, MAPS_WITH_NON_EXEC);
        assert_eq!(result.unwrap(), (false, false));
    }

    #[test]
    fn test_malformed_line_ignored() {
        let settings = create_settings(false, vec![]);
        let result = perform_check(&settings, MAPS_MALFORMED);
        assert_eq!(result.unwrap(), (false, false));
    }

     #[test]
    fn test_empty_maps() {
        let settings = create_settings(false, vec![]);
        let result = perform_check(&settings, "");
        assert_eq!(result.unwrap(), (false, false));
    }
    
    #[test]
    fn test_get_file_info() {
        use std::fs::File;
        use std::io::Write;
        use tempfile::tempdir;
        
        // Create a temporary directory and file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_lib.so");
        let test_content = b"Hello, world! This is a test library.";
        
        // Write test content to file
        let mut file = File::create(&file_path).unwrap();
        file.write_all(test_content).unwrap();
        file.sync_all().unwrap();
        
        // Test get_file_info
        let (size, hash) = get_file_info(&file_path);
        
        // Verify size
        assert_eq!(size, test_content.len() as u64);
        
        // Verify hash is not an error
        assert_ne!(hash, "<error>");
        
        // Calculate expected hash
        let mut hasher = Sha256::new();
        hasher.update(test_content);
        let expected_hash = format!("{:x}", hasher.finalize());
        
        assert_eq!(hash, expected_hash);
        
        // Test with non-existent file
        let (size2, hash2) = get_file_info(Path::new("/nonexistent/file.so"));
        assert_eq!(size2, 0);
        assert_eq!(hash2, "<error>");
    }
    
    #[test]
    fn test_maps_with_extra_whitespace() {
        let settings = create_settings(false, vec![]);
        let maps_content = "7f0000000000-7f1000000000  r-xp  00000000  fd:01  1234  /usr/lib64/libc.so.6\n";
        let result = perform_check(&settings, maps_content);
        assert_eq!(result.unwrap(), (false, false));
    }
    
    #[test]
    fn test_maps_with_comment_after_path() {
        let settings = create_settings(false, vec![]);
        // Similar to original test data with comment
        let maps_content = "7f0000000000-7f1000000000 r-xp 00000000 fd:01 1234 /usr/lib64/libevil.so.1  # comment\n";
        let result = perform_check(&settings, maps_content);
        assert_eq!(result.unwrap(), (true, false)); // Should still detect unauthorized lib
    }
    
    #[test]
    fn test_whitelist_with_version_specific_libs() {
        let settings = create_settings(false, vec!["libcustom-1.2.3.so"]);
        let maps_content = r#"
7f0000000000-7f1000000000 r-xp 00000000 fd:01 1234 /usr/lib64/libcustom-1.2.3.so
7f2000000000-7f3000000000 r-xp 00000000 fd:01 5678 /usr/lib64/libc.so.6
"#;
        let result = perform_check(&settings, maps_content);
        assert_eq!(result.unwrap(), (false, false)); // Should be whitelisted
    }
} 