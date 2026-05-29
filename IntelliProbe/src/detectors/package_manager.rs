//! Multi-distribution package manager detection

use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Apt,      // Debian, Ubuntu, Mint
    Dnf,      // Fedora, RHEL
    Yum,      // Older RHEL
    Pacman,   // Arch, Manjaro
    Zypper,   // openSUSE
    Apk,      // Alpine
    Unknown,
}

impl PackageManager {
    pub fn name(&self) -> &'static str {
        match self {
            PackageManager::Apt => "apt",
            PackageManager::Dnf => "dnf",
            PackageManager::Yum => "yum",
            PackageManager::Pacman => "pacman",
            PackageManager::Zypper => "zypper",
            PackageManager::Apk => "apk",
            PackageManager::Unknown => "unknown",
        }
    }
    
    pub fn install_command(&self, packages: &[&str]) -> String {
        let packages_str = packages.join(" ");
        match self {
            PackageManager::Apt => format!("sudo apt update && sudo apt install -y {}", packages_str),
            PackageManager::Dnf => format!("sudo dnf install -y {}", packages_str),
            PackageManager::Yum => format!("sudo yum install -y {}", packages_str),
            PackageManager::Pacman => format!("sudo pacman -S --noconfirm {}", packages_str),
            PackageManager::Zypper => format!("sudo zypper install -y {}", packages_str),
            PackageManager::Apk => format!("sudo apk add {}", packages_str),
            PackageManager::Unknown => format!("# Unknown package manager. Install manually: {}", packages_str),
        }
    }
    
    pub fn update_command(&self) -> String {
        match self {
            PackageManager::Apt => "sudo apt update".to_string(),
            PackageManager::Dnf => "sudo dnf check-update".to_string(),
            PackageManager::Yum => "sudo yum check-update".to_string(),
            PackageManager::Pacman => "sudo pacman -Sy".to_string(),
            PackageManager::Zypper => "sudo zypper refresh".to_string(),
            PackageManager::Apk => "sudo apk update".to_string(),
            PackageManager::Unknown => "# Unknown package manager".to_string(),
        }
    }
}

pub fn detect_package_manager() -> PackageManager {
    // Check by command existence
    if which::which("apt").is_ok() {
        return PackageManager::Apt;
    }
    if which::which("dnf").is_ok() {
        return PackageManager::Dnf;
    }
    if which::which("yum").is_ok() {
        return PackageManager::Yum;
    }
    if which::which("pacman").is_ok() {
        return PackageManager::Pacman;
    }
    if which::which("zypper").is_ok() {
        return PackageManager::Zypper;
    }
    if which::which("apk").is_ok() {
        return PackageManager::Apk;
    }
    
    // Fallback: check /etc/os-release
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        if content.contains("debian") || content.contains("ubuntu") {
            return PackageManager::Apt;
        }
        if content.contains("fedora") || content.contains("rhel") {
            return PackageManager::Dnf;
        }
        if content.contains("arch") {
            return PackageManager::Pacman;
        }
        if content.contains("opensuse") {
            return PackageManager::Zypper;
        }
        if content.contains("alpine") {
            return PackageManager::Apk;
        }
    }
    
    PackageManager::Unknown
}

pub fn get_distribution_name() -> String {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("PRETTY_NAME=") {
                return line.trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string();
            }
        }
    }
    "Unknown".to_string()
}