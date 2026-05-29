//! Exporters module for multiple output formats

pub mod json;
pub mod markdown;
pub mod html;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ExportFormat {
    Json,
    Markdown,
    Html,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::Json => write!(f, "json"),
            ExportFormat::Markdown => write!(f, "md"),
            ExportFormat::Html => write!(f, "html"),
        }
    }
}