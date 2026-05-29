//! CPU detection probe

use super::{CpuInfo, Probe};
use anyhow::Result;
use serde_json::Value;
use std::fs;

pub struct CpuProbe;

impl Probe for CpuProbe {
    fn name(&self) -> &'static str {
        "cpu"
    }

    fn run(&self) -> Result<Value> {
        let model = get_cpu_model();
        let cores = num_cpus::get();
        let threads = num_cpus::get_physical();
        let frequency_mhz = get_cpu_frequency();
        let cache_size_kb = get_cache_size();
        let virtualization = check_virtualization_support();

        let info = CpuInfo {
            model,
            cores,
            threads,
            frequency_mhz,
            cache_size_kb,
            virtualization,
        };

        Ok(serde_json::to_value(info)?)
    }
}

fn get_cpu_model() -> String {
    // Try /proc/cpuinfo first (Linux)
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("model name") || line.starts_with("Processor") {
                if let Some(model) = line.split(':').nth(1) {
                    return model.trim().to_string();
                }
            }
        }
    }
    
    // Fallback: use sysinfo crate
    let mut system = sysinfo::System::new();
    system.refresh_cpu_all();
    if let Some(cpu) = system.cpus().first() {
        return cpu.brand().to_string();
    }
    
    "Unknown CPU".to_string()
}

fn get_cpu_frequency() -> u64 {
    // Try /proc/cpuinfo
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("cpu MHz") || line.starts_with("cpu clock") {
                if let Some(freq) = line.split(':').nth(1) {
                    if let Ok(f) = freq.trim().parse::<f64>() {
                        return f as u64;
                    }
                }
            }
        }
    }
    
    // Fallback: sysinfo
    let mut system = sysinfo::System::new();
    system.refresh_cpu_all();
    system.cpus().first().map(|c| c.frequency()).unwrap_or(0)
}

fn get_cache_size() -> u64 {
    let mut total_cache = 0;
    
    // Check L1, L2, L3 cache from sysfs
    for cache_level in ["L1", "L2", "L3"] {
        let path = format!("/sys/devices/system/cpu/cpu0/cache/index{}", 
            match cache_level {
                "L1" => "0",
                "L2" => "1",
                "L3" => "2",
                _ => "0",
            }
        );
        
        if let Ok(size_str) = fs::read_to_string(format!("{}/size", path)) {
            let size = size_str.trim().to_string();
            if let Ok(num) = size.trim_end_matches('K').parse::<u64>() {
                total_cache += num;
            }
        }
    }
    
    total_cache
}

fn check_virtualization_support() -> bool {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.contains("vmx") || line.contains("svm") {
                return true;
            }
        }
    }
    false
}