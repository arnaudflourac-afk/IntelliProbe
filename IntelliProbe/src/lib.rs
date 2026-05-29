pub mod analyze;

#[cfg(feature = "web")]
pub mod web;

// Re-exports
pub use analyze::AnalysisResult;
pub use analyze::CompleteSystemReport;