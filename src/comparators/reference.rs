//! Reference system comparisons

use crate::analyze::AnalysisResult;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReferenceConfig {
    pub name: String,
    pub description: String,
    pub min_ram_gb: f64,
    pub min_cpu_cores: usize,
    pub min_gpu_memory_mb: Option<u64>,
    pub required_languages: Vec<String>,
    pub required_tools: Vec<String>,
    pub workloads: Vec<String>,
}

pub fn load_reference(name: &str) -> Result<ReferenceConfig> {
    // Built-in reference configurations
    match name {
        "llm-7b" => Ok(ReferenceConfig {
            name: "LLaMA-2-7B".to_string(),
            description: "Run LLaMA-2-7B model locally".to_string(),
            min_ram_gb: 16.0,
            min_cpu_cores: 8,
            min_gpu_memory_mb: Some(8192),
            required_languages: vec!["Python".to_string()],
            required_tools: vec!["pip".to_string(), "git".to_string()],
            workloads: vec!["LLM inference".to_string(), "Text generation".to_string()],
        }),
        "llm-13b" => Ok(ReferenceConfig {
            name: "LLaMA-2-13B".to_string(),
            description: "Run LLaMA-2-13B model locally".to_string(),
            min_ram_gb: 32.0,
            min_cpu_cores: 12,
            min_gpu_memory_mb: Some(16384),
            required_languages: vec!["Python".to_string()],
            required_tools: vec!["pip".to_string(), "git".to_string()],
            workloads: vec!["LLM inference".to_string(), "Text generation".to_string(), "Fine-tuning".to_string()],
        }),
        "stable-diffusion" => Ok(ReferenceConfig {
            name: "Stable Diffusion".to_string(),
            description: "Run Stable Diffusion image generation".to_string(),
            min_ram_gb: 16.0,
            min_cpu_cores: 6,
            min_gpu_memory_mb: Some(6144),
            required_languages: vec!["Python".to_string()],
            required_tools: vec!["pip".to_string()],
            workloads: vec!["Image generation".to_string(), "Computer vision".to_string()],
        }),
        "web-dev" => Ok(ReferenceConfig {
            name: "Web Development".to_string(),
            description: "Full-stack web development environment".to_string(),
            min_ram_gb: 8.0,
            min_cpu_cores: 4,
            min_gpu_memory_mb: None,
            required_languages: vec!["Node.js".to_string(), "Python".to_string()],
            required_tools: vec!["git".to_string(), "docker".to_string(), "npm".to_string()],
            workloads: vec!["Web development".to_string(), "API development".to_string()],
        }),
        "data-engineering" => Ok(ReferenceConfig {
            name: "Data Engineering".to_string(),
            description: "Data processing and ETL pipelines".to_string(),
            min_ram_gb: 16.0,
            min_cpu_cores: 8,
            min_gpu_memory_mb: None,
            required_languages: vec!["Python".to_string()],
            required_tools: vec!["git".to_string(), "docker".to_string(), "pip".to_string()],
            workloads: vec!["Data processing".to_string(), "ETL pipelines".to_string()],
        }),
        _ => anyhow::bail!("Unknown reference: {}", name),
    }
}

pub fn compare_with_reference(result: &AnalysisResult, ref_name: &str) -> Result<String> {
    let reference = load_reference(ref_name)?;
    let mut comparison = String::new();
    
    comparison.push_str(&format!("\n📊 Comparison with: {}\n", reference.name));
    comparison.push_str(&format!("   {}\n\n", reference.description));
    
    let hw = &result.complete_report.ai_hardware;
    let mut meets_requirements = true;
    
    // Check RAM
    if hw.ram_gb >= reference.min_ram_gb {
        comparison.push_str(&format!("✅ RAM: {:.1} GB (required: {:.1} GB)\n", hw.ram_gb, reference.min_ram_gb));
    } else {
        comparison.push_str(&format!("❌ RAM: {:.1} GB (required: {:.1} GB)\n", hw.ram_gb, reference.min_ram_gb));
        meets_requirements = false;
    }
    
    // Check CPU cores
    if hw.cpu_cores >= reference.min_cpu_cores {
        comparison.push_str(&format!("✅ CPU: {} cores (required: {} cores)\n", hw.cpu_cores, reference.min_cpu_cores));
    } else {
        comparison.push_str(&format!("❌ CPU: {} cores (required: {} cores)\n", hw.cpu_cores, reference.min_cpu_cores));
        meets_requirements = false;
    }
    
    // Check GPU memory
    if let Some(req_vram) = reference.min_gpu_memory_mb {
        if let Some(vram) = hw.gpu_memory_mb {
            if vram >= req_vram {
                comparison.push_str(&format!("✅ GPU VRAM: {} MB (required: {} MB)\n", vram, req_vram));
            } else {
                comparison.push_str(&format!("⚠️ GPU VRAM: {} MB (required: {} MB)\n", vram, req_vram));
                meets_requirements = false;
            }
        } else {
            comparison.push_str(&format!("❌ GPU VRAM: None (required: {} MB)\n", req_vram));
            meets_requirements = false;
        }
    }
    
    // Check languages
    comparison.push_str("\n📝 Required languages:\n");
    for req_lang in &reference.required_languages {
        let found = result.complete_report.dev_capabilities.languages
            .iter()
            .any(|l| l.name == *req_lang && l.installed);
        
        if found {
            comparison.push_str(&format!("✅ {}\n", req_lang));
        } else {
            comparison.push_str(&format!("❌ {} (not installed)\n", req_lang));
            meets_requirements = false;
        }
    }
    
    // Check tools
    comparison.push_str("\n🔧 Required tools:\n");
    let dev = &result.complete_report.dev_capabilities;
    
    for req_tool in &reference.required_tools {
        let found = match req_tool.as_str() {
            "docker" => dev.containers.docker,
            "git" => dev.version_control.git,
            "npm" => result.complete_report.dev_capabilities.languages
                .iter()
                .any(|l| l.name == "Node.js" && l.package_manager.is_some()),
            "pip" => result.complete_report.dev_capabilities.languages
                .iter()
                .any(|l| l.name == "Python" && l.package_manager.is_some()),
            _ => false,
        };
        
        if found {
            comparison.push_str(&format!("✅ {}\n", req_tool));
        } else {
            comparison.push_str(&format!("❌ {} (not available)\n", req_tool));
            meets_requirements = false;
        }
    }
    
    comparison.push_str("\n");
    if meets_requirements {
        comparison.push_str(&format!("✅ **This system meets the requirements for {}**\n", reference.name));
        comparison.push_str(&format!("   Recommended workloads: {}\n", reference.workloads.join(", ")));
    } else {
        comparison.push_str(&format!("❌ **This system does NOT meet the requirements for {}**\n", reference.name));
        comparison.push_str("   Consider upgrading the missing components.\n");
    }
    
    Ok(comparison)
}