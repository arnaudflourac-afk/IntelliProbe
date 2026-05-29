//! Platform detection probe

use super::{PlatformInfo, Probe};
use crate::detectors::package_manager::{detect_package_manager, get_distribution_name};
use anyhow::Result;
use serde_json::Value;
use std::process::Command;

pub struct PlatformProbe;

impl Probe for PlatformProbe {
    fn name(&self) -> &'static str {
        "platform"
    }

    fn run(&self) -> Result<Value> {
        let os = get_os_name();
        let kernel = get_kernel_version();
        let hostname = get_hostname();
        let architecture = get_architecture();
        let package_manager = detect_package_manager().name().to_string();
        
        let info = PlatformInfo {
            os,
            kernel,
            hostname,
            architecture,
            package_manager,
        };
        
        Ok(serde_json::to_value(info)?)
    }
}

fn get_os_name() -> String {
    // Try /etc/os-release first
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("PRETTY_NAME=") {
                return line.trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string();
            }
        }
    }
    
    // Fallback: uname
    if let Ok(output) = Command::new("uname").arg("-s").output() {
        let os = String::from_utf8_lossy(&output.stdout);
        return os.trim().to_string();
    }
    
    "Unknown".to_string()
}

fn get_kernel_version() -> String {
    if let Ok(output) = Command::new("uname").arg("-r").output() {
        let kernel = String::from_utf8_lossy(&output.stdout);
        return kernel.trim().to_string();
    }
    "Unknown".to_string()
}

fn get_hostname() -> String {
    if let Ok(output) = Command::new("hostname").output() {
        let hostname = String::from_utf8_lossy(&output.stdout);
        return hostname.trim().to_string();
    }
    "localhost".to_string()
}

fn get_architecture() -> String {
    if let Ok(output) = Command::new("uname").arg("-m").output() {
        let arch = String::from_utf8_lossy(&output.stdout);
        return arch.trim().to_string();
    }
    "unknown".to_string()
}