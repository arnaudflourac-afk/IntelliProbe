//! REST API routes

use crate::analyze::AnalysisResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    result: Arc<Mutex<Option<AnalysisResult>>>,
}

#[derive(Deserialize)]
pub struct AnalyzeRequest {
    pub path: Option<String>,
    pub benchmark: Option<bool>,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

pub async fn start_server(port: u16, result: AnalysisResult) -> anyhow::Result<()> {
    let state = AppState {
        result: Arc::new(Mutex::new(Some(result))),
    };
    
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/api/report", get(get_report))
        .route("/api/report/summary", get(get_summary))
        .route("/api/report/hardware", get(get_hardware))
        .route("/api/report/languages", get(get_languages))
        .route("/api/report/compare/:id", get(compare_with))
        .route("/api/analyze", post(analyze_system))
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("API server running on http://localhost:{}", port);
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn root() -> &'static str {
    "IntelliProbe API v2.0\n\nAvailable endpoints:\n- GET /health\n- GET /api/report\n- GET /api/report/summary\n- GET /api/report/hardware\n- GET /api/report/languages\n- GET /api/report/compare/:id\n- POST /api/analyze"
}

async fn health() -> Json<ApiResponse<String>> {
    Json(ApiResponse {
        success: true,
        data: Some("OK".to_string()),
        error: None,
    })
}

async fn get_report(State(state): State<AppState>) -> Json<ApiResponse<AnalysisResult>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => Json(ApiResponse {
            success: true,
            data: Some(data.clone()),
            error: None,
        }),
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("No report available".to_string()),
        }),
    }
}

async fn get_summary(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let summary = serde_json::json!({
                "ai_score": data.complete_report.ai_summary.overall_score,
                "ai_tier": data.complete_report.ai_summary.tier,
                "dev_score": data.complete_report.overall_dev_score,
                "dev_tier": data.complete_report.dev_tier,
                "strengths": data.complete_report.ai_summary.strengths,
                "weaknesses": data.complete_report.ai_summary.weaknesses,
                "best_for": data.complete_report.ai_summary.best_for,
            });
            Json(ApiResponse {
                success: true,
                data: Some(summary),
                error: None,
            })
        }
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("No report available".to_string()),
        }),
    }
}

async fn get_hardware(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let hardware = serde_json::json!({
                "cpu": {
                    "cores": data.complete_report.ai_hardware.cpu_cores,
                    "model": data.complete_report.ai_hardware.cpu_model,
                },
                "ram_gb": data.complete_report.ai_hardware.ram_gb,
                "gpu": {
                    "model": data.complete_report.ai_hardware.gpu_model,
                    "memory_mb": data.complete_report.ai_hardware.gpu_memory_mb,
                },
                "disk": {
                    "free_gb": data.complete_report.ai_hardware.disk_free_gb,
                    "total_gb": data.complete_report.ai_hardware.disk_total_gb,
                }
            });
            Json(ApiResponse {
                success: true,
                data: Some(hardware),
                error: None,
            })
        }
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("No report available".to_string()),
        }),
    }
}

async fn get_languages(State(state): State<AppState>) -> Json<ApiResponse<Vec<String>>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            let languages: Vec<String> = data.complete_report.dev_capabilities.languages
                .iter()
                .filter(|l| l.installed)
                .map(|l| l.name.clone())
                .collect();
            Json(ApiResponse {
                success: true,
                data: Some(languages),
                error: None,
            })
        }
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("No report available".to_string()),
        }),
    }
}

async fn compare_with(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Json<ApiResponse<String>> {
    let result = state.result.lock().await;
    match result.as_ref() {
        Some(data) => {
            // Simulate comparison with another system
            let comparison = format!(
                "Comparison with system '{}' not implemented yet.\nCurrent system: AI Score {}/100, Dev Score {}/100",
                id,
                data.complete_report.ai_summary.overall_score,
                data.complete_report.overall_dev_score
            );
            Json(ApiResponse {
                success: true,
                data: Some(comparison),
                error: None,
            })
        }
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("No report available".to_string()),
        }),
    }
}

async fn analyze_system(
    State(state): State<AppState>,
    Json(req): Json<AnalyzeRequest>,
) -> Json<ApiResponse<String>> {
    // Spawn background task for analysis
    tokio::spawn(async move {
        // Perform analysis here
        println!("Starting analysis for path: {:?}", req.path);
    });
    
    Json(ApiResponse {
        success: true,
        data: Some("Analysis started. Check /api/report in a few moments.".to_string()),
        error: None,
    })
}