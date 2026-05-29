//! Markdown exporter

use crate::analyze::AnalysisResult;
use anyhow::Result;
use colored::*;

pub fn export(result: &AnalysisResult) -> Result<String> {
    let report = &result.complete_report;
    let mut md = String::new();
    
    // Header
    md.push_str(&format!("# System Analysis Report\n\n"));
    md.push_str(&format!("**Generated:** {}\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    
    // Scores
    md.push_str("## 📊 Overall Scores\n\n");
    md.push_str(&format!("| Metric | Score | Tier |\n"));
    md.push_str(&format!("|--------|-------|------|\n"));
    md.push_str(&format!("| AI Score | {}/100 | {} |\n", 
        report.ai_summary.overall_score, report.ai_summary.tier));
    md.push_str(&format!("| Dev Score | {}/100 | {} |\n\n", 
        report.overall_dev_score, report.dev_tier));
    
    // Hardware
    md.push_str("## 💻 Hardware Specifications\n\n");
    md.push_str(&format!("- **CPU:** {} cores ({})\n", 
        report.ai_hardware.cpu_cores, 
        report.ai_hardware.cpu_model.as_deref().unwrap_or("Unknown")));
    md.push_str(&format!("- **RAM:** {:.1} GB\n", report.ai_hardware.ram_gb));
    md.push_str(&format!("- **GPU:** {} ({} MB VRAM)\n", 
        report.ai_hardware.gpu_model.as_deref().unwrap_or("None"),
        report.ai_hardware.gpu_memory_mb.unwrap_or(0)));
    md.push_str(&format!("- **Disk:** {:.0} GB free / {:.0} GB total\n\n", 
        report.ai_hardware.disk_free_gb, report.ai_hardware.disk_total_gb));
    
    // Languages
    md.push_str("## 🖥️ Programming Languages\n\n");
    md.push_str("| Language | Version | Package Manager |\n");
    md.push_str("|----------|---------|-----------------|\n");
    for lang in &report.dev_capabilities.languages {
        if lang.installed {
            md.push_str(&format!("| {} | {} | {} |\n", 
                lang.name, 
                lang.version.as_deref().unwrap_or("N/A"),
                lang.package_manager.as_deref().unwrap_or("N/A")));
        }
    }
    md.push_str("\n");
    
    // Containers
    md.push_str("## 🐳 Container Support\n\n");
    md.push_str(&format!("- **Docker:** {}\n", if report.dev_capabilities.containers.docker { "✅" } else { "❌" }));
    md.push_str(&format!("- **Docker Compose:** {}\n", if report.dev_capabilities.containers.compose_available { "✅" } else { "❌" }));
    md.push_str(&format!("- **Kubernetes CLI:** {}\n\n", if report.dev_capabilities.containers.kubernetes_cli { "✅" } else { "❌" }));
    
    // Recommendations
    md.push_str("## 🎯 Recommendations\n\n");
    md.push_str("### AI Workloads\n");
    for workload in &report.ai_recommendations.generative_ai {
        md.push_str(&format!("- {}\n", workload));
    }
    md.push_str("\n### Development Workloads\n");
    md.push_str("#### Web Development\n");
    for rec in &report.dev_recommendations.web_development {
        md.push_str(&format!("- {}\n", rec));
    }
    
    // Installation plan
    if !result.install_plan.is_empty() {
        md.push_str("\n## 📦 Installation Plan\n\n");
        md.push_str("```bash\n");
        for cmd in &result.install_plan {
            md.push_str(&format!("{}\n", cmd));
        }
        md.push_str("```\n");
    }
    
    Ok(md)
}