//! Detectors module for multi-platform support

pub mod package_manager;
pub mod environment;

pub use package_manager::{PackageManager, detect_package_manager, get_distribution_name};
pub use environment::{Environment, detect_environment, get_shell, get_terminal};