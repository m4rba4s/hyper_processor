use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::process::{Command, ExitStatus};
use std::time::Duration;
use std::path::PathBuf;
use std::env;

#[derive(Parser)]
#[command(name = "hyper-processor")]
#[command(about = "HyperProcessor RASP CLI - Runtime Application Self-Protection", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Learn mode - collect libraries used by an application
    Learn {
        /// Duration to run in learning mode (e.g., "5m", "30s")
        #[arg(short, long, default_value = "30s")]
        duration: String,
        
        /// Output file for the whitelist
        #[arg(short, long, default_value = "learned_whitelist.yaml")]
        output: PathBuf,
        
        /// Command to run
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
    
    /// Monitor mode - start Prometheus metrics exporter
    Monitor {
        /// Address to bind the metrics server
        #[arg(short, long, default_value = "0.0.0.0:9100")]
        bind: String,
        
        /// Path to RASP config file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    
    /// Verify library signatures
    Verify {
        /// Library path to verify
        library: PathBuf,
        
        /// Check GPG signature
        #[arg(short, long)]
        gpg: bool,
        
        /// Expected SHA256 hash
        #[arg(short, long)]
        sha256: Option<String>,
    },
    
    /// Protect mode - run application with RASP protection
    Protect {
        /// Enable audit mode (log only, don't block)
        #[arg(short, long)]
        audit: bool,
        
        /// Path to RASP config file
        #[arg(short, long)]
        config: Option<PathBuf>,
        
        /// Additional libraries to whitelist
        #[arg(short, long)]
        whitelist: Vec<String>,
        
        /// Command to run
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
    
    /// Generate whitelist from audit logs
    Generate {
        /// Input log file (JSON format)
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output YAML file
        #[arg(short, long, default_value = "generated_whitelist.yaml")]
        output: PathBuf,
        
        /// Include system libraries
        #[arg(short, long)]
        system: bool,
    },
    
    /// eBPF kernel-level protection (requires root)
    #[cfg(feature = "ebpf")]
    Ebpf {
        /// Enable audit mode (log only, don't block)
        #[arg(short, long)]
        audit: bool,
        
        /// Path to whitelist file
        #[arg(short, long)]
        whitelist: Option<PathBuf>,
        
        /// Clear previous detections
        #[arg(short, long)]
        clear: bool,
        
        /// List detected attempts
        #[arg(short, long)]
        list: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Learn { duration, output, command } => {
            learn_mode(duration, output, command).await
        }
        Commands::Monitor { bind, config } => {
            monitor_mode(bind, config).await
        }
        Commands::Verify { library, gpg, sha256 } => {
            verify_library(library, gpg, sha256)
        }
        Commands::Protect { audit, config, whitelist, command } => {
            protect_mode(audit, config, whitelist, command)
        }
        Commands::Generate { input, output, system } => {
            generate_whitelist(input, output, system)
        }
        #[cfg(feature = "ebpf")]
        Commands::Ebpf { audit, whitelist, clear, list } => {
            ebpf_mode(audit, whitelist, clear, list).await
        }
    }
}

async fn learn_mode(duration_str: String, output: PathBuf, command: Vec<String>) -> Result<()> {
    println!("🎓 Starting learning mode for {}", duration_str);
    
    // Parse duration
    let duration = parse_duration(&duration_str)?;
    
    // Set up environment for learning mode
    let lib_path = find_rasp_library()?;
    let learning_output = tempfile::NamedTempFile::new()?;
    
    // Prepare environment
    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..])
        .env("LD_PRELOAD", &lib_path)
        .env("HYPER_RASP_AUDIT_MODE", "true")
        .env("HYPER_RASP_LEARNING_MODE", "true")
        .env("HYPER_RASP_LEARNING_OUTPUT", learning_output.path())
        .env("RUST_LOG", "warn");
    
    // Start the process
    println!("📚 Running: {}", command.join(" "));
    let mut child = cmd.spawn()
        .context("Failed to start application")?;
    
    // Run for specified duration
    tokio::select! {
        _ = tokio::time::sleep(duration) => {
            println!("⏰ Learning period complete, stopping application...");
            // Try graceful shutdown first
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                let _ = kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM);
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
            let _ = child.kill();
        }
        status = wait_for_child(&mut child) => {
            println!("✅ Application exited with: {:?}", status?);
        }
    }
    
    // Process collected data
    process_learning_data(learning_output.path(), &output)?;
    
    println!("📝 Whitelist saved to: {}", output.display());
    Ok(())
}

async fn monitor_mode(bind: String, config: Option<PathBuf>) -> Result<()> {
    #[cfg(feature = "metrics")]
    {
        use hyper::service::{make_service_fn, service_fn};
        use hyper::{Server};
        
        println!("📊 Starting Prometheus metrics exporter on {}", bind);
        
        if let Some(cfg) = config {
            env::set_var("HYPER_RASP_CONFIG", cfg);
        }
        
        // Initialize metrics (this will be done in the library)
        // For now, just serve the metrics endpoint
        
        let addr = bind.parse()
            .context("Invalid bind address")?;
        
        let make_svc = make_service_fn(|_conn| async {
            Ok::<_, hyper::Error>(service_fn(metrics_handler))
        });
        
        let server = Server::bind(&addr).serve(make_svc);
        
        println!("🚀 Metrics available at http://{}/metrics", bind);
        println!("Press Ctrl+C to stop");
        
        // Handle shutdown
        let graceful = server.with_graceful_shutdown(shutdown_signal());
        
        if let Err(e) = graceful.await {
            eprintln!("Server error: {}", e);
        }
        
        Ok(())
    }
    
    #[cfg(not(feature = "metrics"))]
    {
        eprintln!("Metrics support not compiled. Build with --features cli");
        std::process::exit(1);
    }
}

#[cfg(feature = "metrics")]
async fn metrics_handler(_req: hyper::Request<hyper::Body>) -> Result<hyper::Response<hyper::Body>, hyper::Error> {
    use prometheus::{TextEncoder, Encoder};
    use hyper::{Body, Response, StatusCode};
    
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap())
}

#[cfg(feature = "metrics")]
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
}

fn verify_library(library: PathBuf, gpg: bool, sha256: Option<String>) -> Result<()> {
    println!("🔍 Verifying library: {}", library.display());
    
    // Check if library exists
    if !library.exists() {
        anyhow::bail!("Library not found: {}", library.display());
    }
    
    // Calculate SHA256
    use sha2::{Sha256, Digest};
    use std::fs::File;
    use std::io::Read;
    
    let mut file = File::open(&library)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    
    let result = format!("{:x}", hasher.finalize());
    println!("📄 SHA256: {}", result);
    
    // Verify against expected hash
    if let Some(expected) = sha256 {
        if result == expected {
            println!("✅ Hash verification PASSED");
        } else {
            println!("❌ Hash verification FAILED");
            println!("   Expected: {}", expected);
            println!("   Got:      {}", result);
            std::process::exit(1);
        }
    }
    
    // GPG verification
    if gpg {
        println!("🔐 GPG signature verification not yet implemented");
        // TODO: Implement GPG verification
    }
    
    Ok(())
}

fn protect_mode(audit: bool, config: Option<PathBuf>, whitelist: Vec<String>, command: Vec<String>) -> Result<()> {
    println!("🛡️  Running with RASP protection");
    
    let lib_path = find_rasp_library()?;
    
    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..])
        .env("LD_PRELOAD", &lib_path);
    
    if audit {
        cmd.env("HYPER_RASP_AUDIT_MODE", "true");
        println!("📝 Audit mode enabled (non-blocking)");
    }
    
    if let Some(cfg) = config {
        cmd.env("HYPER_RASP_CONFIG", cfg);
    }
    
    if !whitelist.is_empty() {
        cmd.env("HYPER_RASP_WHITELIST", whitelist.join(","));
        println!("➕ Additional whitelist: {:?}", whitelist);
    }
    
    println!("🚀 Starting: {}", command.join(" "));
    
    let status = cmd.status()
        .context("Failed to execute command")?;
    
    std::process::exit(status.code().unwrap_or(1));
}

fn generate_whitelist(input: PathBuf, output: PathBuf, include_system: bool) -> Result<()> {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};
    use std::collections::HashSet;
    
    println!("🔧 Generating whitelist from: {}", input.display());
    
    let file = File::open(&input)
        .context("Failed to open input file")?;
    let reader = BufReader::new(file);
    
    let mut libraries = HashSet::new();
    
    for line in reader.lines() {
        let line = line?;
        
        // Skip comments and empty lines
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }
        
        // Try to parse as learning mode format first
        if line.contains(r#"{"library":"#) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(library) = json["library"].as_str() {
                    if include_system || !is_system_library(library) {
                        libraries.insert(library.to_string());
                    }
                }
            }
        } 
        // Try to parse as audit log format
        else if line.contains("unauthorized_library_filename") {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(filename) = json["fields"]["unauthorized_library_filename"].as_str() {
                    if include_system || !is_system_library(filename) {
                        libraries.insert(filename.to_string());
                    }
                }
            }
        }
    }
    
    // Get count before moving libraries
    let library_count = libraries.len();
    
    // Generate YAML
    let mut out = File::create(&output)?;
    writeln!(out, "# Generated whitelist from audit logs")?;
    writeln!(out, "# Generated at: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(out, "# Total libraries: {}", library_count)?;
    writeln!(out, "\naudit_mode: false\n")?;
    writeln!(out, "whitelisted_filenames:")?;
    
    let mut sorted: Vec<_> = libraries.into_iter().collect();
    sorted.sort();
    
    for lib in sorted {
        writeln!(out, "  - \"{}\"", lib)?;
    }
    
    println!("✅ Generated whitelist with {} libraries", library_count);
    println!("📝 Output saved to: {}", output.display());
    
    Ok(())
}

// Helper functions

fn find_rasp_library() -> Result<PathBuf> {
    // Check common locations
    let locations = [
        "./target/release/libhyper_processor.so",
        "./target/debug/libhyper_processor.so",
        "/usr/local/lib/libhyper_processor.so",
        "/usr/lib/libhyper_processor.so",
    ];
    
    for loc in &locations {
        let path = PathBuf::from(loc);
        if path.exists() {
            return Ok(path);
        }
    }
    
    // Check if specified via env
    if let Ok(path) = env::var("HYPER_PROCESSOR_LIB") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }
    
    anyhow::bail!("Cannot find libhyper_processor.so. Set HYPER_PROCESSOR_LIB or build with 'cargo build --release'")
}

fn parse_duration(s: &str) -> Result<Duration> {
    if let Some(num) = s.strip_suffix("s") {
        Ok(Duration::from_secs(num.parse()?))
    } else if let Some(num) = s.strip_suffix("m") {
        Ok(Duration::from_secs(num.parse::<u64>()? * 60))
    } else if let Some(num) = s.strip_suffix("h") {
        Ok(Duration::from_secs(num.parse::<u64>()? * 3600))
    } else {
        anyhow::bail!("Invalid duration format. Use format like '30s', '5m', or '1h'")
    }
}

async fn wait_for_child(child: &mut std::process::Child) -> Result<ExitStatus> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => tokio::time::sleep(Duration::from_millis(100)).await,
            Err(e) => return Err(e.into()),
        }
    }
}

fn process_learning_data(input: &std::path::Path, output: &PathBuf) -> Result<()> {
    // This will be implemented when we add learning mode to the library
    // For now, just process as regular audit log
    generate_whitelist(input.to_path_buf(), output.clone(), false)
}

fn is_system_library(name: &str) -> bool {
    name.starts_with("libc.") ||
    name.starts_with("libm.") ||
    name.starts_with("libdl.") ||
    name.starts_with("libpthread.") ||
    name.starts_with("librt.") ||
    name.starts_with("ld-linux") ||
    name.starts_with("libgcc_s.") ||
    name.starts_with("libresolv.")
}

#[cfg(feature = "ebpf")]
async fn ebpf_mode(audit: bool, whitelist: Option<PathBuf>, clear: bool, list: bool) -> Result<()> {
    use hyper_processor::ebpf::{EbpfMonitor};
    
    // Check if running as root
    if !nix::unistd::Uid::effective().is_root() {
        anyhow::bail!("eBPF mode requires root privileges. Please run with sudo.");
    }
    
    println!("🚀 Initializing eBPF kernel-level protection...");
    
    let monitor = EbpfMonitor::new().await
        .context("Failed to initialize eBPF monitor")?;
    
    // Handle clear command
    if clear {
        monitor.clear_attempts().await?;
        println!("✅ Cleared all previous detection records");
        return Ok(());
    }
    
    // Handle list command
    if list {
        let attempts = monitor.get_unauthorized_attempts().await?;
        if attempts.is_empty() {
            println!("No unauthorized library attempts detected.");
        } else {
            println!("🚨 Unauthorized Library Attempts:");
            println!("{:<10} {:<20} {}", "PID", "Timestamp", "Library Path");
            println!("{}", "-".repeat(60));
            
            for attempt in attempts {
                let timestamp = chrono::DateTime::<chrono::Utc>::from_timestamp(
                    (attempt.timestamp / 1_000_000_000) as i64, 
                    (attempt.timestamp % 1_000_000_000) as u32
                ).unwrap_or_default();
                
                println!("{:<10} {:<20} {}", 
                    attempt.pid, 
                    timestamp.format("%Y-%m-%d %H:%M:%S"),
                    attempt.library_path
                );
            }
        }
        return Ok(());
    }
    
    // Load whitelist if provided
    if let Some(whitelist_path) = whitelist {
        println!("📋 Loading whitelist from: {}", whitelist_path.display());
        // TODO: Implement whitelist loading into eBPF maps
        eprintln!("⚠️  Whitelist loading into eBPF maps not yet implemented");
    }
    
    if audit {
        println!("📝 Running in audit mode (non-blocking)");
    } else {
        println!("🛡️  Running in enforcement mode (blocking)");
    }
    
    println!("🔍 Monitoring kernel events... Press Ctrl+C to stop");
    
    // Monitor loop
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\n⏹️  Stopping eBPF monitor...");
                break;
            }
            _ = interval.tick() => {
                // Periodically check for new attempts
                match monitor.get_unauthorized_attempts().await {
                    Ok(attempts) => {
                        for attempt in attempts {
                            let timestamp = chrono::DateTime::<chrono::Utc>::from_timestamp(
                                (attempt.timestamp / 1_000_000_000) as i64,
                                (attempt.timestamp % 1_000_000_000) as u32
                            ).unwrap_or_default();
                            
                            println!("🚨 [{}] PID {} attempted to load: {}",
                                timestamp.format("%H:%M:%S"),
                                attempt.pid,
                                attempt.library_path
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading attempts: {}", e);
                    }
                }
            }
        }
    }
    
    Ok(())
} 