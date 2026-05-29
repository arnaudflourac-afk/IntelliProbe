//! GPU detection probe

use super::{GpuInfo, Probe};
use anyhow::Result;
use serde_json::Value;
use std::process::Command;
use std::path::Path;

pub struct GpuProbe;

impl Probe for GpuProbe {
    fn name(&self) -> &'static str {
        "gpu"
    }

    fn run(&self) -> Result<Value> {
        let (model, memory_mb, driver_version) = detect_gpu();
        let cuda_available = check_cuda();
        let opencl_available = check_opencl();
        let vulkan_available = check_vulkan();

        let info = GpuInfo {
            model,
            memory_mb,
            driver_version,
            cuda_available,
            opencl_available,
            vulkan_available,
        };

        Ok(serde_json::to_value(info)?)
    }
}

fn detect_gpu() -> (Option<String>, Option<u64>, Option<String>) {
    // Try NVIDIA first
    if let Ok(output) = Command::new("nvidia-smi")
        .args(["--query-gpu=name,memory.total,driver_version", "--format=csv,noheader"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = text.trim().split(',').collect();
        if parts.len() >= 3 {
            return (
                Some(parts[0].trim().to_string()),
                parts[1].trim().trim_end_matches(" MiB").parse::<u64>().ok(),
                Some(parts[2].trim().to_string()),
            );
        }
    }
    
    // Try AMD with ROCm
    if let Ok(output) = Command::new("rocm-smi")
        .args(["--showproductname", "--showmeminfo", "vram"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains("GPU") && line.contains("["){
                return (Some(line.to_string()), Some(8192), Some("ROCm".to_string()));
            }
        }
    }
    
    // Try Intel
    if let Ok(output) = Command::new("intel_gpu_top").output() {
        if output.status.success() {
            return (Some("Intel Integrated Graphics".to_string()), None, None);
        }
    }
    
    // Fallback: lspci
    if let Ok(output) = Command::new("lspci").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.contains("vga") || line_lower.contains("3d") || line_lower.contains("display") {
                let model = Some(line.to_string());
                // Try to detect VRAM from /proc/driver/nvidia/gpus/ or similar
                let memory = detect_vram_from_sysfs();
                return (model, memory, None);
            }
        }
    }
    
    (None, None, None)
}

fn detect_vram_from_sysfs() -> Option<u64> {
    // NVIDIA specific
    if let Ok(entries) = std::fs::read_dir("/proc/driver/nvidia/gpus") {
        for entry in entries.flatten() {
            let info_path = entry.path().join("information");
            if let Ok(content) = std::fs::read_to_string(info_path) {
                for line in content.lines() {
                    if line.contains("Video Memory") {
                        if let Some(mem) = line.split(':').nth(1) {
                            let mem_str = mem.trim().trim_end_matches(" MiB");
                            return mem_str.parse::<u64>().ok();
                        }
                    }
                }
            }
        }
    }
    None
}

fn check_cuda() -> bool {
    which::which("nvcc").is_ok() || Path::new("/usr/local/cuda").exists()
}

fn check_opencl() -> bool {
    which::which("clinfo").is_ok() || Path::new("/usr/lib/libOpenCL.so").exists()
}

fn check_vulkan() -> bool {
    which::which("vulkaninfo").is_ok() || Path::new("/usr/lib/libvulkan.so").exists()
}