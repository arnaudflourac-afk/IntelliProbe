//! Programming languages detection probe

use super::{LanguageInfo, Probe};
use anyhow::Result;
use serde_json::Value;
use std::process::Command;

pub struct LanguagesProbe;

impl Probe for LanguagesProbe {
    fn name(&self) -> &'static str {
        "languages"
    }

    fn run(&self) -> Result<Value> {
        let mut languages = Vec::new();
        
        // Rust
        languages.push(detect_language("rustc", &["--version"], "cargo", "Rust"));
        
        // Python
        languages.push(detect_language("python3", &["--version"], "pip3", "Python"));
        if !languages.last().unwrap().installed {
            languages.push(detect_language("python", &["--version"], "pip", "Python"));
        }
        
        // Node.js
        languages.push(detect_language("node", &["--version"], "npm", "Node.js"));
        
        // Go
        languages.push(detect_language("go", &["version"], "go", "Go"));
        
        // Java
        languages.push(detect_language("javac", &["-version"], "mvn", "Java"));
        
        // C# / .NET
        languages.push(detect_language("dotnet", &["--version"], "dotnet", "C#"));
        
        // Ruby
        languages.push(detect_language("ruby", &["--version"], "gem", "Ruby"));
        
        // PHP
        languages.push(detect_language("php", &["--version"], "composer", "PHP"));
        
        // Swift
        languages.push(detect_language("swift", &["--version"], "swift", "Swift"));
        
        // Kotlin
        languages.push(detect_language("kotlin", &["-version"], "gradle", "Kotlin"));
        
        Ok(serde_json::to_value(languages)?)
    }
}

fn detect_language(cmd: &str, args: &[&str], pkg_mgr: &str, display_name: &str) -> LanguageInfo {
    let installed = which::which(cmd).is_ok();
    let version = if installed {
        detect_version(cmd, args)
    } else {
        None
    };
    
    let package_manager = if installed && which::which(pkg_mgr).is_ok() {
        Some(pkg_mgr.to_string())
    } else {
        None
    };
    
    LanguageInfo {
        name: display_name.to_string(),
        version: version.unwrap_or_else(|| "unknown".to_string()),
        package_manager,
        installed,
    }
}

fn detect_version(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    
    // Try to extract version number using regex
    let re = regex::Regex::new(r"(\d+\.\d+\.\d+)").unwrap();
    if let Some(cap) = re.captures(&text) {
        return Some(cap[1].to_string());
    }
    
    // Fallback: first line
    text.lines().next().map(|l| l.trim().to_string())
}