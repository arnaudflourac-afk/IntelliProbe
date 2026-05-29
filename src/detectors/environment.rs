//! Development environment detection

use std::path::Path;

#[derive(Debug, Clone)]
pub struct Environment {
    pub shell: String,
    pub terminal: String,
    pub user_home: String,
    pub path_dirs: Vec<String>,
    pub env_vars: Vec<(String, String)>,
}

pub fn detect_environment() -> Environment {
    Environment {
        shell: get_shell(),
        terminal: get_terminal(),
        user_home: get_user_home(),
        path_dirs: get_path_dirs(),
        env_vars: get_important_env_vars(),
    }
}

pub fn get_shell() -> String {
    std::env::var("SHELL")
        .or_else(|_| std::env::var("COMSPEC"))
        .unwrap_or_else(|_| "/bin/sh".to_string())
}

pub fn get_terminal() -> String {
    std::env::var("TERM")
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn get_user_home() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string())
}

pub fn get_path_dirs() -> Vec<String> {
    if let Ok(path) = std::env::var("PATH") {
        path.split(':').map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    }
}

pub fn get_important_env_vars() -> Vec<(String, String)> {
    let important_vars = [
        "PATH", "HOME", "USER", "LANG", "LC_ALL", "EDITOR", "VISUAL",
        "PYTHONPATH", "NODE_PATH", "RUSTUP_HOME", "CARGO_HOME", "GOPATH",
    ];
    
    important_vars
        .iter()
        .filter_map(|&var| {
            std::env::var(var)
                .ok()
                .map(|value| (var.to_string(), value))
        })
        .collect()
}

pub fn check_ide_detection() -> Vec<String> {
    let mut ides = Vec::new();
    
    // Check for common IDEs
    let ide_commands = vec![
        ("code", "VS Code"),
        ("idea", "IntelliJ IDEA"),
        ("pycharm", "PyCharm"),
        ("rustrover", "RustRover"),
        ("clion", "CLion"),
        ("webstorm", "WebStorm"),
        ("goland", "GoLand"),
        ("vim", "Vim"),
        ("nvim", "Neovim"),
        ("emacs", "Emacs"),
        ("subl", "Sublime Text"),
        ("atom", "Atom"),
    ];
    
    for (cmd, name) in ide_commands {
        if which::which(cmd).is_ok() {
            ides.push(name.to_string());
        }
    }
    
    ides
}