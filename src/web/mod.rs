//! Web dashboard module - NE COMPILE QUE SI FEATURE "web" EST ACTIVE

#![cfg(feature = "web")]

use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::fs;
use std::net::SocketAddr;

pub async fn start_dashboard() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        .route("/data", get(data));

    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                    🚀 DASHBOARD READY 🚀                    ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║                                                            ║");
    println!("║     🌐 Open in your browser: http://{}              ║", addr);
    println!("║                                                            ║");
    println!("║     📊 Dashboard features:                                 ║");
    println!("║        • AI & Dev Scores                                   ║");
    println!("║        • Hardware analysis                                 ║");
    println!("║        • Language detection                                ║");
    println!("║        • Code snippets                                     ║");
    println!("║                                                            ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // La méthode simple et recommandée avec axum 0.7
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index() -> impl IntoResponse {
    match fs::read_to_string("src/web/dashboard.html") {
        Ok(html) => Html(html).into_response(),
        Err(_) => Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>IntelliProbe Dashboard</title>
    <style>
        body { font-family: monospace; margin: 40px; background: #0a0a0f; color: #fff; }
        h1 { color: #00eaff; }
        pre { background: #12121a; padding: 20px; border-radius: 8px; overflow-x: auto; }
        .error { color: #ff4444; }
    </style>
</head>
<body>
    <h1>🚀 IntelliProbe Dashboard</h1>
    <div class="error">⚠️ dashboard.html not found at src/web/dashboard.html</div>
    <p>Please make sure the file exists.</p>
    <pre>cat analysis_result.json | jq .</pre>
</body>
</html>
"#).into_response(),
    }
}

async fn data() -> impl IntoResponse {
    match fs::read_to_string("output/analysis_result.json") {
        Ok(json) => json,
        Err(_) => "{}".to_string(),
    }
}