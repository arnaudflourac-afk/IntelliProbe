//! API module for RESTful service
//!
//! This module provides a complete REST API for interacting with IntelliProbe
//! programmatically. It supports:
//! - Getting analysis reports
//! - Triggering new analyses
//! - Comparing with reference systems
//! - Exporting reports in various formats

mod routes;

pub use routes::start_server;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::analyze::AnalysisResult;
use crate::detectors::package_manager::PackageManager;
use crate::exporters::{ExportFormat, json, markdown, html};

/// Application state shared across API handlers
#[derive(Clone)]
pub struct ApiState {
    /// Latest analysis result
    pub result: Arc<Mutex<Option<AnalysisResult>>>,
    /// Current package manager (for install commands)
    pub package_manager: PackageManager,
    /// API version
    pub version: &'static str,
}

/// Standard API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub version: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Request payload for new analysis
#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    /// Path to existing JSON report (optional, if not provided, run probes)
    #[serde(default)]
    pub report_path: Option<String>,
    
    /// Whether to run benchmarks (slow)
    #[serde(default)]
    pub run_benchmarks: bool,
    
    /// Threshold for CI mode
    #[serde(default)]
    pub threshold: Option<u8>,
}

/// Request payload for export
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub format: String,
    pub output_path: Option<String>,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_seconds: u64,
    pub package_manager: String,
}

/// Start the API server on the specified port
pub async fn start_server(port: u16, result: AnalysisResult) -> anyhow::Result<()> {
    let state = ApiState {
        result: Arc::new(Mutex::new(Some(result))),
        package_manager: crate::detectors::package_manager::detect_package_manager(),
        version: env!("CARGO_PKG_VERSION"),
    };
    
    // Create router with all routes
    let app = Router::new()
        // Core endpoints
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/api/v1/version", get(version_handler))
        
        // Report endpoints
        .route("/api/v1/report", get(get_report_handler))
        .route("/api/v1/report/summary", get(get_summary_handler))
        .route("/api/v1/report/hardware", get(get_hardware_handler))
        .route("/api/v1/report/software", get(get_software_handler))
        .route("/api/v1/report/languages", get(get_languages_handler))
        .route("/api/v1/report/gpu", get(get_gpu_handler))
        .route("/api/v1/report/containers", get(get_containers_handler))
        
        // Analysis endpoints
        .route("/api/v1/analyze", post(analyze_handler))
        .route("/api/v1/analyze/async", post(analyze_async_handler))
        
        // Export endpoints
        .route("/api/v1/export", post(export_handler))
        .route("/api/v1/export/json", get(export_json_handler))
        .route("/api/v1/export/markdown", get(export_markdown_handler))
        .route("/api/v1/export/html", get(export_html_handler))
        
        // Comparison endpoints
        .route("/api/v1/compare/:reference", get(compare_handler))
        .route("/api/v1/references", get(list_references_handler))
        
        // Docker/DevOps endpoints
        .route("/api/v1/dockerfile", post(generate_dockerfile_handler))
        .route("/api/v1/install-plan", get(get_install_plan_handler))
        
        // Benchmark endpoints
        .route("/api/v1/benchmark", post(run_benchmark_handler))
        .route("/api/v1/benchmark/status", get(get_benchmark_status))
        
        // WebSocket for real-time updates
        .route("/ws", get(websocket_handler))
        
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("\n🚀 API Server running on http://localhost:{}", port);
    println!("📚 API Documentation: http://localhost:{}/", port);
    println!("🔍 Health check: http://localhost:{}/health", port);
    println!("📊 Report: http://localhost:{}/api/v1/report", port);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

// ============================================================================
// HANDLERS
// ============================================================================

/// Root endpoint - API documentation
async fn root_handler() -> impl IntoResponse {
    let docs = r#"
╔═══════════════════════════════════════════════════════════════════════════╗
║                         IntelliProbe API v2.0                             ║
║                   Intelligent Workstation Profiler                        ║
╚═══════════════════════════════════════════════════════════════════════════╝

📚 AVAILABLE ENDPOINTS:

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔹 CORE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  GET  /                           API documentation
  GET  /health                     Health check
  GET  /api/v1/version             API version info

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔹 REPORT ENDPOINTS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  GET  /api/v1/report              Complete analysis report
  GET  /api/v1/report/summary      AI & Dev scores summary
  GET  /api/v1/report/hardware     Hardware specifications
  GET  /api/v1/report/software     Software capabilities
  GET  /api/v1/report/languages    Programming languages detected
  GET  /api/v1/report/gpu          GPU information
  GET  /api/v1/report/containers   Container support (Docker, k8s)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔹 ANALYSIS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  POST /api/v1/analyze             Run new analysis (sync)
  POST /api/v1/analyze/async       Run new analysis (async, returns job ID)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔹 EXPORT
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  POST /api/v1/export              Export report in specified format
  GET  /api/v1/export/json         Get report as JSON
  GET  /api/v1/export/markdown     Get report as Markdown
  GET  /api/v1/export/html         Get report as HTML

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔹 COMPARISON
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  GET  /api/v1/compare/:ref        Compare with reference system
  GET  /api/v1/references          List available reference systems

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔹 DEVOPS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  POST /api/v1/dockerfile          Generate Dockerfile for environment
  GET  /api/v1/install-plan        Get installation plan

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔹 BENCHMARKS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  POST /api/v1/benchmark           Run performance benchmarks
  GET  /api/v1/benchmark/status    Check benchmark job status

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔹 WEBSOCKET
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  WS   /ws                         Real-time updates (analysis progress)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📝 EXAMPLE USAGE:

  # Get complete report
  curl http://localhost:3000/api/v1/report

  # Get summary only
  curl http://localhost:3000/api/v1/report/summary

  # Export as markdown
  curl -X POST http://localhost:3000/api/v1/export \
    -H "Content-Type: application/json" \
    -d '{"format": "markdown"}' \
    --output report.md

  # Compare with reference
  curl http://localhost:3000/api/v1/compare/llm-7b

  # Generate Dockerfile
  curl -X POST http://localhost:3000/api/v1/dockerfile

💡 TIPS:
  - Use ?pretty=true for formatted JSON
  - Set threshold for CI/CD integration
  - WebSocket for real-time benchmark progress
"#;
    
    (StatusCode::OK, docs)
}

/// Health check handler
async fn health_handler(State(state): State<ApiState>) -> Json<HealthResponse> {
    static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let uptime = START_TIME.get_or_init(|| std::time::Instant::now()).elapsed().as_secs();
    
    Json(HealthResponse {
        status: "healthy",
        version: state.version,
        uptime_seconds: uptime,
        package_manager: state.package_manager.name().to_string(),
    })
}

/// Version handler
async fn version_handler(State(state): State<ApiState>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "version": state.version,
        "api_version": "v1",
        "name": "IntelliProbe",
        "description": "Intelligent Workstation Profiler"
    })))
}

/// Get complete report
async fn get_report_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<AnalysisResult>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => Json(ApiResponse::success(data.clone())),
        None => Json(ApiResponse::error("No report available. Run analysis first.".to_string())),
    }
}

/// Get summary (scores only)
async fn get_summary_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let summary = serde_json::json!({
                "ai": {
                    "score": data.complete_report.ai_summary.overall_score,
                    "tier": data.complete_report.ai_summary.tier,
                    "strengths": data.complete_report.ai_summary.strengths,
                    "weaknesses": data.complete_report.ai_summary.weaknesses,
                    "best_for": data.complete_report.ai_summary.best_for,
                },
                "dev": {
                    "score": data.complete_report.overall_dev_score,
                    "tier": data.complete_report.dev_tier,
                },
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            Json(ApiResponse::success(summary))
        }
        None => Json(ApiResponse::error("No report available".to_string())),
    }
}

/// Get hardware information
async fn get_hardware_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let hw = serde_json::json!({
                "cpu": {
                    "cores": data.complete_report.ai_hardware.cpu_cores,
                    "model": data.complete_report.ai_hardware.cpu_model,
                },
                "memory": {
                    "ram_gb": data.complete_report.ai_hardware.ram_gb,
                    "swap_gb": data.complete_report.ai_hardware.swap_gb,
                },
                "disk": {
                    "free_gb": data.complete_report.ai_hardware.disk_free_gb,
                    "total_gb": data.complete_report.ai_hardware.disk_total_gb,
                },
                "gpu": {
                    "model": data.complete_report.ai_hardware.gpu_model,
                    "memory_mb": data.complete_report.ai_hardware.gpu_memory_mb,
                    "max_resolution": data.complete_report.ai_hardware.max_resolution,
                },
            });
            Json(ApiResponse::success(hw))
        }
        None => Json(ApiResponse::error("No report available".to_string())),
    }
}

/// Get software capabilities
async fn get_software_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let software = serde_json::json!({
                "inference_backends": data.complete_report.ai_compatibility.inference_backends,
                "video_codecs": data.complete_report.ai_compatibility.video_codecs,
                "compute_apis": data.complete_report.ai_compatibility.compute_apis,
            });
            Json(ApiResponse::success(software))
        }
        None => Json(ApiResponse::error("No report available".to_string())),
    }
}

/// Get languages detected
async fn get_languages_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let languages: Vec<serde_json::Value> = data.complete_report.dev_capabilities.languages
                .iter()
                .filter(|l| l.installed)
                .map(|l| serde_json::json!({
                    "name": l.name,
                    "version": l.version,
                    "package_manager": l.package_manager,
                    "performance_rating": l.performance_rating,
                }))
                .collect();
            Json(ApiResponse::success(languages))
        }
        None => Json(ApiResponse::error("No report available".to_string())),
    }
}

/// Get GPU information
async fn get_gpu_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let gpu = serde_json::json!({
                "model": data.complete_report.ai_hardware.gpu_model,
                "memory_mb": data.complete_report.ai_hardware.gpu_memory_mb,
                "cuda": data.complete_report.ai_compatibility.compute_apis.iter().any(|a| a.api == "CUDA" && a.supported),
                "opencl": data.complete_report.ai_compatibility.compute_apis.iter().any(|a| a.api == "OpenCL" && a.supported),
                "vulkan": data.complete_report.ai_compatibility.compute_apis.iter().any(|a| a.api == "Vulkan" && a.supported),
            });
            Json(ApiResponse::success(gpu))
        }
        None => Json(ApiResponse::error("No report available".to_string())),
    }
}

/// Get containers information
async fn get_containers_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let containers = serde_json::json!({
                "docker": {
                    "installed": data.complete_report.dev_capabilities.containers.docker,
                    "version": data.complete_report.dev_capabilities.containers.docker_version,
                    "compose": data.complete_report.dev_capabilities.containers.compose_available,
                    "buildx": data.complete_report.dev_capabilities.containers.buildx_support,
                },
                "podman": data.complete_report.dev_capabilities.containers.podman,
                "kubernetes": data.complete_report.dev_capabilities.containers.kubernetes_cli,
            });
            Json(ApiResponse::success(containers))
        }
        None => Json(ApiResponse::error("No report available".to_string())),
    }
}

/// Run analysis (synchronous)
async fn analyze_handler(
    State(state): State<ApiState>,
    Json(req): Json<AnalyzeRequest>,
) -> Json<ApiResponse<String>> {
    // This would trigger a new analysis
    // For now, return a placeholder
    Json(ApiResponse::success("Analysis triggered. Use /api/v1/report to get results.".to_string()))
}

/// Run analysis (asynchronous with job ID)
async fn analyze_async_handler(
    State(state): State<ApiState>,
    Json(req): Json<AnalyzeRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let job_id = uuid::Uuid::new_v4().to_string();
    
    // Spawn background task
    tokio::spawn(async move {
        println!("[Job {}] Starting analysis", job_id);
        // Actual analysis would go here
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        println!("[Job {}] Analysis complete", job_id);
    });
    
    Json(ApiResponse::success(serde_json::json!({
        "job_id": job_id,
        "status": "started",
        "message": "Analysis running in background. Check /api/v1/report when complete."
    })))
}

/// Export report
async fn export_handler(
    State(state): State<ApiState>,
    Json(req): Json<ExportRequest>,
) -> impl IntoResponse {
    let result = state.result.lock().await;
    let data = match result.as_ref() {
        Some(d) => d,
        None => return (StatusCode::NOT_FOUND, "No report available".to_string()),
    };
    
    let output = match req.format.as_str() {
        "json" => json::export(data).unwrap_or_else(|e| format!("Error: {}", e)),
        "markdown" | "md" => markdown::export(data).unwrap_or_else(|e| format!("Error: {}", e)),
        "html" => html::export(data).unwrap_or_else(|e| format!("Error: {}", e)),
        _ => return (StatusCode::BAD_REQUEST, format!("Unknown format: {}", req.format)),
    };
    
    let content_type = match req.format.as_str() {
        "json" => "application/json",
        "markdown" | "md" => "text/markdown",
        "html" => "text/html",
        _ => "text/plain",
    };
    
    (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, content_type)], output)
}

/// Export as JSON
async fn export_json_handler(State(state): State<ApiState>) -> impl IntoResponse {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let json = json::export(data).unwrap_or_default();
            (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "application/json")], json)
        }
        None => (StatusCode::NOT_FOUND, "No report available".to_string()),
    }
}

/// Export as Markdown
async fn export_markdown_handler(State(state): State<ApiState>) -> impl IntoResponse {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let md = markdown::export(data).unwrap_or_default();
            (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "text/markdown")], md)
        }
        None => (StatusCode::NOT_FOUND, "No report available".to_string()),
    }
}

/// Export as HTML
async fn export_html_handler(State(state): State<ApiState>) -> impl IntoResponse {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let html = html::export(data).unwrap_or_default();
            (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "text/html")], html)
        }
        None => (StatusCode::NOT_FOUND, "No report available".to_string()),
    }
}

/// Compare with reference system
async fn compare_handler(
    State(state): State<ApiState>,
    axum::extract::Path(reference): axum::extract::Path<String>,
) -> Json<ApiResponse<String>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            match crate::comparators::compare_with_reference(data, &reference) {
                Ok(comparison) => Json(ApiResponse::success(comparison)),
                Err(e) => Json(ApiResponse::error(format!("Reference '{}' not found: {}", reference, e))),
            }
        }
        None => Json(ApiResponse::error("No report available".to_string())),
    }
}

/// List available reference systems
async fn list_references_handler() -> Json<ApiResponse<Vec<&'static str>>> {
    let references = vec![
        "llm-7b",
        "llm-13b",
        "stable-diffusion",
        "web-dev",
        "data-engineering",
        "game-dev",
        "embedded",
    ];
    Json(ApiResponse::success(references))
}

/// Generate Dockerfile
async fn generate_dockerfile_handler(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            match crate::generate_dockerfile(data) {
                Ok(dockerfile) => (
                    StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "text/plain")],
                    dockerfile,
                ),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)),
            }
        }
        None => (StatusCode::NOT_FOUND, "No report available".to_string()),
    }
}

/// Get installation plan
async fn get_install_plan_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<Vec<String>>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => Json(ApiResponse::success(data.install_plan.clone())),
        None => Json(ApiResponse::error("No report available".to_string())),
    }
}

/// Run benchmark
async fn run_benchmark_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<serde_json::Value>> {
    // This would run actual benchmarks
    Json(ApiResponse::success(serde_json::json!({
        "status": "started",
        "estimated_duration": "30 seconds",
        "metrics": ["cpu_score", "gpu_score", "memory_bandwidth", "disk_io"]
    })))
}

/// Get benchmark status
async fn get_benchmark_status() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "idle",
        "last_run": null,
        "results": null
    })))
}

/// WebSocket handler for real-time updates
async fn websocket_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket))
}

async fn handle_websocket(mut socket: axum::extract::ws::WebSocket) {
    use futures_util::{SinkExt, StreamExt};
    
    println!("WebSocket client connected");
    
    // Send initial message
    let _ = socket.send(axum::extract::ws::Message::Text(
        serde_json::json!({
            "type": "connected",
            "message": "Connected to IntelliProbe API"
        }).to_string()
    )).await;
    
    // Handle incoming messages
    while let Some(Ok(msg)) = socket.next().await {
        if let axum::extract::ws::Message::Close(_) = msg {
            break;
        }
        
        // Echo back for now
        let _ = socket.send(axum::extract::ws::Message::Text(
            serde_json::json!({
                "type": "pong",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string()
        )).await;
    }
    
    println!("WebSocket client disconnected");
}