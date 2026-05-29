//! JSON exporter

use crate::analyze::AnalysisResult;
use anyhow::Result;

pub fn export(result: &AnalysisResult) -> Result<String> {
    Ok(serde_json::to_string_pretty(result)?)
}