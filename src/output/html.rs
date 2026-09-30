//! Rapport HTML autonome (données intégrées, aucune ressource externe)
//! et mini-serveur HTTP pour le consulter depuis un autre poste.

use crate::report::Report;
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

const TEMPLATE: &str = include_str!("../web/dashboard.html");
const PLACEHOLDER: &str = "/*__REPORT_JSON__*/null";

pub fn render(r: &Report) -> String {
    // « </ » échappé pour ne pas fermer la balise <script> ; reste du JSON valide.
    let json = serde_json::to_string(r).unwrap_or_else(|_| "{}".into()).replace("</", "<\\/");
    TEMPLATE.replacen(PLACEHOLDER, &json, 1)
}

/// Sert le rapport : `/` (page) et `/report.json` (données brutes).
pub fn serve(r: &Report, host: &str, port: u16) -> Result<()> {
    let html = render(r);
    let json = serde_json::to_string_pretty(r)?;
    let listener = TcpListener::bind((host, port)).with_context(|| format!("impossible d'écouter sur {}:{}", host, port))?;
    println!("\n  Dashboard : http://{}:{}/   (Ctrl+C pour arrêter)", if host == "0.0.0.0" { "<ip-de-la-carte>" } else { host }, port);
    if host == "0.0.0.0" {
        println!("  Attention : le rapport est visible par toute machine du réseau local.");
    }
    for stream in listener.incoming() {
        let Ok(mut s) = stream else { continue };
        let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buf = [0u8; 4096];
        let n = s.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req.split_whitespace().nth(1).unwrap_or("/");
        let (status, ctype, body) = match path {
            "/" | "/index.html" => ("200 OK", "text/html; charset=utf-8", html.as_bytes()),
            "/report.json" => ("200 OK", "application/json; charset=utf-8", json.as_bytes()),
            _ => ("404 Not Found", "text/plain; charset=utf-8", "introuvable".as_bytes()),
        };
        let head = format!("HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", status, ctype, body.len());
        let _ = s.write_all(head.as_bytes()).and_then(|_| s.write_all(body));
    }
    Ok(())
}
