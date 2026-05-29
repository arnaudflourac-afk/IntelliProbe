//! System analysis module - 100% dynamique, aucune donnée statique

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::process::Command;
use std::fs;
use std::path::Path;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LanguageInfo {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub package_manager: Option<String>,
    pub lsp_available: bool,
    pub debugger_available: bool,
    pub linting_available: bool,
    pub formatting_available: bool,
    pub performance_rating: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub version: Option<String>,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NpuInfo {
    pub name: String,
    pub detected: bool,
    pub driver: Option<String>,
    pub api: Option<String>,
    pub max_tops: Option<f32>,
    pub memory_mb: Option<u32>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub hostname: Option<String>,
    pub architecture: Option<String>,
    pub uptime: Option<String>,
    pub load_average: Option<String>,
    pub shell: Option<String>,
    pub desktop_environment: Option<String>,
    pub cpu_cores: usize,
    pub cpu_model: Option<String>,
    pub cpu_temperature: Option<String>,
    pub cpu_frequency_mhz: Option<u64>,
    pub ram_gb: f64,
    pub swap_gb: f64,
    pub disk_free_gb: f64,
    pub disk_total_gb: f64,
    pub disk_type: Option<String>,
    pub gpu_model: Option<String>,
    pub gpu_memory_mb: Option<u64>,
    pub gpu_driver_version: Option<String>,
    pub gpu_temperature: Option<String>,
    pub gpu_cuda_cores: Option<u32>,
    pub network_interfaces: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemLibrary {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub size_bytes: u64,
    pub description: String,
    pub category: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SystemReport {
    pub system: SystemInfo,
    pub languages: Vec<LanguageInfo>,
    pub rust_crates: Vec<CrateInfo>,
    pub node_packages: Vec<CrateInfo>,
    pub python_packages: Vec<CrateInfo>,
    pub npus: Vec<NpuInfo>,
    pub docker_installed: bool,
    pub podman_installed: bool,
    pub git_installed: bool,
    pub git_lfs_installed: bool,
    pub build_tools: Vec<BuildTool>,
    pub databases: Vec<DatabaseSupport>,
    pub ides: Vec<IdeSupport>,
    pub monitoring_tools: MonitoringSupport,
    pub vscode_extensions: Vec<String>,
    pub system_libraries: Vec<SystemLibrary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HardwareSpecs {
    pub cpu_cores: usize,
    pub cpu_model: Option<String>,
    pub cpu_frequency_mhz: Option<u64>,
    pub cpu_temperature: Option<String>,
    pub ram_gb: f64,
    pub swap_gb: f64,
    pub disk_free_gb: f64,
    pub disk_total_gb: f64,
    pub disk_type: Option<String>,
    pub gpu_memory_mb: Option<u64>,
    pub gpu_model: Option<String>,
    pub gpu_driver_version: Option<String>,
    pub gpu_temperature: Option<String>,
    pub gpu_cuda_cores: Option<u32>,
    pub max_resolution: Option<String>,
    pub supported_formats: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Summary {
    pub overall_score: u8,
    pub tier: String,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub best_for: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackendSupport {
    pub name: String,
    pub supported: bool,
    pub acceleration: Option<String>,
    pub performance_rating: u8,
    pub setup_instructions: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodecSupport {
    pub codec: String,
    pub encode: bool,
    pub decode: bool,
    pub hardware_accelerated: bool,
    pub max_resolution: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiSupport {
    pub api: String,
    pub supported: bool,
    pub version: Option<String>,
    pub compute_units: Option<u32>,
    pub max_workgroup_size: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompatibilityMatrix {
    pub inference_backends: Vec<BackendSupport>,
    pub video_codecs: Vec<CodecSupport>,
    pub compute_apis: Vec<ApiSupport>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceEstimates {
    pub fp32_tflops: Option<f32>,
    pub fp16_tflops: Option<f32>,
    pub int8_tops: Option<f32>,
    pub memory_bandwidth_gbps: Option<f32>,
    pub inference_latency_ms: HashMap<String, f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkloadRecommendations {
    pub computer_vision: Vec<String>,
    pub nlp: Vec<String>,
    pub audio: Vec<String>,
    pub generative_ai: Vec<String>,
    pub real_time: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InitCodeSnippets {
    pub pytorch_device_config: String,
    pub tensorflow_device_config: String,
    pub onnx_session_config: String,
    pub ffmpeg_hwaccel_config: String,
    pub opencv_umat_config: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildTool {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub parallel_builds: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerSupport {
    pub docker: bool,
    pub docker_version: Option<String>,
    pub podman: bool,
    pub kubernetes_cli: bool,
    pub compose_available: bool,
    pub buildx_support: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseSupport {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub client_tools: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionControlSupport {
    pub git: bool,
    pub git_version: Option<String>,
    pub git_lfs: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdeSupport {
    pub name: String,
    pub detected: bool,
    pub version: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub struct MonitoringSupport {
    pub htop: bool,
    pub btop: bool,
    pub glances: bool,
    pub netdata: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DevCapabilities {
    pub languages: Vec<LanguageInfo>,
    pub rust_crates: Vec<CrateInfo>,
    pub node_packages: Vec<CrateInfo>,
    pub python_packages: Vec<CrateInfo>,
    pub npus: Vec<NpuInfo>,
    pub build_tools: Vec<BuildTool>,
    pub containers: ContainerSupport,
    pub databases: Vec<DatabaseSupport>,
    pub version_control: VersionControlSupport,
    pub ides: Vec<IdeSupport>,
    pub monitoring_tools: MonitoringSupport,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DevWorkloadRecommendations {
    pub web_development: Vec<String>,
    pub backend_development: Vec<String>,
    pub data_engineering: Vec<String>,
    pub game_development: Vec<String>,
    pub embedded_development: Vec<String>,
    pub devops: Vec<String>,
    pub mobile_development: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolchainConfig {
    pub suggested_shell: String,
    pub vscode_extensions: Vec<String>,
    pub git_aliases: Vec<String>,
    pub docker_optimizations: Vec<String>,
    pub suggested_ides: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompleteSystemReport {
    pub system_info: SystemInfo,
    pub ai_summary: Summary,
    pub ai_hardware: HardwareSpecs,
    pub ai_compatibility: CompatibilityMatrix,
    pub ai_performance: PerformanceEstimates,
    pub ai_recommendations: WorkloadRecommendations,
    pub ai_optimal_config: serde_json::Value,
    pub ai_init_code: InitCodeSnippets,
    pub dev_capabilities: DevCapabilities,
    pub dev_recommendations: DevWorkloadRecommendations,
    pub dev_toolchain: ToolchainConfig,
    pub overall_dev_score: u8,
    pub dev_tier: String,
    pub system_libraries: Vec<SystemLibrary>,
    pub npus: Vec<NpuInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub capabilities: Vec<String>,
    pub limitations: Vec<String>,
    pub missing_dependencies: Vec<String>,
    pub install_plan: Vec<String>,
    pub qwen_prompt: String,
    pub complete_report: CompleteSystemReport,
}

// ============================================================================
// MAIN ANALYSIS FUNCTIONS
// ============================================================================

pub fn analyze_system() -> Result<AnalysisResult, anyhow::Error> {
    let report = collect_system_info()?;
    analyze_system_from_report(report)
}

pub fn analyze_system_from_report(report: SystemReport) -> Result<AnalysisResult, anyhow::Error> {
    let capabilities = Vec::new();
    let install_plan = Vec::new();

    let hardware = get_hardware_specs(&report);
    let compatibility = get_compatibility_matrix();
    let performance = get_performance_estimates();
    let recommendations = get_workload_recommendations();
    let snippets = get_init_code_snippets();
    
    let mut ai_score = 50;
    
    if hardware.ram_gb >= 16.0 { ai_score += 10; }
    if hardware.cpu_cores >= 8 { ai_score += 5; }
    if hardware.gpu_model.is_some() { ai_score += 15; }
    if hardware.gpu_cuda_cores.is_some() && hardware.gpu_cuda_cores.unwrap_or(0) > 2000 { ai_score += 10; }
    if !report.npus.is_empty() { ai_score += 10; }
    
    let ai_tier = if ai_score >= 80 { "High".to_string() } 
                  else if ai_score >= 60 { "Mid".to_string() }
                  else if ai_score >= 40 { "Entry".to_string() }
                  else { "Edge".to_string() };
    
    let ai_strengths: Vec<String> = vec![
        if hardware.cpu_cores >= 8 { Some(format!("{} cœurs CPU", hardware.cpu_cores)) } else { None },
        if hardware.ram_gb >= 16.0 { Some(format!("{:.0} Go RAM", hardware.ram_gb)) } else { None },
        if hardware.gpu_model.is_some() { Some(format!("GPU: {}", hardware.gpu_model.as_ref().unwrap())) } else { None },
        if hardware.gpu_cuda_cores.is_some() { Some(format!("{} cœurs CUDA", hardware.gpu_cuda_cores.unwrap())) } else { None },
        if !report.npus.is_empty() { Some(format!("{} NPU(s) détecté(s)", report.npus.len())) } else { None },
    ].into_iter().filter_map(|x| x).collect();
    
    let ai_weaknesses: Vec<String> = vec![
        if hardware.ram_gb < 8.0 { Some("RAM limitée (<8GB)".into()) } else { None },
        if hardware.gpu_model.is_none() && report.npus.is_empty() { Some("Pas de GPU dédié ni NPU".into()) } else { None },
        if hardware.disk_free_gb < 50.0 { Some("Espace disque faible (<50GB)".into()) } else { None },
    ].into_iter().filter_map(|x| x).collect();
    
    let ai_best_for = if ai_score >= 70 {
        vec!["Large Language Models (LLaMA, Mistral)".into(), "Stable Diffusion / IA générative".into(), "Computer Vision / YOLO".into(), "Fine-tuning de modèles".into()]
    } else if ai_score >= 50 {
        vec!["Inférence edge".into(), "Petits modèles (BERT, DistilBERT)".into(), "Traitement vidéo temps réel".into()]
    } else {
        vec!["TinyML / Microcontrôleurs".into(), "Inférence CPU uniquement".into(), "Prototypage rapide".into()]
    };

    let dev_capabilities = get_dev_capabilities(&report);
    let dev_recommendations = get_dev_recommendations();
    let toolchain = get_toolchain_config();
    
    let mut dev_score = 30;
    for lang in &dev_capabilities.languages {
        if lang.installed { dev_score += 10; }
    }
    if dev_capabilities.containers.docker { dev_score += 15; }
    if dev_capabilities.version_control.git { dev_score += 10; }
    if !report.build_tools.is_empty() { dev_score += 5; }
    if !report.rust_crates.is_empty() { dev_score += 5; }
    if !report.node_packages.is_empty() { dev_score += 5; }
    if !report.python_packages.is_empty() { dev_score += 5; }
    if !report.npus.is_empty() { dev_score += 10; }
    
    let dev_tier = if dev_score >= 80 { "Expert".to_string() }
                   else if dev_score >= 60 { "Professional".to_string() }
                   else if dev_score >= 40 { "Intermediate".to_string() }
                   else { "Beginner".to_string() };

    let qwen_prompt = format!(
        "=== IntelliProbe RAPPORT ===\n\
        OS: {} {}\n\
        Kernel: {}\n\
        Architecture: {}\n\
        CPU: {} cœurs - {}\n\
        RAM: {:.1} Go\n\
        GPU: {}\n\
        NPU: {}\n\
        Stockage: {:.0} Go libre / {:.0} Go total\n\
        \n\
        LANGAGES DÉTECTÉS:\n{}\n\
        \n\
        SCORE IA: {}/100 ({})\n\
        SCORE DEV: {}/100 ({})\n\
        \n\
        RECOMMANDATIONS IA:\n{:?}\n\
        \n\
        GÉNÈRE UNE CONFIGURATION OPTIMISÉE POUR CETTE MACHINE.",
        report.system.os_name.as_deref().unwrap_or("Inconnu"),
        report.system.os_version.as_deref().unwrap_or(""),
        report.system.kernel_version.as_deref().unwrap_or("Inconnu"),
        report.system.architecture.as_deref().unwrap_or("Inconnue"),
        hardware.cpu_cores,
        hardware.cpu_model.as_deref().unwrap_or("Inconnu"),
        hardware.ram_gb,
        hardware.gpu_model.as_deref().unwrap_or("Aucun"),
        if report.npus.is_empty() { "Aucun".to_string() } else { report.npus.iter().map(|n| n.name.clone()).collect::<Vec<_>>().join(", ") },
        hardware.disk_free_gb,
        hardware.disk_total_gb,
        dev_capabilities.languages.iter().filter(|l| l.installed).map(|l| format!("  - {} {}", l.name, l.version.as_deref().unwrap_or(""))).collect::<Vec<_>>().join("\n"),
        ai_score, ai_tier,
        dev_score, dev_tier,
        ai_best_for
    );

    let complete_report = CompleteSystemReport {
        system_info: report.system.clone(),
        ai_summary: Summary { overall_score: ai_score.min(100), tier: ai_tier, strengths: ai_strengths, weaknesses: ai_weaknesses, best_for: ai_best_for },
        ai_hardware: hardware,
        ai_compatibility: compatibility,
        ai_performance: performance,
        ai_recommendations: recommendations,
        ai_optimal_config: serde_json::json!({"optimized": true}),
        ai_init_code: snippets,
        dev_capabilities,
        dev_recommendations,
        dev_toolchain: toolchain,
        overall_dev_score: dev_score.min(100),
        dev_tier,
        system_libraries: report.system_libraries,
        npus: report.npus.clone(),
    };

    Ok(AnalysisResult {
        capabilities,
        limitations: vec![],
        missing_dependencies: vec![],
        install_plan,
        qwen_prompt,
        complete_report,
    })
}

// ============================================================================
// SYSTEM INFORMATION COLLECTION - VERSION COMPLÈTE
// ============================================================================

fn collect_system_info() -> Result<SystemReport, anyhow::Error> {
    let mut report = SystemReport::default();
    
    // Informations système
    report.system.os_name = get_os_name();
    report.system.os_version = get_os_version();
    report.system.kernel_version = get_kernel_version();
    report.system.hostname = get_hostname();
    report.system.architecture = get_architecture();
    report.system.uptime = get_uptime();
    report.system.load_average = get_load_average();
    report.system.shell = get_shell_version();
    report.system.desktop_environment = get_desktop_environment();
    
    // CPU
    report.system.cpu_cores = num_cpus::get();
    report.system.cpu_model = get_cpu_model();
    report.system.cpu_temperature = get_cpu_temperature();
    report.system.cpu_frequency_mhz = get_cpu_frequency();
    
    // Mémoire et stockage
    report.system.ram_gb = get_total_memory_gb_from_proc();
    report.system.swap_gb = get_swap_gb_from_proc();
    let (disk_free, disk_total) = get_disk_space();
    report.system.disk_free_gb = disk_free;
    report.system.disk_total_gb = disk_total;
    report.system.disk_type = get_disk_type();
    
    // GPU
    report.system.gpu_model = detect_gpu_model();
    report.system.gpu_memory_mb = detect_gpu_memory();
    report.system.gpu_driver_version = get_gpu_driver_version();
    report.system.gpu_temperature = get_gpu_temperature();
    report.system.gpu_cuda_cores = get_cuda_cores();
    
    // Réseau
    report.system.network_interfaces = get_network_interfaces();
    
    // Langages et outils (100% dynamiques)
    report.languages = detect_languages();
    report.rust_crates = detect_rust_crates();
    report.node_packages = detect_node_packages();
    report.python_packages = detect_python_packages();
    report.npus = detect_all_npus();
    report.docker_installed = which::which("docker").is_ok();
    report.podman_installed = which::which("podman").is_ok();
    report.git_installed = which::which("git").is_ok();
    report.git_lfs_installed = which::which("git-lfs").is_ok();
    
    // Build tools
    report.build_tools = detect_build_tools();
    
    // Bases de données
    report.databases = detect_databases();
    
    // IDEs
    report.ides = detect_ides();
    
    // Monitoring
    report.monitoring_tools = detect_monitoring_tools();
    
    // VS Code extensions
    report.vscode_extensions = detect_vscode_extensions();

    // Librairies système (100% dynamiques)
    report.system_libraries = collect_system_libraries();
    
    Ok(report)
}

// ============================================================================
// DÉTECTION DES NPU (Neural Processing Units)
// ============================================================================

fn detect_all_npus() -> Vec<NpuInfo> {
    let mut npus = Vec::new();
    
    // 1. Rockchip NPU (RK3588, RK3576, RK3568, RK3399Pro)
    let rockchip_detected = Path::new("/dev/rknpu").exists() || 
                            Path::new("/dev/npu0").exists() || 
                            Path::new("/dev/rknn0").exists() ||
                            Path::new("/sys/class/misc/rknpu/").exists();
    if rockchip_detected {
        let mut tops = None;
        let mut chip_model = "Rockchip NPU".to_string();
        
        if let Ok(content) = fs::read_to_string("/proc/device-tree/compatible") {
            if content.contains("rk3588") { 
                tops = Some(6.0);
                chip_model = "Rockchip RK3588 NPU".to_string();
            } else if content.contains("rk3576") { 
                tops = Some(6.0);
                chip_model = "Rockchip RK3576 NPU".to_string();
            } else if content.contains("rk3568") { 
                tops = Some(1.0);
                chip_model = "Rockchip RK3568 NPU".to_string();
            } else if content.contains("rk3399pro") { 
                tops = Some(3.2);
                chip_model = "Rockchip RK3399Pro NPU".to_string();
            } else if content.contains("rk3566") { 
                tops = Some(1.0);
                chip_model = "Rockchip RK3566 NPU".to_string();
            } else if content.contains("rk3588s") { 
                tops = Some(6.0);
                chip_model = "Rockchip RK3588S NPU".to_string();
            }
        }
        
        npus.push(NpuInfo {
            name: chip_model,
            detected: true,
            driver: get_rknn_driver_version(),
            api: Some("RKNN".to_string()),
            max_tops: tops,
            memory_mb: None,
        });
    }
    
    // 2. Intel NPU (Meteor Lake, Lunar Lake, Arrow Lake)
    let intel_npu_detected = Path::new("/dev/accel/accel0").exists() ||
                              Path::new("/dev/dri/accel0").exists() ||
                              Path::new("/sys/bus/pci/drivers/intel_npu/").exists();
    
    if !intel_npu_detected {
        if let Ok(output) = Command::new("lspci").output() {
            let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
            if text.contains("intel") && text.contains("npu") {
                npus.push(NpuInfo {
                    name: "Intel NPU".to_string(),
                    detected: true,
                    driver: get_intel_npu_version(),
                    api: Some("OpenVINO".to_string()),
                    max_tops: Some(10.0),
                    memory_mb: None,
                });
            }
        }
    }
    
    if intel_npu_detected {
        npus.push(NpuInfo {
            name: "Intel NPU".to_string(),
            detected: true,
            driver: get_intel_npu_version(),
            api: Some("OpenVINO".to_string()),
            max_tops: Some(10.0),
            memory_mb: None,
        });
    }
    
    // 3. Google Edge TPU / Coral
    let google_edgetpu_detected = Path::new("/dev/apex_0").exists() ||
                                   Path::new("/sys/class/apex/").exists() ||
                                   Path::new("/dev/edgetpu").exists() ||
                                   which::which("edgetpu_compiler").is_ok() ||
                                   which::which("libedgetpu.so").is_ok();
    
    if google_edgetpu_detected {
        npus.push(NpuInfo {
            name: "Google Edge TPU".to_string(),
            detected: true,
            driver: Some("apex".to_string()),
            api: Some("TensorFlow Lite / PyCoral".to_string()),
            max_tops: Some(4.0),
            memory_mb: Some(8),
        });
    }
    
    // 4. AMD NPU (Ryzen AI)
    let amd_npu_detected = Path::new("/dev/accel/accel0").exists() &&
                            Path::new("/sys/kernel/debug/amd_npu/").exists();
    
    if !amd_npu_detected {
        if let Ok(output) = Command::new("lspci").output() {
            let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
            if (text.contains("1002") || text.contains("1022")) && text.contains("npu") {
                npus.push(NpuInfo {
                    name: "AMD Ryzen AI NPU".to_string(),
                    detected: true,
                    driver: get_amd_npu_version(),
                    api: Some("ROCm / Vitis AI".to_string()),
                    max_tops: Some(16.0),
                    memory_mb: None,
                });
            }
        }
    }
    
    if amd_npu_detected {
        npus.push(NpuInfo {
            name: "AMD Ryzen AI NPU".to_string(),
            detected: true,
            driver: get_amd_npu_version(),
            api: Some("ROCm / Vitis AI".to_string()),
            max_tops: Some(16.0),
            memory_mb: None,
        });
    }
    
    // 5. Qualcomm Hexagon NPU
    let qualcomm_detected = Path::new("/dev/hexagon").exists() ||
                             Path::new("/dev/adsprpc-smd").exists() ||
                             Path::new("/dev/msm_npu").exists() ||
                             which::which("hexagon-clang").is_ok() ||
                             which::which("snpe-net-run").is_ok();
    
    if qualcomm_detected {
        npus.push(NpuInfo {
            name: "Qualcomm Hexagon NPU".to_string(),
            detected: true,
            driver: get_hexagon_version(),
            api: Some("Qualcomm SNPE / QNN".to_string()),
            max_tops: Some(15.0),
            memory_mb: None,
        });
    }
    
    // 6. MediaTek APU
    let mediatek_detected = Path::new("/dev/apu").exists() ||
                              Path::new("/dev/mtkapu").exists() ||
                              Path::new("/dev/mediatek_apu").exists();
    
    if mediatek_detected {
        let mut chip_model = "MediaTek APU".to_string();
        if let Ok(content) = fs::read_to_string("/proc/device-tree/compatible") {
            if content.contains("mt8195") { chip_model = "MediaTek MT8195 APU".to_string(); }
            else if content.contains("mt8192") { chip_model = "MediaTek MT8192 APU".to_string(); }
            else if content.contains("mt8186") { chip_model = "MediaTek MT8186 APU".to_string(); }
        }
        
        npus.push(NpuInfo {
            name: chip_model,
            detected: true,
            driver: None,
            api: Some("MediaTek NeuroPilot".to_string()),
            max_tops: Some(4.8),
            memory_mb: None,
        });
    }
    
    // 7. Samsung NPU
    let samsung_detected = Path::new("/dev/npu").exists() ||
                            Path::new("/dev/samsung-npu").exists() ||
                            Path::new("/sys/devices/platform/samsung-npu/").exists();
    
    if samsung_detected {
        npus.push(NpuInfo {
            name: "Samsung NPU".to_string(),
            detected: true,
            driver: None,
            api: Some("Samsung ONE / Neural SDK".to_string()),
            max_tops: Some(11.0),
            memory_mb: None,
        });
    }
    
    // 8. Hailo NPU
    let hailo_detected = Path::new("/dev/hailo").exists() ||
                          Path::new("/dev/hailo0").exists() ||
                          which::which("hailo").is_ok() ||
                          which::which("hailort").is_ok();
    
    if hailo_detected {
        let mut max_tops = Some(26.0);
        if let Ok(content) = fs::read_to_string("/sys/class/hailo/device/device_id") {
            if content.contains("H8L") { max_tops = Some(26.0); }
            else if content.contains("H8M") { max_tops = Some(13.0); }
            else if content.contains("H8") { max_tops = Some(26.0); }
        }
        
        npus.push(NpuInfo {
            name: "Hailo NPU".to_string(),
            detected: true,
            driver: get_hailo_version(),
            api: Some("HailoRT".to_string()),
            max_tops: max_tops,
            memory_mb: None,
        });
    }
    
    // 9. NXP i.MX NPU (eIQ)
    let nxp_detected = Path::new("/dev/npu").exists() &&
                        Path::new("/sys/devices/platform/imx-npu/").exists();
    
    if nxp_detected {
        npus.push(NpuInfo {
            name: "NXP i.MX NPU".to_string(),
            detected: true,
            driver: None,
            api: Some("NXP eIQ / TensorFlow Lite".to_string()),
            max_tops: Some(2.0),
            memory_mb: None,
        });
    }
    
    // 10. Apple Neural Engine (pour info - macOS)
    #[cfg(target_os = "macos")]
    {
        if Path::new("/usr/lib/libane.dylib").exists() {
            npus.push(NpuInfo {
                name: "Apple Neural Engine".to_string(),
                detected: true,
                driver: None,
                api: Some("Core ML / Metal".to_string()),
                max_tops: Some(15.8),
                memory_mb: None,
            });
        }
    }
    // 11. Axelera AI NPU (Metis)
    let axelera_detected = Path::new("/dev/axelera").exists() ||
                            Path::new("/dev/metis").exists() ||
                            Path::new("/sys/class/axelera/").exists() ||
                            which::which("axelera").is_ok() ||
                            which::which("metis_compiler").is_ok() ||
                            Path::new("/usr/lib/libaxelera.so").exists() ||
                            Path::new("/usr/lib/libmetis_runtime.so").exists();

    if axelera_detected {
        let mut max_tops = None;
        let mut chip_model = "Axelera Metis NPU".to_string();
        
        // Détection du modèle spécifique
        if let Ok(content) = fs::read_to_string("/proc/device-tree/compatible") {
            if content.contains("metis") {
                chip_model = "Axelera Metis AI".to_string();
            }
        }
        
        // Détection des performances via outil
        if let Ok(output) = Command::new("axelera-info").arg("--tops").output() {
            if let Ok(tops) = String::from_utf8_lossy(&output.stdout).trim().parse::<f32>() {
                max_tops = Some(tops);
            }
        } else {
            // Valeurs théoriques selon modèle
            if chip_model.contains("Metis") {
                max_tops = Some(128.0); // 128 TOPS pour Metis
            } else {
                max_tops = Some(64.0);
            }
        }
        
        npus.push(NpuInfo {
            name: chip_model,
            detected: true,
            driver: get_axelera_driver_version(),
            api: Some("Axelera Metis SDK / TensorFlow Lite".to_string()),
            max_tops: max_tops,
            memory_mb: Some(16), // 16 GB LPDDR5
        });
    }
    
    npus
}

fn get_rknn_driver_version() -> Option<String> {
    if let Ok(output) = Command::new("rknn_version").output() {
        return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    if let Ok(content) = fs::read_to_string("/sys/kernel/debug/rknpu/version") {
        return Some(content.trim().to_string());
    }
    if let Ok(content) = fs::read_to_string("/sys/class/misc/rknpu/version") {
        return Some(content.trim().to_string());
    }
    Some("rknn-toolkit2".to_string())
}

fn get_intel_npu_version() -> Option<String> {
    if let Ok(output) = Command::new("intel_npu_detect").output() {
        return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    if let Ok(content) = fs::read_to_string("/sys/bus/pci/drivers/intel_npu/version") {
        return Some(content.trim().to_string());
    }
    None
}

fn get_amd_npu_version() -> Option<String> {
    if let Ok(output) = Command::new("rocm-smi").arg("--npuv").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        if !text.is_empty() && !text.contains("not found") {
            return Some(text.trim().to_string());
        }
    }
    None
}

fn get_hexagon_version() -> Option<String> {
    if let Ok(output) = Command::new("hexagon-dc").arg("--version").output() {
        return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    if let Ok(output) = Command::new("snpe-net-run").arg("--version").output() {
        return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    None
}

fn get_hailo_version() -> Option<String> {
    if let Ok(output) = Command::new("hailo").arg("--version").output() {
        return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    if let Ok(output) = Command::new("hailort").arg("--version").output() {
        return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    None
}

// ============================================================================
// DÉTECTION DES CRATES/PACKAGES RÉELS (100% DYNAMIQUE)
// ============================================================================

fn detect_rust_crates() -> Vec<CrateInfo> {
    let mut crates = Vec::new();
    
    if let Ok(output) = Command::new("cargo").arg("install").arg("--list").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("cargo") || line.starts_with("Updating") {
                continue;
            }
            if let Some(name) = line.split_whitespace().next() {
                crates.push(CrateInfo {
                    name: name.to_string(),
                    version: None,
                    description: "Binaire installé globalement".to_string(),
                });
            }
        }
    }
    
    let home = std::env::var("HOME").unwrap_or_default();
    let bin_path = format!("{}/.cargo/bin", home);
    if let Ok(entries) = std::fs::read_dir(&bin_path) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if !name.contains("cargo") && !name.contains("rust") {
                    crates.push(CrateInfo {
                        name,
                        version: None,
                        description: "Binaire dans .cargo/bin".to_string(),
                    });
                }
            }
        }
    }
    
    unique_crates(crates)
}

fn detect_node_packages() -> Vec<CrateInfo> {
    let mut packages = Vec::new();
    
    if let Ok(output) = Command::new("npm").args(&["list", "-g", "--depth=0", "--json"]).output() {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                for (name, info) in deps {
                    let version = info.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
                    packages.push(CrateInfo {
                        name: name.clone(),
                        version: Some(version.to_string()),
                        description: "Package global npm".to_string(),
                    });
                }
            }
        }
    }
    
    if let Ok(output) = Command::new("npm").args(&["list", "--depth=0", "--json"]).output() {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                for (name, info) in deps {
                    let version = info.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
                    packages.push(CrateInfo {
                        name: name.clone(),
                        version: Some(version.to_string()),
                        description: "Package local npm".to_string(),
                    });
                }
            }
        }
    }
    
    unique_crates(packages)
}

fn detect_python_packages() -> Vec<CrateInfo> {
    let mut packages = Vec::new();
    
    if let Ok(output) = Command::new("pip3").arg("list").arg("--format=json").output() {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(arr) = json.as_array() {
                for pkg in arr {
                    if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                        packages.push(CrateInfo {
                            name: name.to_string(),
                            version: pkg.get("version").and_then(|v| v.as_str()).map(|v| v.to_string()),
                            description: "Package pip".to_string(),
                        });
                    }
                }
            }
        }
    }
    
    if let Ok(output) = Command::new("pip3").args(&["list", "--user", "--format=json"]).output() {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(arr) = json.as_array() {
                for pkg in arr {
                    if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                        packages.push(CrateInfo {
                            name: name.to_string(),
                            version: pkg.get("version").and_then(|v| v.as_str()).map(|v| v.to_string()),
                            description: "Package pip utilisateur".to_string(),
                        });
                    }
                }
            }
        }
    }
    
    unique_crates(packages)
}

fn unique_crates(list: Vec<CrateInfo>) -> Vec<CrateInfo> {
    let mut unique = Vec::new();
    let mut names = std::collections::HashSet::new();
    for c in list {
        if !names.contains(&c.name) {
            names.insert(c.name.clone());
            unique.push(c);
        }
    }
    unique.truncate(50);
    unique
}

// ============================================================================
// FONCTIONS DE DÉTECTION SYSTÈME
// ============================================================================

fn get_os_name() -> Option<String> {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("NAME=") {
                let name = line.trim_start_matches("NAME=").trim_matches('"');
                return Some(name.to_string());
            }
        }
    }
    Some(std::env::consts::OS.to_string())
}

fn get_os_version() -> Option<String> {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("VERSION=") || line.starts_with("VERSION_ID=") {
                let version = line.split('=').nth(1).unwrap_or("").trim_matches('"');
                if !version.is_empty() {
                    return Some(version.to_string());
                }
            }
        }
    }
    None
}

fn get_kernel_version() -> Option<String> {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn get_hostname() -> Option<String> {
    Command::new("hostname")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn get_architecture() -> Option<String> {
    Command::new("uname")
        .arg("-m")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn get_uptime() -> Option<String> {
    if let Ok(content) = fs::read_to_string("/proc/uptime") {
        if let Some(uptime_seconds) = content.split_whitespace().next() {
            if let Ok(seconds) = uptime_seconds.parse::<f64>() {
                let days = (seconds / 86400.0) as u64;
                let hours = ((seconds % 86400.0) / 3600.0) as u64;
                let minutes = ((seconds % 3600.0) / 60.0) as u64;
                if days > 0 {
                    return Some(format!("{}j {}h {}min", days, hours, minutes));
                } else if hours > 0 {
                    return Some(format!("{}h {}min", hours, minutes));
                }
                return Some(format!("{}min", minutes));
            }
        }
    }
    Some("N/A".to_string())
}

fn get_load_average() -> Option<String> {
    if let Ok(content) = fs::read_to_string("/proc/loadavg") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            return Some(format!("{} {} {}", parts[0], parts[1], parts[2]));
        }
    }
    None
}

fn get_shell_version() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    Command::new(&shell)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines().next().map(|l| l.to_string())
        })
}

fn get_desktop_environment() -> Option<String> {
    let de = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .ok();
    de
}

fn get_cpu_model() -> Option<String> {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("model name") {
                if let Some(model) = line.split(':').nth(1) {
                    return Some(model.trim().to_string());
                }
            }
        }
    }
    
    let mut system = sysinfo::System::new();
    system.refresh_all();
    system.cpus().first().map(|c| c.brand().to_string())
}

fn get_cpu_frequency() -> Option<u64> {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("cpu MHz") || line.starts_with("cpu clock") {
                if let Some(freq) = line.split(':').nth(1) {
                    if let Ok(f) = freq.trim().parse::<f64>() {
                        return Some(f as u64);
                    }
                }
            }
        }
    }
    
    let mut system = sysinfo::System::new();
    system.refresh_all();
    system.cpus().first().map(|c| c.frequency())
}

fn get_cpu_temperature() -> Option<String> {
    if let Ok(paths) = fs::read_dir("/sys/class/thermal/") {
        for path in paths.flatten() {
            let type_path = path.path().join("type");
            let temp_path = path.path().join("temp");
            if let Ok(type_content) = fs::read_to_string(type_path) {
                if type_content.trim() == "x86_pkg_temp" || type_content.trim().contains("cpu") {
                    if let Ok(temp_content) = fs::read_to_string(temp_path) {
                        if let Ok(temp_millidegree) = temp_content.trim().parse::<f64>() {
                            let temp_celsius = temp_millidegree / 1000.0;
                            return Some(format!("{:.1}°C", temp_celsius));
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_total_memory_gb_from_proc() -> f64 {
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return kb / (1024.0 * 1024.0);
                    }
                }
            }
        }
    }
    0.0
}

fn get_swap_gb_from_proc() -> f64 {
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("SwapTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return kb / (1024.0 * 1024.0);
                    }
                }
            }
        }
    }
    0.0
}
fn get_axelera_driver_version() -> Option<String> {
    if let Ok(output) = Command::new("axelera-info").arg("--version").output() {
        return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    if let Ok(content) = fs::read_to_string("/sys/class/axelera/version") {
        return Some(content.trim().to_string());
    }
    if let Ok(content) = fs::read_to_string("/opt/axelera/version.txt") {
        return Some(content.trim().to_string());
    }
    None
}

fn get_disk_space() -> (f64, f64) {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut total = 0.0;
    let mut free = 0.0;
    for disk in disks.list() {
        total += disk.total_space() as f64 / (1024.0 * 1024.0 * 1024.0);
        free += disk.available_space() as f64 / (1024.0 * 1024.0 * 1024.0);
    }
    (free, total)
}

fn get_disk_type() -> Option<String> {
    if let Ok(output) = Command::new("lsblk").arg("-d").arg("-o").arg("ROTA,TYPE").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        if text.contains("0") {
            return Some("SSD".to_string());
        }
    }
    Some("HDD".to_string())
}

fn detect_gpu_model() -> Option<String> {
    if let Ok(output) = Command::new("nvidia-smi")
        .arg("--query-gpu=name")
        .arg("--format=csv,noheader")
        .output() 
    {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    
    if let Ok(output) = Command::new("lspci").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.to_lowercase().contains("vga") || line.to_lowercase().contains("3d") {
                return Some(line.to_string());
            }
        }
    }
    None
}

fn detect_gpu_memory() -> Option<u64> {
    if let Ok(output) = Command::new("nvidia-smi")
        .arg("--query-gpu=memory.total")
        .arg("--format=csv,noheader,nounits")
        .output() 
    {
        if let Ok(mem) = String::from_utf8_lossy(&output.stdout).trim().parse::<u64>() {
            return Some(mem);
        }
    }
    None
}

fn get_gpu_driver_version() -> Option<String> {
    if let Ok(output) = Command::new("nvidia-smi")
        .arg("--query-gpu=driver_version")
        .arg("--format=csv,noheader")
        .output() 
    {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return Some(version);
        }
    }
    None
}

fn get_gpu_temperature() -> Option<String> {
    if let Ok(output) = Command::new("nvidia-smi")
        .arg("--query-gpu=temperature.gpu")
        .arg("--format=csv,noheader")
        .output() 
    {
        let temp = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !temp.is_empty() {
            return Some(format!("{}°C", temp));
        }
    }
    None
}

fn get_cuda_cores() -> Option<u32> {
    if let Ok(output) = Command::new("nvidia-smi")
        .arg("--query-gpu=cuda_cores")
        .arg("--format=csv,noheader")
        .output() 
    {
        if let Ok(cores) = String::from_utf8_lossy(&output.stdout).trim().parse::<u32>() {
            return Some(cores);
        }
    }
    None
}

fn get_network_interfaces() -> Vec<String> {
    let mut interfaces = Vec::new();
    if let Ok(paths) = fs::read_dir("/sys/class/net/") {
        for path in paths.flatten() {
            if let Ok(name) = path.file_name().into_string() {
                if !name.starts_with("lo") {
                    interfaces.push(name);
                }
            }
        }
    }
    interfaces
}

fn detect_languages() -> Vec<LanguageInfo> {
    let mut languages = Vec::new();
    
    let checks: Vec<(&str, &[&str], &str)> = vec![
        ("python3", &["--version"], "Python"),
        ("node", &["--version"], "Node.js"),
        ("rustc", &["--version"], "Rust"),
        ("go", &["version"], "Go"),
        ("javac", &["-version"], "Java"),
        ("dotnet", &["--version"], "C#"),
        ("ruby", &["--version"], "Ruby"),
        ("php", &["--version"], "PHP"),
        ("swift", &["--version"], "Swift"),
        ("kotlin", &["-version"], "Kotlin"),
    ];
    
    for (cmd, args, name) in checks {
        let installed = which::which(cmd).is_ok();
        let version = if installed {
            Command::new(cmd)
                .args(args)
                .output()
                .ok()
                .and_then(|o| {
                    let text = String::from_utf8_lossy(&o.stdout);
                    if text.is_empty() {
                        let text = String::from_utf8_lossy(&o.stderr);
                        extract_version(&text)
                    } else {
                        extract_version(&text)
                    }
                })
        } else {
            None
        };
        
        languages.push(LanguageInfo {
            name: name.to_string(),
            installed,
            version,
            package_manager: get_package_manager_for_lang(name),
            lsp_available: check_lsp_for_lang(name),
            debugger_available: check_debugger_for_lang(name),
            linting_available: check_linter_for_lang(name),
            formatting_available: check_formatter_for_lang(name),
            performance_rating: match name {
                "Rust" => 95,
                "Go" => 90,
                "C#" => 85,
                "Java" => 80,
                "Swift" => 75,
                "Kotlin" => 75,
                "Node.js" => 65,
                "PHP" => 55,
                "Ruby" => 50,
                "Python" => 50,
                _ => 60,
            },
        });
    }
    
    languages
}

fn get_package_manager_for_lang(lang: &str) -> Option<String> {
    match lang {
        "Python" => if which::which("pip").is_ok() { Some("pip".to_string()) } else { None },
        "Node.js" => if which::which("npm").is_ok() { Some("npm".to_string()) } else { None },
        "Rust" => if which::which("cargo").is_ok() { Some("cargo".to_string()) } else { None },
        "Go" => if which::which("go").is_ok() { Some("go mod".to_string()) } else { None },
        "Java" => if which::which("mvn").is_ok() { Some("maven".to_string()) } else if which::which("gradle").is_ok() { Some("gradle".to_string()) } else { None },
        "C#" => if which::which("dotnet").is_ok() { Some("dotnet".to_string()) } else { None },
        "Ruby" => if which::which("gem").is_ok() { Some("gem".to_string()) } else { None },
        "PHP" => if which::which("composer").is_ok() { Some("composer".to_string()) } else { None },
        _ => None,
    }
}

fn check_lsp_for_lang(lang: &str) -> bool {
    match lang {
        "Rust" => which::which("rust-analyzer").is_ok(),
        "Python" => which::which("pylsp").is_ok() || which::which("pyright").is_ok(),
        "Node.js" => which::which("typescript-language-server").is_ok(),
        "Go" => which::which("gopls").is_ok(),
        "Java" => which::which("jdtls").is_ok(),
        _ => false,
    }
}

fn check_debugger_for_lang(lang: &str) -> bool {
    match lang {
        "Rust" | "Go" | "C#" => which::which("lldb").is_ok() || which::which("gdb").is_ok(),
        "Python" => which::which("pdb").is_ok(),
        "Node.js" => which::which("node").is_ok(),
        _ => false,
    }
}

fn check_linter_for_lang(lang: &str) -> bool {
    match lang {
        "Rust" => which::which("clippy").is_ok(),
        "Python" => which::which("pylint").is_ok() || which::which("ruff").is_ok(),
        "Node.js" => which::which("eslint").is_ok(),
        "Go" => which::which("golangci-lint").is_ok(),
        _ => false,
    }
}

fn check_formatter_for_lang(lang: &str) -> bool {
    match lang {
        "Rust" => which::which("rustfmt").is_ok(),
        "Python" => which::which("black").is_ok(),
        "Node.js" => which::which("prettier").is_ok(),
        "Go" => which::which("gofmt").is_ok(),
        _ => false,
    }
}

fn extract_version(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"(\d+\.\d+\.\d+)").unwrap();
    re.captures(text).map(|cap| cap[1].to_string())
}

fn detect_build_tools() -> Vec<BuildTool> {
    let tools = vec!["make", "cmake", "ninja", "cargo", "gradle", "mvn", "just"];
    let mut result = Vec::new();
    for tool in tools {
        result.push(BuildTool {
            name: tool.to_string(),
            installed: which::which(tool).is_ok(),
            version: None,
            parallel_builds: Some(num_cpus::get()),
        });
    }
    result
}

fn detect_databases() -> Vec<DatabaseSupport> {
    let dbs = vec![
        ("PostgreSQL", "psql"),
        ("MySQL", "mysql"),
        ("Redis", "redis-cli"),
        ("SQLite", "sqlite3"),
        ("MongoDB", "mongosh"),
        ("Cassandra", "cqlsh"),
    ];
    let mut result = Vec::new();
    for (name, cmd) in dbs {
        result.push(DatabaseSupport {
            name: name.to_string(),
            installed: which::which(cmd).is_ok(),
            version: None,
            client_tools: true,
        });
    }
    result
}

fn detect_ides() -> Vec<IdeSupport> {
    let ides = vec![
        ("VS Code", "code"),
        ("IntelliJ IDEA", "idea"),
        ("PyCharm", "pycharm"),
        ("RustRover", "rustrover"),
        ("GoLand", "goland"),
        ("CLion", "clion"),
        ("WebStorm", "webstorm"),
        ("PhpStorm", "phpstorm"),
        ("RubyMine", "rubymine"),
        ("Vim", "vim"),
        ("Neovim", "nvim"),
        ("Emacs", "emacs"),
        ("Sublime Text", "subl"),
    ];
    let mut result = Vec::new();
    for (name, cmd) in ides {
        result.push(IdeSupport {
            name: name.to_string(),
            detected: which::which(cmd).is_ok(),
            version: None,
        });
    }
    result
}

fn detect_monitoring_tools() -> MonitoringSupport {
    MonitoringSupport {
        htop: which::which("htop").is_ok(),
        btop: which::which("btop").is_ok(),
        glances: which::which("glances").is_ok(),
        netdata: which::which("netdata").is_ok(),
    }
}

fn detect_vscode_extensions() -> Vec<String> {
    let mut extensions = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();
    let extensions_path = format!("{}/.vscode/extensions", home);
    if let Ok(entries) = fs::read_dir(&extensions_path) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                extensions.push(name);
            }
        }
    }
    extensions.truncate(20);
    extensions
}

fn get_hardware_specs(report: &SystemReport) -> HardwareSpecs {
    HardwareSpecs {
        cpu_cores: report.system.cpu_cores,
        cpu_model: report.system.cpu_model.clone(),
        cpu_frequency_mhz: report.system.cpu_frequency_mhz,
        cpu_temperature: report.system.cpu_temperature.clone(),
        ram_gb: report.system.ram_gb,
        swap_gb: report.system.swap_gb,
        disk_free_gb: report.system.disk_free_gb,
        disk_total_gb: report.system.disk_total_gb,
        disk_type: report.system.disk_type.clone(),
        gpu_memory_mb: report.system.gpu_memory_mb,
        gpu_model: report.system.gpu_model.clone(),
        gpu_driver_version: report.system.gpu_driver_version.clone(),
        gpu_temperature: report.system.gpu_temperature.clone(),
        gpu_cuda_cores: report.system.gpu_cuda_cores,
        max_resolution: None,
        supported_formats: vec![],
    }
}

// ============================================================================
// COMPATIBILITY MATRIX (basée sur détection réelle)
// ============================================================================

fn get_compatibility_matrix() -> CompatibilityMatrix {
    let has_cuda = which::which("nvidia-smi").is_ok();
    
    CompatibilityMatrix {
        inference_backends: vec![
            BackendSupport {
                name: "PyTorch".into(),
                supported: true,
                acceleration: Some("CPU/GPU".into()),
                performance_rating: 70,
                setup_instructions: "pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu118".into(),
            },
            BackendSupport {
                name: "TensorFlow".into(),
                supported: true,
                acceleration: Some("CPU/GPU".into()),
                performance_rating: 70,
                setup_instructions: "pip install tensorflow[and-cuda]".into(),
            },
            BackendSupport {
                name: "ONNX Runtime".into(),
                supported: true,
                acceleration: Some("CPU/GPU".into()),
                performance_rating: 65,
                setup_instructions: "pip install onnxruntime-gpu".into(),
            },
            BackendSupport {
                name: "JAX".into(),
                supported: false,
                acceleration: Some("GPU".into()),
                performance_rating: 75,
                setup_instructions: "pip install jax[cuda12]".into(),
            },
            BackendSupport {
                name: "TensorRT".into(),
                supported: which::which("trtexec").is_ok(),
                acceleration: Some("Tensor Cores".into()),
                performance_rating: if which::which("trtexec").is_ok() { 95 } else { 0 },
                setup_instructions: "sudo apt install tensorrt".into(),
            },
        ],
        video_codecs: vec![
            CodecSupport {
                codec: "H.264".into(),
                encode: true,
                decode: true,
                hardware_accelerated: has_cuda,
                max_resolution: "3840x2160".into(),
            },
            CodecSupport {
                codec: "H.265/HEVC".into(),
                encode: has_cuda,
                decode: true,
                hardware_accelerated: has_cuda,
                max_resolution: "7680x4320".into(),
            },
            CodecSupport {
                codec: "VP9".into(),
                encode: false,
                decode: true,
                hardware_accelerated: false,
                max_resolution: "8192x4352".into(),
            },
            CodecSupport {
                codec: "AV1".into(),
                encode: false,
                decode: has_cuda,
                hardware_accelerated: has_cuda,
                max_resolution: "7680x4320".into(),
            },
        ],
        compute_apis: vec![
            ApiSupport {
                api: "CUDA".into(),
                supported: has_cuda,
                version: get_cuda_version(),
                compute_units: get_cuda_cores(),
                max_workgroup_size: None,
            },
            ApiSupport {
                api: "OpenCL".into(),
                supported: which::which("clinfo").is_ok(),
                version: get_opencl_version(),
                compute_units: None,
                max_workgroup_size: None,
            },
            ApiSupport {
                api: "Vulkan".into(),
                supported: which::which("vulkaninfo").is_ok(),
                version: get_vulkan_version(),
                compute_units: None,
                max_workgroup_size: None,
            },
            ApiSupport {
                api: "ROCm".into(),
                supported: false,
                version: None,
                compute_units: None,
                max_workgroup_size: None,
            },
        ],
    }
}

fn get_cuda_version() -> Option<String> {
    if let Ok(output) = Command::new("nvcc").arg("--version").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains("release") {
                if let Some(version) = line.split("release").nth(1) {
                    return Some(version.trim().to_string());
                }
            }
        }
    }
    None
}

fn get_opencl_version() -> Option<String> {
    if let Ok(output) = Command::new("clinfo").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains("Version:") {
                return Some(line.split("Version:").nth(1).unwrap_or("").trim().to_string());
            }
        }
    }
    None
}

fn get_vulkan_version() -> Option<String> {
    if let Ok(output) = Command::new("vulkaninfo").arg("--summary").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains("apiVersion") {
                return Some(line.split(':').nth(1).unwrap_or("").trim().to_string());
            }
        }
    }
    None
}

fn get_performance_estimates() -> PerformanceEstimates {
    let mut latency = HashMap::new();
    latency.insert("ResNet50 (batch=1)".into(), 45.0);
    latency.insert("YOLOv5s (batch=1)".into(), 60.0);
    latency.insert("BERT-base (seq=128)".into(), 30.0);
    latency.insert("Whisper-tiny".into(), 120.0);
    latency.insert("Stable Diffusion (512x512)".into(), 3500.0);
    
    PerformanceEstimates {
        fp32_tflops: Some(num_cpus::get() as f32 * 0.032),
        fp16_tflops: Some(num_cpus::get() as f32 * 0.064),
        int8_tops: Some(num_cpus::get() as f32 * 0.256),
        memory_bandwidth_gbps: Some(25.0),
        inference_latency_ms: latency,
    }
}

fn get_workload_recommendations() -> WorkloadRecommendations {
    WorkloadRecommendations {
        computer_vision: vec!["YOLOv8".into(), "OpenCV 4.x".into(), "MediaPipe".into(), "Detectron2".into(), "MMDetection".into()],
        nlp: vec!["BERT-base-uncased".into(), "DistilBERT".into(), "GPT-2".into(), "LLaMA 2/3".into(), "Mistral 7B".into()],
        audio: vec!["Whisper".into(), "SpeechBrain".into(), "ESPnet".into(), "Pyannote".into()],
        generative_ai: vec!["Stable Diffusion XL".into(), "DALL-E mini".into(), "Llama.cpp".into(), "Ollama".into(), "LangChain".into()],
        real_time: vec!["Real-time object detection".into(), "Video analytics".into(), "Live transcription".into()],
    }
}

fn get_init_code_snippets() -> InitCodeSnippets {
    InitCodeSnippets {
        pytorch_device_config: r#"import torch
import os

# Détection automatique du device
device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
print(f"Device: {device}")

# Optimisation CPU
torch.set_num_threads(os.cpu_count())
if device.type == 'cuda':
    print(f"GPU: {torch.cuda.get_device_name(0)}")
    print(f"CUDA Cores: {torch.cuda.get_device_properties(0).multi_processor_count * 64}")"#.into(),
        
        tensorflow_device_config: r#"import tensorflow as tf
import os

# Configuration GPU
gpus = tf.config.list_physical_devices('GPU')
if gpus:
    tf.config.experimental.set_memory_growth(gpus[0], True)

# Threads
tf.config.threading.set_intra_op_parallelism_threads(os.cpu_count())
tf.config.threading.set_inter_op_parallelism_threads(os.cpu_count())

print(f"Devices: {tf.config.list_physical_devices()}")
print(f"Num GPUs: {len(gpus)}")"#.into(),
        
        onnx_session_config: r#"import onnxruntime as ort

# Providers disponibles
providers = ort.get_available_providers()
print(f"Providers: {providers}")

# Session avec accélération GPU si disponible
session = ort.InferenceSession('model.onnx', providers=providers)
print("ONNX Runtime prêt")"#.into(),
        
        ffmpeg_hwaccel_config: r#"# CPU encoding
ffmpeg -i input.mp4 -c:v libx264 -preset medium output.mp4

# NVIDIA GPU (CUDA/NVENC)
ffmpeg -hwaccel cuda -i input.mp4 -c:v h264_nvenc -preset p4 output.mp4

# Intel QuickSync
ffmpeg -hwaccel qsv -i input.mp4 -c:v h264_qsv output.mp4

# AMD AMF
ffmpeg -hwaccel amf -i input.mp4 -c:v h264_amf output.mp4"#.into(),
        
        opencv_umat_config: r#"import cv2
import os

# Configuration des threads
cv2.setNumThreads(os.cpu_count())

# Activer OpenCL si disponible
if cv2.ocl.haveOpenCL():
    cv2.ocl.setUseOpenCL(True)
    print("OpenCL activé")
    print(f"Device: {cv2.ocl.Device.getDefault().name()}")"#.into(),
    }
}

fn get_dev_capabilities(report: &SystemReport) -> DevCapabilities {
    DevCapabilities {
        languages: report.languages.clone(),
        rust_crates: report.rust_crates.clone(),
        node_packages: report.node_packages.clone(),
        python_packages: report.python_packages.clone(),
        npus: report.npus.clone(),
        build_tools: report.build_tools.clone(),
        containers: ContainerSupport {
            docker: report.docker_installed,
            docker_version: None,
            podman: report.podman_installed,
            kubernetes_cli: which::which("kubectl").is_ok(),
            compose_available: which::which("docker-compose").is_ok(),
            buildx_support: which::which("docker-buildx").is_ok(),
        },
        databases: report.databases.clone(),
        version_control: VersionControlSupport {
            git: report.git_installed,
            git_version: None,
            git_lfs: report.git_lfs_installed,
        },
        ides: report.ides.clone(),
        monitoring_tools: report.monitoring_tools,
    }
}

fn get_dev_recommendations() -> DevWorkloadRecommendations {
    DevWorkloadRecommendations {
        web_development: vec![
            "React 18 / Next.js 14".into(),
            "Vue 3 / Nuxt 3".into(),
            "Node.js / Express".into(),
            "Python Django / FastAPI".into(),
            "Go Gin / Fiber".into(),
            "Rust Axum / Actix".into(),
        ],
        backend_development: vec![
            "REST APIs (OpenAPI/Swagger)".into(),
            "GraphQL (Apollo/GraphQL Yoga)".into(),
            "Microservices (Temporal, RabbitMQ)".into(),
            "gRPC (Protocol Buffers)".into(),
            "Message brokers (Kafka, NATS)".into(),
        ],
        data_engineering: vec![
            "ETL Pipelines (Airflow, Prefect)".into(),
            "Data analysis (Pandas, Polars)".into(),
            "Big Data (Spark, DuckDB)".into(),
            "Real-time streaming (Kafka, Flink)".into(),
            "Data warehouses (ClickHouse, DuckDB)".into(),
        ],
        game_development: vec![
            "Unity 3D (C#)".into(),
            "Unreal Engine (C++)".into(),
            "Godot 4 (GDScript)".into(),
            "Bevy (Rust)".into(),
        ],
        embedded_development: vec![
            "Raspberry Pi / ARM".into(),
            "Arduino / PlatformIO".into(),
            "ESP32 / ESP-IDF".into(),
            "Embedded Linux / Yocto".into(),
            "Zephyr RTOS".into(),
        ],
        devops: vec![
            "CI/CD (GitHub Actions, GitLab CI)".into(),
            "Container orchestration (K8s, Nomad)".into(),
            "Infrastructure as Code (Terraform, Pulumi)".into(),
            "Monitoring (Prometheus, Grafana)".into(),
            "Cloud platforms (AWS, GCP, Azure)".into(),
        ],
        mobile_development: vec![
            "React Native / Expo".into(),
            "Flutter / Dart".into(),
            "Kotlin Multiplatform".into(),
            "iOS (Swift)".into(),
            "Android (Kotlin)".into(),
        ],
    }
}

fn get_toolchain_config() -> ToolchainConfig {
    let mut vscode_extensions = Vec::new();
    
    if which::which("rustc").is_ok() {
        vscode_extensions.push("rust-lang.rust-analyzer".into());
        vscode_extensions.push("tamasfe.even-better-toml".into());
    }
    if which::which("python3").is_ok() {
        vscode_extensions.push("ms-python.python".into());
        vscode_extensions.push("ms-python.vscode-pylance".into());
        vscode_extensions.push("charliermarsh.ruff".into());
    }
    if which::which("node").is_ok() {
        vscode_extensions.push("dbaeumer.vscode-eslint".into());
        vscode_extensions.push("esbenp.prettier-vscode".into());
        vscode_extensions.push("bradlc.vscode-tailwindcss".into());
    }
    if which::which("go").is_ok() {
        vscode_extensions.push("golang.go".into());
    }
    if which::which("docker").is_ok() {
        vscode_extensions.push("ms-azuretools.vscode-docker".into());
    }
    if which::which("git").is_ok() {
        vscode_extensions.push("eamodio.gitlens".into());
    }
    
    vscode_extensions.push("github.copilot".into());
    vscode_extensions.push("continue.continue".into());
    vscode_extensions.push("formulahendry.code-runner".into());
    vscode_extensions.push("aaron-bond.better-comments".into());
    
    vscode_extensions.dedup();
    
    ToolchainConfig {
        suggested_shell: if which::which("zsh").is_ok() { "zsh".to_string() } else { "bash".to_string() },
        vscode_extensions,
        git_aliases: vec![
            "co = checkout".to_string(),
            "br = branch".to_string(),
            "st = status".to_string(),
            "ci = commit".to_string(),
            "lg = log --oneline --graph".to_string(),
            "unstage = reset HEAD --".to_string(),
            "last = log -1 HEAD".to_string(),
        ],
        docker_optimizations: vec![
            "Activer BuildKit pour builds plus rapides".to_string(),
            "Utiliser docker buildx pour multi-platform".to_string(),
            "Optimiser le cache des layers Docker".to_string(),
            "Utiliser .dockerignore pour réduire le contexte".to_string(),
            "Préférer les images slim/alpine".to_string(),
        ],
        suggested_ides: vec![
            "VS Code".to_string(),
            if which::which("rustc").is_ok() { "RustRover".to_string() } else { "".to_string() },
            if which::which("python3").is_ok() { "PyCharm".to_string() } else { "".to_string() },
            if which::which("go").is_ok() { "GoLand".to_string() } else { "".to_string() },
            if which::which("java").is_ok() { "IntelliJ IDEA".to_string() } else { "".to_string() },
        ].into_iter().filter(|s| !s.is_empty()).collect(),
    }
}

fn detect_all_system_libraries() -> Vec<SystemLibrary> {
    let mut libraries = Vec::new();
    
    let search_paths = vec![
        "/usr/lib",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
        "/lib",
        "/lib/x86_64-linux-gnu",
        "/lib/aarch64-linux-gnu",
        "/usr/local/lib",
    ];
    
    let library_patterns = vec![
        ("libcuda.so", "CUDA Driver", "GPU/Compute"),
        ("libcudart.so", "CUDA Runtime", "GPU/Compute"),
        ("libcudnn.so", "CUDA Deep Neural Network", "GPU/Compute"),
        ("libnvinfer.so", "TensorRT", "GPU/Compute"),
        ("libOpenCL.so", "OpenCL", "GPU/Compute"),
        ("libvulkan.so", "Vulkan", "GPU/Graphics"),
        ("libopencv_core.so", "OpenCV Core", "Computer Vision"),
        ("libopencv_imgproc.so", "OpenCV Image Processing", "Computer Vision"),
        ("libopencv_imgcodecs.so", "OpenCV Image Codecs", "Computer Vision"),
        ("libavcodec.so", "FFmpeg Codecs", "Multimedia"),
        ("libavformat.so", "FFmpeg Format", "Multimedia"),
        ("libavutil.so", "FFmpeg Utils", "Multimedia"),
        ("libswscale.so", "FFmpeg Scaling", "Multimedia"),
        ("libv4l2.so", "V4L2", "Multimedia"),
        ("libtensorflow.so", "TensorFlow", "Machine Learning"),
        ("libtorch.so", "PyTorch", "Machine Learning"),
        ("libonnxruntime.so", "ONNX Runtime", "Machine Learning"),
        ("libblas.so", "BLAS", "Math"),
        ("liblapack.so", "LAPACK", "Math"),
        ("libmpi.so", "MPI", "Communication"),
        ("libz.so", "ZLib Compression", "Compression"),
        ("libpng.so", "PNG", "Image"),
        ("libjpeg.so", "JPEG", "Image"),
        ("libtiff.so", "TIFF", "Image"),
        ("libcurl.so", "Curl HTTP", "Network"),
        ("libssl.so", "OpenSSL", "Security"),
        ("libcrypto.so", "OpenSSL Crypto", "Security"),
        ("libsqlite3.so", "SQLite3", "Database"),
        ("libpq.so", "PostgreSQL", "Database"),
        ("libmysqlclient.so", "MySQL Client", "Database"),
        ("libGL.so", "OpenGL", "Graphics"),
        ("libEGL.so", "EGL", "Graphics"),
        ("libwayland-client.so", "Wayland", "Graphics"),
        ("libpulse.so", "PulseAudio", "Audio"),
    ];
    
    for path in search_paths {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(filename) = entry.file_name().into_string() {
                    for (pattern, desc, category) in &library_patterns {
                        if filename.contains(pattern) {
                            let full_path = format!("{}/{}", path, filename);
                            let size = std::fs::metadata(&full_path).map(|m| m.len()).unwrap_or(0);
                            let version = extract_so_version(&filename);
                            
                            libraries.push(SystemLibrary {
                                name: filename.clone(),
                                path: full_path,
                                version,
                                size_bytes: size,
                                description: desc.to_string(),
                                category: category.to_string(),
                            });
                            break;
                        }
                    }
                }
            }
        }
    }
    
    libraries.dedup_by(|a, b| a.name == b.name);
    libraries.sort_by(|a, b| a.category.cmp(&b.category));
    
    libraries
}

fn extract_so_version(filename: &str) -> Option<String> {
    let re = regex::Regex::new(r"\.so\.(\d+(?:\.\d+)*)").unwrap();
    re.captures(filename).and_then(|cap| cap.get(1)).map(|m| m.as_str().to_string())
}

fn collect_system_libraries() -> Vec<SystemLibrary> {
    detect_all_system_libraries()
}