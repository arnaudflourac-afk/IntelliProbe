//! Libraries detection probe

use super::{LibrariesInfo, Library, Crate, SystemLibrary, Probe};
use anyhow::Result;
use duct::cmd;
use std::path::Path;
use walkdir::WalkDir;
use serde_json::Value;

pub struct LibrariesProbe;

impl Probe for LibrariesProbe {
    fn name(&self) -> &'static str { "libraries" }
    
    fn run(&self) -> Result<Value> {
        let mut info = LibrariesInfo::default();
        
        // Detect Python libraries
        if which::which("pip3").is_ok() || which::which("pip").is_ok() {
            info.python = detect_python_libraries()?;
        }
        
        // Detect Node.js libraries
        if which::which("npm").is_ok() || which::which("yarn").is_ok() {
            info.node = detect_node_libraries()?;
        }
        
        // Detect Rust crates
        if which::which("cargo").is_ok() {
            info.rust = detect_rust_crates()?;
        }
        
        // Detect system libraries
        info.system = detect_system_libraries()?;
        
        Ok(serde_json::to_value(info)?)
    }
}

fn detect_python_libraries() -> Result<Vec<Library>> {
    let mut libraries = Vec::new();
    
    // Get list of installed packages
    let output = cmd("pip3", ["list", "--format=json"])
        .stderr_to_stdout()
        .read()?;
    
    if let Ok(packages) = serde_json::from_str::<Vec<PythonPackage>>(&output) {
        for pkg in packages {
            let category = categorize_python_library(&pkg.name);
            let description = get_python_library_description(&pkg.name);
            
            libraries.push(Library {
                name: pkg.name,
                version: pkg.version,
                description,
                category,
            });
        }
    }
    
    Ok(libraries)
}

fn detect_node_libraries() -> Result<Vec<Library>> {
    let mut libraries = Vec::new();
    
    // Try npm first, then yarn
    let output = if which::which("npm").is_ok() {
        cmd("npm", ["list", "--depth=0", "--json"])
            .stderr_to_stdout()
            .read()
            .ok()
    } else {
        None
    };
    
    if let Some(out) = output {
        if let Ok(json) = serde_json::from_str::<Value>(&out) {
            if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                for (name, info) in deps {
                    let version = info.get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    libraries.push(Library {
                        name: name.clone(),
                        version,
                        description: get_node_library_description(name),
                        category: categorize_node_library(name),
                    });
                }
            }
        }
    }
    
    Ok(libraries)
}

fn detect_rust_crates() -> Result<Vec<Crate>> {
    let mut crates = Vec::new();
    
    // Check for Cargo.toml in current or parent directories
    let manifest_path = find_cargo_manifest()?;
    
    if let Some(path) = manifest_path {
        let content = std::fs::read_to_string(path)?;
        // Parse dependencies from Cargo.toml (simplified)
        for line in content.lines() {
            if line.contains("=") && !line.starts_with('[') {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() >= 2 {
                    let name = parts[0].trim().to_string();
                    let version = parts[1].trim().trim_matches('"').to_string();
                    
                    crates.push(Crate {
                        name,
                        version,
                        description: get_rust_crate_description(&name),
                    });
                }
            }
        }
    }
    
    Ok(crates)
}

fn detect_system_libraries() -> Result<Vec<SystemLibrary>> {
    let mut libraries = Vec::new();
    let lib_paths = vec![
        "/usr/lib",
        "/usr/lib64",
        "/usr/local/lib",
        "/lib",
        "/lib64",
    ];
    
    let common_libs = vec![
        "libssl", "libcrypto", "libcurl", "libz", "libpng", "libjpeg",
        "libtiff", "libxml2", "libxslt", "libsqlite3", "libpq", "libmysqlclient",
    ];
    
    for lib_name in common_libs {
        for path in &lib_paths {
            let lib_path = Path::new(path);
            if lib_path.exists() {
                for entry in WalkDir::new(lib_path)
                    .max_depth(2)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let name = entry.file_name().to_string_lossy();
                    if name.contains(lib_name) && name.contains(".so") {
                        let version = extract_version(&name);
                        libraries.push(SystemLibrary {
                            name: lib_name.to_string(),
                            version,
                            path: entry.path().to_string_lossy().to_string(),
                        });
                        break;
                    }
                }
            }
        }
    }
    
    Ok(libraries)
}

fn categorize_python_library(name: &str) -> String {
    match name {
        n if n.contains("torch") => "Deep Learning".to_string(),
        n if n.contains("tensorflow") => "Deep Learning".to_string(),
        n if n.contains("sklearn") => "Machine Learning".to_string(),
        n if n.contains("transformers") => "NLP".to_string(),
        n if n.contains("opencv") => "Computer Vision".to_string(),
        n if n.contains("fastapi") || n.contains("django") || n.contains("flask") => "Web".to_string(),
        n if n.contains("pandas") || n.contains("numpy") => "Data Science".to_string(),
        n if n.contains("pytest") => "Testing".to_string(),
        _ => "Utility".to_string(),
    }
}

fn categorize_node_library(name: &str) -> String {
    match name {
        "express" | "koa" | "fastify" => "Web Framework".to_string(),
        "react" | "vue" | "angular" => "Frontend".to_string(),
        "jest" | "mocha" | "vitest" => "Testing".to_string(),
        "typescript" => "Language".to_string(),
        _ => "Utility".to_string(),
    }
}

fn get_python_library_description(name: &str) -> String {
    match name {
        "torch" => "PyTorch deep learning framework".to_string(),
        "tensorflow" => "TensorFlow machine learning platform".to_string(),
        "numpy" => "NumPy: array processing for numbers, strings, records".to_string(),
        "pandas" => "Pandas: data analysis and manipulation tool".to_string(),
        "scikit-learn" => "Simple and efficient tools for predictive data analysis".to_string(),
        "transformers" => "State-of-the-art Natural Language Processing".to_string(),
        "opencv-python" => "Computer Vision library".to_string(),
        _ => format!("{} library", name),
    }
}

fn get_node_library_description(name: &str) -> String {
    match name {
        "express" => "Fast, unopinionated, minimalist web framework".to_string(),
        "react" => "UI library for building component-based interfaces".to_string(),
        "typescript" => "Typed superset of JavaScript".to_string(),
        _ => format!("{} package", name),
    }
}

fn get_rust_crate_description(name: &str) -> String {
    match name {
        "serde" => "Serialization framework for Rust".to_string(),
        "tokio" => "Event-driven, non-blocking I/O platform".to_string(),
        "axum" => "Web framework focusing on ergonomics and modularity".to_string(),
        _ => format!("{} crate", name),
    }
}

fn extract_version(filename: &str) -> Option<String> {
    let re = regex::Regex::new(r"\.so\.(\d+(?:\.\d+)*)").unwrap();
    re.captures(filename)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

fn find_cargo_manifest() -> Result<Option<std::path::PathBuf>> {
    let mut current = std::env::current_dir()?;
    loop {
        let manifest = current.join("Cargo.toml");
        if manifest.exists() {
            return Ok(Some(manifest));
        }
        if !current.pop() {
            break;
        }
    }
    Ok(None)
}

#[derive(Debug, serde::Deserialize)]
struct PythonPackage {
    name: String,
    version: String,
}