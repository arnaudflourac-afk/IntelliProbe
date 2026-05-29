//! Memory and disk detection probe

use super::{MemoryInfo, Probe};
use anyhow::Result;
use serde_json::Value;
use sysinfo::{System, Disks};

pub struct MemoryProbe;

impl Probe for MemoryProbe {
    fn name(&self) -> &'static str {
        "memory"
    }

    fn run(&self) -> Result<Value> {
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_disks_list();
        
        let total_ram_gb = system.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
        let free_ram_gb = system.free_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
        let swap_gb = system.total_swap() as f64 / (1024.0 * 1024.0 * 1024.0);
        
        let mut total_disk_gb = 0.0;
        let mut free_disk_gb = 0.0;
        
        let disks = Disks::new_with_refreshed_list();
        for disk in &disks {
            total_disk_gb += disk.total_space() as f64 / (1024.0 * 1024.0 * 1024.0);
            free_disk_gb += disk.available_space() as f64 / (1024.0 * 1024.0 * 1024.0);
        }
        
        let info = MemoryInfo {
            total_ram_gb,
            free_ram_gb,
            swap_gb,
            total_disk_gb,
            free_disk_gb,
        };
        
        Ok(serde_json::to_value(info)?)
    }
}