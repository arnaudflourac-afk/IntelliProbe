//! HTML exporter

use crate::analyze::AnalysisResult;
use anyhow::Result;

pub fn export(result: &AnalysisResult) -> Result<String> {
    let mut html = String::new();
    
    html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SysProbe Report</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background: #f5f5f5;
        }
        .card {
            background: white;
            border-radius: 8px;
            padding: 20px;
            margin-bottom: 20px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        .card h2 {
            margin-top: 0;
            color: #2c3e50;
            border-bottom: 2px solid #3498db;
            padding-bottom: 10px;
        }
        .score {
            font-size: 2em;
            font-weight: bold;
            text-align: center;
            padding: 20px;
            border-radius: 8px;
        }
        .score-high { background: #27ae60; color: white; }
        .score-mid { background: #f39c12; color: white; }
        .score-entry { background: #e67e22; color: white; }
        .score-edge { background: #95a5a6; color: white; }
        .badge {
            display: inline-block;
            padding: 4px 12px;
            margin: 4px;
            border-radius: 16px;
            font-size: 0.85em;
            background: #ecf0f1;
        }
        .badge-success { background: #27ae60; color: white; }
        .badge-warning { background: #e74c3c; color: white; }
        table {
            width: 100%;
            border-collapse: collapse;
        }
        th, td {
            padding: 10px;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }
        th {
            background: #34495e;
            color: white;
        }
        tr:hover {
            background: #f5f5f5;
        }
        code {
            background: #f4f4f4;
            padding: 2px 6px;
            border-radius: 4px;
            font-family: 'Courier New', monospace;
        }
        pre {
            background: #2c3e50;
            color: #ecf0f1;
            padding: 15px;
            border-radius: 8px;
            overflow-x: auto;
        }
    </style>
</head>
<body>
"#);
    
    // Scores
    let ai_score = result.complete_report.ai_summary.overall_score;
    let ai_tier = result.complete_report.ai_summary.tier.to_lowercase();
    let dev_score = result.complete_report.overall_dev_score;
    let dev_tier = result.complete_report.dev_tier.to_lowercase();
    
    html.push_str(&format!(r#"
<div class="card">
    <h1>🤖 SysProbe System Analysis Report</h1>
    <p><strong>Generated:</strong> {}</p>
</div>

<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px;">
    <div class="card">
        <h2>🎯 AI Score</h2>
        <div class="score score-{}">{}/100</div>
        <p>Tier: <strong>{}</strong></p>
    </div>
    <div class="card">
        <h2>🛠️ Dev Score</h2>
        <div class="score score-{}">{}/100</div>
        <p>Tier: <strong>{}</strong></p>
    </div>
</div>
"#, 
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        ai_tier, ai_score, result.complete_report.ai_summary.tier,
        dev_tier, dev_score, result.complete_report.dev_tier
    ));
    
    // Hardware
    html.push_str(r#"
<div class="card">
    <h2>💻 Hardware Specifications</h2>
    <table>
        <tr><th>Component</th><th>Details</th></tr>
"#);
    
    let hw = &result.complete_report.ai_hardware;
    html.push_str(&format!(r#"
        <tr><td>CPU</td><td>{} cores ({})</td></tr>
        <tr><td>RAM</td><td>{:.1} GB</td></tr>
        <tr><td>GPU</td><td>{}</td></tr>
        <tr><td>GPU Memory</td><td>{}</td></tr>
        <tr><td>Disk</td><td>{:.0} GB free / {:.0} GB total</td></tr>
    </table>
</div>
"#,
        hw.cpu_cores,
        hw.cpu_model.as_deref().unwrap_or("Unknown"),
        hw.ram_gb,
        hw.gpu_model.as_deref().unwrap_or("None"),
        hw.gpu_memory_mb.map(|m| format!("{} MB", m)).unwrap_or_else(|| "Shared".to_string()),
        hw.disk_free_gb,
        hw.disk_total_gb
    ));
    
    // Languages
    html.push_str(r#"
<div class="card">
    <h2>📝 Programming Languages</h2>
    <table>
        <tr><th>Language</th><th>Version</th><th>Package Manager</th></tr>
"#);
    
    for lang in &result.complete_report.dev_capabilities.languages {
        if lang.installed {
            html.push_str(&format!(r#"
        <tr>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
        </tr>
"#,
                lang.name,
                lang.version.as_deref().unwrap_or("N/A"),
                lang.package_manager.as_deref().unwrap_or("N/A")
            ));
        }
    }
    
    html.push_str(r#"
    </table>
</div>
"#);
    
    // Recommendations
    html.push_str(r#"
<div class="card">
    <h2>🎯 Recommendations</h2>
    <h3>AI Workloads</h3>
    <div>
"#);
    
    for rec in &result.complete_report.ai_recommendations.generative_ai {
        html.push_str(&format!(r#"<span class="badge badge-success">{}</span>"#, rec));
    }
    
    html.push_str(r#"</div>
    <h3>Development Workloads</h3>
    <div>
"#);
    
    for rec in &result.complete_report.dev_recommendations.web_development {
        html.push_str(&format!(r#"<span class="badge">{}</span>"#, rec));
    }
    
    html.push_str(r#"</div>
</div>
"#);
    
    // Installation plan
    if !result.install_plan.is_empty() {
        html.push_str(r#"
<div class="card">
    <h2>📦 Installation Plan</h2>
    <pre>"#);
        
        for cmd in &result.install_plan {
            html.push_str(&format!("{}\n", cmd));
        }
        
        html.push_str(r#"</pre>
</div>
"#);
    }
    
    html.push_str(r#"
</body>
</html>
"#);
    
    Ok(html)
}