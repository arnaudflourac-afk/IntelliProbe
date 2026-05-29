use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use intelli_probe::analyze::{self, AnalysisResult};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "IntelliProbe")]
#[command(about = "Intelligent Workstation Profiler for AI & Development", long_about = None)]
struct Cli {
    /// Input JSON file (skip probing)
    #[arg(short, long)]
    input: Option<String>,
    
    /// Start web dashboard
    #[arg(long)]
    dashboard: bool,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    print_banner();

    // --- Analyse système ---
    let result = if let Some(input_path) = cli.input {
        let data = fs::read_to_string(input_path)?;
        let report: analyze::SystemReport = serde_json::from_str(&data)?;
        analyze::analyze_system_from_report(report)?
    } else {
        analyze::analyze_system()?
    };

    // --- Création du dossier output/ ---
    fs::create_dir_all("output")?;

    // --- Chemins des fichiers ---
    let system_path = PathBuf::from("output").join("system_capabilities.json");
    let analysis_path = PathBuf::from("output").join("analysis_result.json");

    // --- Écriture dans output/ ---
    fs::write(system_path, serde_json::to_string_pretty(&result)?)?;
    fs::write(analysis_path, serde_json::to_string_pretty(&result.complete_report)?)?;

    // --- Affichage terminal ---
    print_results(&result);

    // --- Dashboard ---
    if cli.dashboard {
        println!("\n{}", "🌐 Starting dashboard...".green());
        #[cfg(feature = "web")]
        intelli_probe::web::start_dashboard().await?;
        #[cfg(not(feature = "web"))]
        println!("Dashboard feature not enabled. Recompile with --features web");
    }

    Ok(())
}

fn print_banner() {
    use colored::*;

    println!("{}", "╔══════════════════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║                                                                      ║".bright_cyan());
    println!("{}", "║                       Intelli Probe v1.0                             ║".bright_cyan().bold());
    println!("{}", "║            Intelligent Workstation Profiler & Analyzer               ║".bright_cyan());
    println!("{}", "║                                                                      ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════════════╝".bright_cyan());
}


fn print_results(result: &AnalysisResult) {
    println!("\n{}", "═".repeat(60).green());
    println!("{}", "📊 SYSTEM ANALYSIS COMPLETE".bold().green());
    println!("{}", "═".repeat(60).green());
    
    println!("\n🤖 AI SCORE: {}/100 ({})", 
        result.complete_report.ai_summary.overall_score,
        result.complete_report.ai_summary.tier);
    
    println!("🛠️  DEV SCORE: {}/100 ({})", 
        result.complete_report.overall_dev_score,
        result.complete_report.dev_tier);
    
    println!("\n🎯 BEST FOR AI:");
    for b in &result.complete_report.ai_summary.best_for {
        println!("  • {}", b);
    }
    
    println!("\n💻 DETECTED LANGUAGES:");
    for lang in &result.complete_report.dev_capabilities.languages {
        if lang.installed {
            println!("  • {} ({})", lang.name, lang.version.as_deref().unwrap_or("unknown"));
        }
    }
    
    if !result.install_plan.is_empty() {
        println!("\n📦 INSTALLATION PLAN:");
        for cmd in &result.install_plan {
            println!("  $ {}", cmd);
        }
    }
}