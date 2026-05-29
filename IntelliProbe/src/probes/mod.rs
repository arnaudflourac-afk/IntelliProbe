//! System probes module

mod cpu;
mod gpu;
mod memory;
mod languages;
mod libraries;
mod benchmarks;
mod platform;

pub use cpu::CpuProbe;
pub use gpu::GpuProbe;
pub use memory::MemoryProbe;
pub use languages::LanguagesProbe;
pub use libraries::LibrariesProbe;
pub use benchmarks::BenchmarksProbe;
pub use platform::PlatformProbe;

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Trait for system probes
pub trait Probe: Send + Sync {
    fn name(&self) -> &'static str;
    fn run(&self) -> Result<serde_json::Value>;
    fn is_available(&self) -> bool { true }
}

/// Complete system information from probes
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SystemInfo {
    pub platform: PlatformInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpu: GpuInfo,
    pub languages: Vec<LanguageInfo>,
    pub libraries: LibrariesInfo,
    pub benchmarks: Option<BenchmarkResults>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub kernel: String,
    pub hostname: String,
    pub architecture: String,
    pub package_manager: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model: String,
    pub cores: usize,
    pub threads: usize,
    pub frequency_mhz: u64,
    pub cache_size_kb: u64,
    pub virtualization: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_ram_gb: f64,
    pub free_ram_gb: f64,
    pub swap_gb: f64,
    pub total_disk_gb: f64,
    pub free_disk_gb: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GpuInfo {
    pub model: Option<String>,
    pub memory_mb: Option<u64>,
    pub driver_version: Option<String>,
    pub cuda_available: bool,
    pub opencl_available: bool,
    pub vulkan_available: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LanguageInfo {
    pub name: String,
    pub version: String,
    pub package_manager: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LibrariesInfo {
    pub python: Vec<Library>,
    pub node: Vec<Library>,
    pub rust: Vec<Crate>,
    pub system: Vec<SystemLibrary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Library {
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Crate {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemLibrary {
    pub name: String,
    pub version: Option<String>,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub cpu_score: u32,
    pub gpu_score: u32,
    pub memory_bandwidth_mbps: f64,
    pub disk_io_mbps: f64,
}

/// Run all available probes
pub fn run_all_probes(run_benchmarks: bool) -> Result<SystemInfo> {
    let mut system_info = SystemInfo::default();
    
    system_info.platform = PlatformProbe.run()?;
    system_info.cpu = CpuProbe.run()?;
    system_info.memory = MemoryProbe.run()?;
    system_info.gpu = GpuProbe.run()?;
    system_info.languages = LanguagesProbe.run()?;
    system_info.libraries = LibrariesProbe.run()?;
    
    if run_benchmarks {
        system_info.benchmarks = Some(BenchmarksProbe.run()?);
    }
    
    Ok(system_info)
}