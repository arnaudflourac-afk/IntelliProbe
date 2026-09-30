//! Python : interpréteur, inventaire complet des paquets, et test réel des
//! frameworks IA (import dans un sous-processus isolé + accélérateurs vus).

use crate::report::{Framework, Package, Python};
use crate::util::*;
use serde::Deserialize;
use std::collections::HashSet;
use std::time::Duration;

const INVENTORY: &str = r#"
import sys, json, os, importlib.util
out = {"version": sys.version.split()[0], "executable": sys.executable, "packages": [], "modules": []}
venv = sys.prefix if sys.prefix != getattr(sys, "base_prefix", sys.prefix) else None
out["venv"] = venv or os.environ.get("CONDA_DEFAULT_ENV")
pkgs = {}
try:
    from importlib import metadata
    dists = []
    for d in metadata.distributions():
        try:
            dists.append((d.metadata["Name"], d.version, d.metadata["Summary"]))
        except Exception:
            pass
except Exception:
    import pkg_resources
    dists = [(d.project_name, d.version, None) for d in pkg_resources.working_set]
for name, version, summary in dists:
    if not name or name.lower() in pkgs:
        continue
    pkgs[name.lower()] = {"name": name, "version": version, "description": (summary or "")[:200] or None}
out["packages"] = sorted(pkgs.values(), key=lambda p: p["name"].lower())
for m in MODULES:
    try:
        if importlib.util.find_spec(m) is not None:
            out["modules"].append(m)
    except Exception:
        pass
print(json.dumps(out))
"#;

const WRAPPER_HEAD: &str = "import json, sys, os, warnings\nwarnings.filterwarnings('ignore')\nR = {'version': None, 'acc': [], 'details': []}\ntry:\n";
const WRAPPER_TAIL: &str = "\n    R['ok'] = True\nexcept BaseException as e:\n    R['ok'] = False\n    R['error'] = (type(e).__name__ + ': ' + str(e))[:300]\nsys.stdout.write('\\n__IPJSON__' + json.dumps(R, default=str) + '\\n')\n";

struct FrameworkDef {
    name: &'static str,
    dists: &'static [&'static str],
    module: &'static str,
    code: &'static str,
}

const FRAMEWORKS: &[FrameworkDef] = &[
    FrameworkDef {
        name: "PyTorch",
        dists: &["torch"],
        module: "torch",
        code: r#"
import torch
R["version"] = torch.__version__
if torch.cuda.is_available():
    for i in range(torch.cuda.device_count()):
        p = torch.cuda.get_device_properties(i)
        R["acc"].append("cuda:%d %s (%.1f Go)" % (i, p.name, p.total_memory / 1e9))
mps = getattr(torch.backends, "mps", None)
if mps is not None and mps.is_available():
    R["acc"].append("mps")
xpu = getattr(torch, "xpu", None)
if xpu is not None and xpu.is_available():
    R["acc"].append("xpu")
R["details"].append("build CUDA : %s" % torch.version.cuda)
if getattr(torch.version, "hip", None):
    R["details"].append("build ROCm/HIP : %s" % torch.version.hip)
if torch.backends.cudnn.is_available():
    R["details"].append("cuDNN : %s" % torch.backends.cudnn.version())
R["details"].append("threads CPU : %d" % torch.get_num_threads())
"#,
    },
    FrameworkDef {
        name: "TensorFlow",
        dists: &["tensorflow", "tensorflow-cpu", "tensorflow-aarch64", "tf-nightly", "tensorflow-gpu"],
        module: "tensorflow",
        code: r#"
import tensorflow as tf
R["version"] = tf.__version__
for d in tf.config.list_physical_devices():
    if d.device_type != "CPU":
        R["acc"].append("%s %s" % (d.device_type, d.name))
R["details"].append("compilé avec CUDA : %s" % tf.test.is_built_with_cuda())
"#,
    },
    FrameworkDef {
        name: "TFLite runtime",
        dists: &["tflite-runtime"],
        module: "tflite_runtime",
        code: r#"
import tflite_runtime
from tflite_runtime import interpreter
R["version"] = getattr(tflite_runtime, "__version__", None)
"#,
    },
    FrameworkDef {
        name: "LiteRT (ex-TFLite)",
        dists: &["ai-edge-litert"],
        module: "ai_edge_litert",
        code: r#"
import ai_edge_litert
from ai_edge_litert import interpreter
R["version"] = getattr(ai_edge_litert, "__version__", None)
"#,
    },
    FrameworkDef {
        name: "ONNX Runtime",
        dists: &["onnxruntime", "onnxruntime-gpu", "onnxruntime-openvino", "onnxruntime-qnn", "onnxruntime-rocm", "onnxruntime-directml"],
        module: "onnxruntime",
        code: r#"
import onnxruntime as ort
R["version"] = ort.__version__
providers = ort.get_available_providers()
# Azure = exécution distante, pas un accélérateur local.
R["acc"] = [p for p in providers if p not in ("CPUExecutionProvider", "AzureExecutionProvider")]
R["details"].append("providers : " + ", ".join(providers))
"#,
    },
    FrameworkDef {
        name: "OpenVINO",
        dists: &["openvino"],
        module: "openvino",
        code: r#"
try:
    import openvino as ov
    core = ov.Core()
    R["version"] = ov.get_version()
except (ImportError, AttributeError):
    from openvino.runtime import Core, get_version
    core = Core()
    R["version"] = get_version()
for d in core.available_devices:
    try:
        name = core.get_property(d, "FULL_DEVICE_NAME")
    except Exception:
        name = ""
    if d == "CPU":
        R["details"].append("CPU : %s" % name)
    else:
        R["acc"].append("%s %s" % (d, name))
"#,
    },
    FrameworkDef {
        name: "JAX",
        dists: &["jax"],
        module: "jax",
        code: r#"
import jax
R["version"] = jax.__version__
for d in jax.devices():
    if d.platform != "cpu":
        R["acc"].append(str(d))
R["details"].append("backend par défaut : %s" % jax.default_backend())
"#,
    },
    FrameworkDef {
        name: "RKNN Toolkit Lite 2 (NPU Rockchip)",
        dists: &["rknn-toolkit-lite2", "rknn_toolkit_lite2"],
        module: "rknnlite",
        code: r#"
from rknnlite.api import RKNNLite
R["details"].append("RKNNLite importable : inférence NPU possible depuis Python")
"#,
    },
    FrameworkDef {
        name: "RKNN Toolkit 2 (conversion de modèles)",
        dists: &["rknn-toolkit2"],
        module: "rknn",
        code: r#"
from rknn.api import RKNN
R["details"].append("RKNN importable : conversion ONNX/TFLite -> .rknn possible")
"#,
    },
    FrameworkDef {
        name: "HailoRT",
        dists: &["hailort", "hailo-platform", "hailo_platform"],
        module: "hailo_platform",
        code: r#"
import hailo_platform
from hailo_platform import Device
R["version"] = getattr(hailo_platform, "__version__", None)
try:
    R["acc"] = [str(d) for d in Device.scan()]
except Exception as e:
    R["details"].append("scan : %s" % e)
"#,
    },
    FrameworkDef {
        name: "PyCoral (Edge TPU)",
        dists: &["pycoral"],
        module: "pycoral",
        code: r#"
from pycoral.utils.edgetpu import list_edge_tpus
R["acc"] = ["%s %s" % (t.get("type"), t.get("path")) for t in list_edge_tpus()]
"#,
    },
    FrameworkDef {
        name: "TensorRT",
        dists: &["tensorrt"],
        module: "tensorrt",
        code: r#"
import tensorrt as trt
R["version"] = trt.__version__
"#,
    },
    FrameworkDef {
        name: "CuPy",
        dists: &["cupy", "cupy-cuda12x", "cupy-cuda11x"],
        module: "cupy",
        code: r#"
import cupy
R["version"] = cupy.__version__
for i in range(cupy.cuda.runtime.getDeviceCount()):
    n = cupy.cuda.runtime.getDeviceProperties(i)["name"]
    R["acc"].append("cuda:%d %s" % (i, n.decode() if isinstance(n, bytes) else n))
"#,
    },
    FrameworkDef {
        name: "OpenCV",
        dists: &["opencv-python", "opencv-contrib-python", "opencv-python-headless", "opencv-contrib-python-headless"],
        module: "cv2",
        code: r#"
import cv2
R["version"] = cv2.__version__
try:
    n = cv2.cuda.getCudaEnabledDeviceCount()
    if n:
        R["acc"].append("CUDA x%d" % n)
except Exception:
    pass
if cv2.ocl.haveOpenCL():
    R["acc"].append("OpenCL")
info = cv2.getBuildInformation().splitlines()
for key in ("NVIDIA CUDA", "OpenCL", "GStreamer", "FFMPEG", "v4l/v4l2", "Parallel framework", "Inference Engine"):
    for line in info:
        if line.strip().lower().startswith(key.lower() + ":"):
            R["details"].append(" ".join(line.split()))
            break
"#,
    },
    FrameworkDef {
        name: "NumPy",
        dists: &["numpy"],
        module: "numpy",
        code: r#"
import numpy as np
R["version"] = np.__version__
blas = None
try:
    b = np.show_config(mode="dicts").get("Build Dependencies", {}).get("blas", {})
    blas = "%s %s" % (b.get("name"), b.get("version", ""))
except Exception:
    pass
if blas:
    R["details"].append("BLAS : %s" % blas.strip())
"#,
    },
    FrameworkDef {
        name: "llama-cpp-python",
        dists: &["llama-cpp-python", "llama_cpp_python"],
        module: "llama_cpp",
        code: r#"
import llama_cpp
R["version"] = getattr(llama_cpp, "__version__", None)
try:
    R["details"].append("offload GPU supporté : %s" % llama_cpp.llama_supports_gpu_offload())
except Exception:
    pass
"#,
    },
    FrameworkDef {
        name: "Ultralytics (YOLO)",
        dists: &["ultralytics"],
        module: "ultralytics",
        code: "\nimport ultralytics\nR['version'] = ultralytics.__version__\n",
    },
    FrameworkDef {
        name: "Transformers",
        dists: &["transformers"],
        module: "transformers",
        code: "\nimport transformers\nR['version'] = transformers.__version__\n",
    },
    FrameworkDef {
        name: "MediaPipe",
        dists: &["mediapipe"],
        module: "mediapipe",
        code: "\nimport mediapipe\nR['version'] = mediapipe.__version__\n",
    },
];

#[derive(Deserialize)]
struct Inventory {
    version: Option<String>,
    executable: String,
    venv: Option<String>,
    packages: Vec<Package>,
    modules: Vec<String>,
}

#[derive(Deserialize, Default)]
struct ProbeResult {
    version: Option<String>,
    #[serde(default)]
    acc: Vec<String>,
    #[serde(default)]
    details: Vec<String>,
    #[serde(default)]
    ok: bool,
    error: Option<String>,
}

pub fn probe(test_frameworks: bool, log: &dyn Fn(&str)) -> Option<Python> {
    let exe = find_exe("python3").or_else(|| find_exe("python"))?.to_string_lossy().into_owned();
    let workdir = std::env::temp_dir();
    let modules: Vec<String> = FRAMEWORKS.iter().map(|f| format!("{:?}", f.module)).collect();
    let script = INVENTORY.replace("MODULES", &format!("[{}]", modules.join(", ")));
    let out = run_with(&exe, &["-c", &script], Duration::from_secs(60), &[], Some(&workdir))?;
    let inv: Inventory = serde_json::from_str(out.stdout.lines().last()?).ok()?;

    let mut py = Python { executable: inv.executable, version: inv.version, virtualenv: inv.venv, packages: inv.packages, frameworks: Vec::new() };
    let norm = |s: &str| s.to_lowercase().replace('_', "-");
    let dists: HashSet<String> = py.packages.iter().map(|p| norm(&p.name)).collect();
    let modules: HashSet<String> = inv.modules.into_iter().collect();

    for f in FRAMEWORKS {
        let dist = f.dists.iter().find(|d| dists.contains(&norm(d)));
        if dist.is_none() && !modules.contains(f.module) {
            continue;
        }
        let pkg_version = dist.and_then(|d| py.packages.iter().find(|p| norm(&p.name) == norm(d))).and_then(|p| p.version.clone());
        let mut fw = Framework { name: f.name.into(), package: dist.map(|d| d.to_string()).unwrap_or_else(|| format!("{} (module système)", f.module)), version: pkg_version, ..Default::default() };
        if test_frameworks {
            log(&format!("import {}", f.module));
            let r = run_framework(&exe, f.code, &workdir);
            fw.import_ok = r.ok;
            fw.version = r.version.or(fw.version);
            fw.accelerators = r.acc;
            fw.details = r.details;
            fw.error = r.error;
        }
        py.frameworks.push(fw);
    }
    Some(py)
}

fn run_framework(exe: &str, code: &str, workdir: &std::path::Path) -> ProbeResult {
    let body: String = code.lines().map(|l| format!("    {}\n", l)).collect();
    let script = format!("{}{}{}", WRAPPER_HEAD, body, WRAPPER_TAIL);
    let envs = [("TF_CPP_MIN_LOG_LEVEL", "3"), ("PYTHONWARNINGS", "ignore"), ("YOLO_OFFLINE", "1")];
    match run_with(exe, &["-c", &script], Duration::from_secs(180), &envs, Some(workdir)) {
        None => ProbeResult { error: Some("délai dépassé (180 s) ou interpréteur introuvable".into()), ..Default::default() },
        Some(o) => o
            .stdout
            .lines()
            .find_map(|l| l.strip_prefix("__IPJSON__"))
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_else(|| ProbeResult {
                // Crash natif (segfault, instruction illégale…) : on remonte la fin de stderr.
                error: Some(format!("le processus s'est arrêté : {}", o.stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("sans message").chars().take(300).collect::<String>())),
                ..Default::default()
            }),
    }
}
