//! Outils d'inférence présents (ollama, llama.cpp, TensorRT, Hailo, OpenVINO…)
//! et modèles déjà téléchargés.

use super::devtools::{check_tools, ToolDef};
use crate::report::{Ai, Tool};
use crate::util::*;
use std::path::Path;
use std::time::Duration;

const AI: &str = "IA";
const DIAG: &str = "Diagnostic accélérateurs";
const V: Option<&[&str]> = Some(&["--version"]);

const AI_TOOLS: &[ToolDef] = &[
    ToolDef { category: AI, name: "Ollama", commands: &["ollama"], version_args: V },
    ToolDef { category: AI, name: "llama.cpp (llama-cli)", commands: &["llama-cli"], version_args: V },
    ToolDef { category: AI, name: "llama.cpp (llama-server)", commands: &["llama-server"], version_args: V },
    ToolDef { category: AI, name: "llama.cpp (llama-bench)", commands: &["llama-bench"], version_args: None },
    ToolDef { category: AI, name: "whisper.cpp", commands: &["whisper-cli", "whisper-cpp"], version_args: None },
    ToolDef { category: AI, name: "vLLM", commands: &["vllm"], version_args: V },
    ToolDef { category: AI, name: "LM Studio CLI", commands: &["lms"], version_args: None },
    ToolDef { category: AI, name: "Hugging Face CLI", commands: &["huggingface-cli", "hf"], version_args: None },
    ToolDef { category: AI, name: "HailoRT CLI", commands: &["hailortcli"], version_args: V },
    ToolDef { category: AI, name: "Coral Edge TPU compiler", commands: &["edgetpu_compiler"], version_args: V },
    ToolDef { category: AI, name: "RKNN server (debug NPU)", commands: &["rknn_server"], version_args: None },
    ToolDef { category: AI, name: "OpenVINO benchmark_app", commands: &["benchmark_app"], version_args: None },
    ToolDef { category: AI, name: "OpenVINO ovc (conversion)", commands: &["ovc"], version_args: V },
    ToolDef { category: AI, name: "TFLite benchmark_model", commands: &["benchmark_model"], version_args: None },
    ToolDef { category: AI, name: "ONNX Runtime perf test", commands: &["onnxruntime_perf_test"], version_args: None },
    ToolDef { category: DIAG, name: "nvidia-smi", commands: &["nvidia-smi"], version_args: None },
    ToolDef { category: DIAG, name: "rocm-smi", commands: &["rocm-smi"], version_args: None },
    ToolDef { category: DIAG, name: "rocminfo", commands: &["rocminfo"], version_args: None },
    ToolDef { category: DIAG, name: "clinfo (OpenCL)", commands: &["clinfo"], version_args: None },
    ToolDef { category: DIAG, name: "vulkaninfo", commands: &["vulkaninfo"], version_args: None },
    ToolDef { category: DIAG, name: "vainfo (VA-API)", commands: &["vainfo"], version_args: None },
    ToolDef { category: DIAG, name: "jetson_release", commands: &["jetson_release"], version_args: None },
];

pub fn probe() -> (Ai, Vec<Tool>) {
    let (mut runtimes, missing) = check_tools(AI_TOOLS);

    // trtexec n'est généralement pas dans le PATH (Jetson, paquets TensorRT).
    for p in ["/usr/src/tensorrt/bin/trtexec", "/usr/local/tensorrt/bin/trtexec"] {
        if Path::new(p).is_file() && !runtimes.iter().any(|t| t.command == "trtexec") {
            runtimes.push(Tool { category: AI.into(), name: "TensorRT trtexec".into(), command: "trtexec".into(), path: Some(p.into()), version: None });
        }
    }

    let mut ai = Ai { runtimes, ..Default::default() };
    if ai.runtimes.iter().any(|t| t.command == "ollama") {
        ai.ollama_models = ollama_models();
    }
    (ai, missing)
}

/// `ollama list` (nécessite le service ollama démarré).
fn ollama_models() -> Vec<String> {
    let Some(out) = run_timeout("ollama", &["list"], Duration::from_secs(5)).filter(|o| o.success) else { return Vec::new() };
    out.stdout
        .lines()
        .skip(1)
        .filter_map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            // NAME  ID  SIZE(2 colonnes : « 4.7 GB »)  MODIFIED…
            match cols.as_slice() {
                [name, _id, size, unit, ..] => Some(format!("{} ({} {})", name, size, unit)),
                [name, ..] => Some(name.to_string()),
                [] => None,
            }
        })
        .collect()
}
