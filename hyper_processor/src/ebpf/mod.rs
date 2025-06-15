use aya::{Bpf, programs::Lsm, programs::lsm::LsmLink, maps::HashMap as BpfHashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

pub struct EbpfMonitor {
    bpf: Arc<RwLock<Bpf>>,
    _links: Vec<LsmLink>,
}

impl EbpfMonitor {
    pub async fn new() -> Result<Self> {
        // Check if we can load eBPF programs
        if !Self::check_kernel_support()? {
            anyhow::bail!("Kernel doesn't support required eBPF features. Need kernel 5.7+ with BTF enabled.");
        }

        // Load the eBPF program
        let mut bpf = Bpf::load(aya::include_bytes_aligned!(
            "../../../target/bpfel-unknown-none/release/hyper_processor_ebpf"
        ))?;

        let mut links = Vec::new();

        // Attach to bprm_check_security LSM hook for exec monitoring
        if let Some(program) = bpf.program_mut("check_exec") {
            let lsm: &mut Lsm = program.try_into()?;
            lsm.load()?;
            links.push(lsm.attach()?);
        }

        // Attach to file_open LSM hook for library loading
        if let Some(program) = bpf.program_mut("check_file_open") {
            let lsm: &mut Lsm = program.try_into()?;
            lsm.load()?;
            links.push(lsm.attach()?);
        }

        Ok(Self {
            bpf: Arc::new(RwLock::new(bpf)),
            _links: links,
        })
    }

    /// Check if kernel supports required eBPF features
    fn check_kernel_support() -> Result<bool> {
        // Check kernel version using /proc/version
        let version_str = std::fs::read_to_string("/proc/version")?;
        
        // Extract version from string like "Linux version 6.14.8-300.fc42.x86_64"
        if let Some(version_part) = version_str.split_whitespace().nth(2) {
            let parts: Vec<&str> = version_part.split('.').collect();
            if parts.len() >= 2 {
                let major: u32 = parts[0].parse().unwrap_or(0);
                let minor: u32 = parts[1].parse().unwrap_or(0);
                
                // Need at least kernel 5.7 for LSM BPF
                if major < 5 || (major == 5 && minor < 7) {
                    return Ok(false);
                }
            }
        }

        // Check if BTF is available
        if !std::path::Path::new("/sys/kernel/btf/vmlinux").exists() {
            eprintln!("Warning: BTF not found. eBPF functionality may be limited.");
            return Ok(false);
        }

        Ok(true)
    }

    /// Get unauthorized library attempts from eBPF map
    pub async fn get_unauthorized_attempts(&self) -> Result<Vec<UnauthorizedAttempt>> {
        let bpf = self.bpf.read().await;
        let mut attempts = Vec::new();

        // Read from the unauthorized_libs map
        if let Some(map) = bpf.map("unauthorized_libs") {
            let unauthorized_map: &BpfHashMap<_, u64, UnauthorizedInfo> = map.try_into()?;
            
            for item in unauthorized_map.iter() {
                let (pid, info) = item?;
                attempts.push(UnauthorizedAttempt {
                    pid,
                    library_path: info.path_str(),
                    timestamp: info.timestamp,
                });
            }
        }

        Ok(attempts)
    }

    /// Clear the unauthorized attempts map
    pub async fn clear_attempts(&self) -> Result<()> {
        let mut bpf = self.bpf.write().await;
        
        if let Some(map) = bpf.map_mut("unauthorized_libs") {
            let unauthorized_map: &mut BpfHashMap<_, u64, UnauthorizedInfo> = map.try_into()?;
            
            // Clear all entries
            let keys: Vec<_> = unauthorized_map.keys().collect();
            for key in keys {
                let _ = unauthorized_map.remove(&key);
            }
        }

        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnauthorizedInfo {
    pub path: [u8; 256],
    pub timestamp: u64,
}

impl UnauthorizedInfo {
    fn path_str(&self) -> String {
        let len = self.path.iter().position(|&b| b == 0).unwrap_or(256);
        String::from_utf8_lossy(&self.path[..len]).to_string()
    }
}

pub struct UnauthorizedAttempt {
    pub pid: u64,
    pub library_path: String,
    pub timestamp: u64,
}
