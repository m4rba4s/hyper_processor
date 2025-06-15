/*!
 * HyperProcessor
 * 
 * A high-performance data processing library built with Rust, C, and Assembly optimizations.
 * Implements SOLID principles with a focus on efficiency and functional programming.
 */

use ctor::ctor;
// use log::{info, error, debug}; // REMOVED - Will use tracing macros directly
use crate::config::Settings;
use std::env;
use crate::preload_check::{perform_check};
use std::fs;
use std::process;
use tracing::{span, Level as TracingLevel, debug, info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt};
#[cfg(feature = "learning")]
use ctor::dtor;

// Main modules
pub mod config;
pub mod preload_check;

#[cfg(feature = "metrics")]
mod metrics;

#[cfg(feature = "learning")]
mod learning;

#[cfg(feature = "ebpf")]
pub mod ebpf;


/// Library constructor function called when the shared library is loaded.
#[ctor]
fn init_library() {
    // --- Get Process Info Early --- (These will be fields of the root span)
    let pid_val = process::id();
    
    let comm_result = fs::read_to_string("/proc/self/comm");
    let comm_val = comm_result
        .as_ref()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| String::from("<unknown>"));
    
    let cmdline_result = fs::read_to_string("/proc/self/cmdline");
    let cmdline_val = cmdline_result
        .as_ref()
        .map(|s| s.replace('\0', " ").trim().to_string())
        .unwrap_or_else(|_| String::from("<unknown>"));
    
    let ppid_val = nix::unistd::getppid().as_raw() as u32;
    
    let ld_preload_val = env::var("LD_PRELOAD").unwrap_or_else(|_| String::from("<not set>"));
    
    let version_val = env!("CARGO_PKG_VERSION");
    
    // Early logging fallback (before tracing is initialized)
    // We use a specific prefix to identify pre-logging errors
    let _log_prefix = format!("[{pid_val} {comm_val}]");

    // Initialize metrics if feature is enabled
    #[cfg(feature = "metrics")]
    {
        if let Err(e) = metrics::init() {
            eprintln!("{} [Init] Failed to initialize metrics: {}", _log_prefix, e);
        }
    }

    // Create a root span that will carry these fields for all log events within its scope.
    let root_span = span!(TracingLevel::INFO, "hyper_rasp_init", 
        pid = pid_val,
        ppid = ppid_val,
        process_name = comm_val.as_str(),
        ld_preload = ld_preload_val.as_str(),
        cmdline = cmdline_val.as_str(),
        version = version_val
    );
    let _enter = root_span.enter(); // Enter the span, fields will be attached to subsequent events

    // --- Load Configuration First ---
    let mut settings = match Settings::load() { 
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "[pid:{} ppid:{} process_name:'{}'] [HYPER_RASP PRE-LOGGING ERROR] Failed to load configuration: {}. Using default settings.", 
                pid_val, ppid_val, comm_val, e // Use pre-span values for pre-logging
            );
            Settings::default()
        }
    }; 

    // --- Override audit_mode from environment variable (highest priority) ---
    match env::var("HYPER_RASP_AUDIT_MODE") {
        Ok(val) => {
            let val_lower = val.to_lowercase();
            if val_lower == "true" || val_lower == "1" || val_lower == "yes" {
                 if !settings.audit_mode { 
                     // eprintln!("[CTOR] Audit Mode explicitly enabled via environment variable."); 
                 }
                settings.audit_mode = true;
            } else if val_lower == "false" || val_lower == "0" || val_lower == "no" {
                 if settings.audit_mode { 
                     // eprintln!("[CTOR] Audit Mode explicitly disabled via environment variable.");
                 }
                settings.audit_mode = false;
            }
        }
        Err(_) => {}
    }

    // Initialize learning mode if enabled
    #[cfg(feature = "learning")]
    {
        if settings.learning_mode {
            if let Some(ref output_path) = settings.learning_output {
                if let Err(e) = learning::init(output_path.clone()) {
                    eprintln!("{} [Init] Failed to initialize learning mode: {}", _log_prefix, e);
                }
                debug!("Learning mode initialized, output: {}", output_path);
            }
        }
    }

    // --- Initialize Logger ---
    // base_log_fields array is no longer needed as fields are in the root_span

    let log_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(if cfg!(debug_assertions) { "debug" } else { "info" }));

    let json_layer = fmt::layer()
        .json()
        .with_current_span(true) // Enable to see span fields in logs
        .with_span_list(true)   // Include span context in logs
        .with_target(true)
        .with_file(true)
        .with_line_number(true);
        
    // Initialize the global subscriber
    let subscriber = tracing_subscriber::registry()
        .with(log_filter)
        .with(json_layer);

    if subscriber.try_init().is_err() {
         eprintln!(
            "[pid:{} ppid:{} process_name:'{}'] [HYPER_RASP PRE-LOGGING ERROR] Failed to initialize global tracing subscriber.",
            pid_val, ppid_val, comm_val // Use pre-span values here
        );
    }
    
    // Log final status using the initialized logger
    info!(audit_mode = settings.audit_mode, "HyperProcessor RASP library loaded.");

    // --- Perform Check only if NOT running tests ---
    if !cfg!(test) {
        info!("Running preload check...");
        match std::fs::read_to_string("/proc/self/maps") {
            Ok(maps_content) => {
                debug!(maps_content = %maps_content, "Read /proc/self/maps content."); // Using key-value for potentially large content

                match perform_check(&settings, &maps_content) {
                    Ok((found_unauthorized, audit_mode_used)) => {
                        if found_unauthorized && !audit_mode_used {
                            error!("Terminating process due to unauthorized library detection.");
                            std::process::exit(1);
                        } else {
                             info!("Preload check completed.");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "FATAL: Preload check function failed internally. Terminating.");
                         std::process::exit(1); 
                    }
                }
            }
            Err(e) => {
                 error!(error = %e, "FATAL: Could not read /proc/self/maps. Terminating.");
                 std::process::exit(1); 
            }
        }
    } else {
         info!("[Test] Skipping preload check because running tests.");
    }
}

/// Library destructor - called when the library is unloaded
#[cfg(feature = "learning")]
#[dtor]
fn cleanup_library() {
    // Save learning data if learning mode was active
    learning::save_and_cleanup();
}
