use prometheus::{IntCounter, IntCounterVec, Opts, register_int_counter, register_int_counter_vec};
use std::sync::OnceLock;

static METRICS: OnceLock<Metrics> = OnceLock::new();

pub struct Metrics {
    pub blocks_total: IntCounter,
    pub audits_total: IntCounter,
    pub library_loads: IntCounterVec,
    pub unauthorized_loads: IntCounterVec,
}

impl Metrics {
    fn new() -> prometheus::Result<Self> {
        let blocks_total = register_int_counter!(
            Opts::new("hyper_processor_blocks_total", "Total number of blocked library loads")
        )?;
        
        let audits_total = register_int_counter!(
            Opts::new("hyper_processor_audits_total", "Total number of audited library loads")
        )?;
        
        let library_loads = register_int_counter_vec!(
            Opts::new("hyper_processor_library_loads", "Total library load attempts"),
            &["library", "status"]
        )?;
        
        let unauthorized_loads = register_int_counter_vec!(
            Opts::new("hyper_processor_unauthorized_loads", "Unauthorized library load attempts"),
            &["library", "action"]
        )?;
        
        Ok(Metrics {
            blocks_total,
            audits_total,
            library_loads,
            unauthorized_loads,
        })
    }
}

pub fn init() -> prometheus::Result<()> {
    let metrics = Metrics::new()?;
    METRICS.set(metrics).map_err(|_| {
        prometheus::Error::AlreadyReg
    })?;
    Ok(())
}

// Uncomment if needed to access metrics directly
// pub fn get() -> &'static Metrics {
//     METRICS.get().expect("Metrics not initialized")
// }

pub fn record_unauthorized_library(library_name: &str, audit_mode: bool) {
    if let Some(metrics) = METRICS.get() {
        if audit_mode {
            metrics.audits_total.inc();
            metrics.unauthorized_loads.with_label_values(&[library_name, "audit"]).inc();
        } else {
            metrics.blocks_total.inc();
            metrics.unauthorized_loads.with_label_values(&[library_name, "block"]).inc();
        }
        metrics.library_loads.with_label_values(&[library_name, "unauthorized"]).inc();
    }
}

pub fn record_authorized_library(library_name: &str) {
    if let Some(metrics) = METRICS.get() {
        metrics.library_loads.with_label_values(&[library_name, "authorized"]).inc();
    }
} 