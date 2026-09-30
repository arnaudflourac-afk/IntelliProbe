//! Analyse : conclusions tirées uniquement des données mesurées.
//! - voies d'accélération IA (matériel / driver / runtime / bindings)
//! - constats (points forts, manques, pièges)
//! - estimation LLM calculée à partir de la RAM et de la bande passante mesurées

use crate::probes::libs::has_lib;
use crate::report::*;

pub fn analyze(r: &mut Report) {
    fill_npu_runtimes(r);
    r.ai.paths = accel_paths(r);
    r.analysis.llm = llm_estimate(r);
    r.analysis.findings = findings(r);
}

// ---------------------------------------------------------------- Runtimes NPU

/// (fabricant, librairies, paquets Python) du runtime de chaque famille de NPU.
const NPU_RUNTIMES: &[(&str, &[&str], &[&str])] = &[
    ("Rockchip", &["librknnrt.so", "librknn_api.so", "librkllmrt.so"], &["rknn-toolkit-lite2", "rknn-toolkit2"]),
    ("Hailo", &["libhailort.so"], &["hailort", "hailo-platform"]),
    ("Google", &["libedgetpu.so"], &["pycoral", "tflite-runtime"]),
    ("VeriSilicon", &["libOpenVX.so", "libtim-vx.so", "libvx_delegate.so"], &[]),
    ("Intel", &["libopenvino.so"], &["openvino"]),
    ("AMD", &["libxrt_coreutil.so"], &["onnxruntime-vitisai", "ryzen-ai"]),
    ("NVIDIA", &["libnvinfer.so"], &["tensorrt"]),
    ("ARM", &["libethosu"], &["ethos-u-vela"]),
    ("Axelera", &["libaxruntime.so"], &["axelera-runtime"]),
];

fn python_has(r: &Report, pkg: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().replace('_', "-");
    r.python.as_ref().is_some_and(|p| p.packages.iter().any(|x| norm(&x.name) == norm(pkg)))
}

fn fill_npu_runtimes(r: &mut Report) {
    let found: Vec<Vec<String>> = r
        .npus
        .iter()
        .map(|n| {
            let Some((_, libs, pkgs)) = NPU_RUNTIMES.iter().find(|(v, _, _)| n.vendor.as_deref() == Some(*v)) else { return Vec::new() };
            let mut f: Vec<String> = libs.iter().filter(|l| has_lib(l)).map(|l| l.to_string()).collect();
            f.extend(pkgs.iter().filter(|p| python_has(r, p)).map(|p| format!("python: {}", p)));
            f
        })
        .collect();
    for (n, f) in r.npus.iter_mut().zip(found) {
        n.runtime_found = f;
    }
}

// ---------------------------------------------------------------- Voies d'accélération

fn fw_with_acc(r: &Report, pred: impl Fn(&str) -> bool) -> Vec<String> {
    r.python
        .iter()
        .flat_map(|p| p.frameworks.iter())
        .filter(|f| f.import_ok && f.accelerators.iter().any(|a| pred(a)))
        .map(|f| format!("{} {}", f.name, f.version.as_deref().unwrap_or("")).trim().to_string())
        .collect()
}

fn status(hw: bool, driver: bool, runtime: bool) -> String {
    match (hw, driver, runtime) {
        (true, true, true) => "utilisable",
        (true, _, _) => "partiel",
        _ => "absent",
    }
    .into()
}

fn accel_paths(r: &Report) -> Vec<AccelPath> {
    let mut paths = Vec::new();

    // CPU : toujours présent. On liste ce qui tourne réellement dessus.
    let mut cpu_bind: Vec<String> = r
        .python
        .iter()
        .flat_map(|p| p.frameworks.iter())
        .filter(|f| f.import_ok && !matches!(f.name.as_str(), "NumPy" | "Transformers" | "Ultralytics (YOLO)"))
        .map(|f| f.name.clone())
        .collect();
    cpu_bind.extend(r.ai.runtimes.iter().filter(|t| t.category == "IA" && (t.command.starts_with("llama") || t.command == "ollama")).map(|t| t.name.clone()));
    let simd: Vec<&str> = r.cpu.isa_features.iter().map(|f| f.flag.as_str()).collect();
    paths.push(AccelPath {
        name: "CPU".into(),
        hardware: true,
        driver: true,
        runtime: !cpu_bind.is_empty(),
        bindings: cpu_bind.clone(),
        status: if cpu_bind.is_empty() { "partiel".into() } else { "utilisable".into() },
        detail: format!("{} cœurs logiques ; extensions : {}", r.cpu.logical_cores, if simd.is_empty() { "aucune détectée".into() } else { simd.join(", ") }),
    });

    // CUDA (NVIDIA dédié ou Jetson).
    let nvidia = r.gpus.iter().any(|g| g.vendor.as_deref() == Some("NVIDIA"));
    if nvidia || r.compute.cuda.is_some() {
        let cuda = r.compute.cuda.clone().unwrap_or_default();
        let driver = cuda.driver_version.is_some() || r.board.jetson.is_some() || has_lib("libcuda.so");
        let runtime = cuda.toolkit_version.is_some() || has_lib("libcudart.so");
        let bindings = fw_with_acc(r, |a| a.contains("cuda") || a.contains("CUDA") || a.starts_with("GPU"));
        paths.push(AccelPath {
            name: "NVIDIA CUDA".into(),
            hardware: nvidia,
            driver,
            runtime: runtime || !bindings.is_empty(),
            status: status(nvidia, driver, runtime || !bindings.is_empty()),
            detail: format!(
                "driver {} (CUDA max {}), toolkit {}, cuDNN {}, TensorRT {}",
                cuda.driver_version.as_deref().unwrap_or("?"),
                cuda.driver_cuda_version.as_deref().unwrap_or("?"),
                cuda.toolkit_version.as_deref().unwrap_or("absent"),
                cuda.cudnn.as_deref().unwrap_or("absent"),
                cuda.tensorrt.as_deref().unwrap_or("absent")
            ),
            bindings,
        });
    }

    // ROCm (AMD).
    let amd = r.gpus.iter().any(|g| g.vendor.as_deref() == Some("AMD"));
    if amd || r.compute.rocm_version.is_some() {
        let driver = r.gpus.iter().any(|g| g.driver.as_deref() == Some("amdgpu"));
        let runtime = r.compute.rocm_version.is_some() || has_lib("libamdhip64.so");
        let bindings = fw_with_acc(r, |a| a.contains("cuda") || a.contains("ROCM") || a.contains("rocm"));
        paths.push(AccelPath {
            name: "AMD ROCm / HIP".into(),
            hardware: amd,
            driver,
            runtime,
            status: status(amd, driver, runtime),
            detail: format!("ROCm {}", r.compute.rocm_version.as_deref().unwrap_or("absent")),
            bindings,
        });
    }

    // OpenVINO (CPU/GPU/NPU Intel).
    let intel = r.gpus.iter().any(|g| g.vendor.as_deref() == Some("Intel")) || r.npus.iter().any(|n| n.vendor.as_deref() == Some("Intel"));
    let ov_bind = fw_with_acc(r, |a| a.starts_with("GPU") || a.starts_with("NPU"));
    if intel || has_lib("libopenvino.so") || python_has(r, "openvino") {
        let runtime = has_lib("libopenvino.so") || python_has(r, "openvino");
        paths.push(AccelPath {
            name: "Intel OpenVINO (GPU/NPU)".into(),
            hardware: intel,
            driver: r.gpus.iter().any(|g| matches!(g.driver.as_deref(), Some("i915") | Some("xe"))) || r.npus.iter().any(|n| n.driver.as_deref() == Some("intel_vpu")),
            runtime,
            status: status(intel, true, runtime),
            detail: "OpenVINO cible le CPU, l'iGPU Intel et le NPU Intel".into(),
            bindings: ov_bind,
        });
    }

    // OpenCL : calcul GPU générique (Mali, Adreno, Intel, AMD, NVIDIA…).
    let cl_devices: usize = r.compute.opencl_platforms.iter().map(|p| p.devices.len()).sum();
    let has_gpu = !r.gpus.is_empty();
    if has_gpu || cl_devices > 0 {
        let runtime = has_lib("libOpenCL.so") || has_lib("libmali.so");
        paths.push(AccelPath {
            name: "OpenCL".into(),
            hardware: has_gpu,
            driver: cl_devices > 0,
            runtime,
            status: status(has_gpu, cl_devices > 0, runtime),
            detail: if cl_devices > 0 {
                r.compute.opencl_platforms.iter().map(|p| format!("{} : {}", p.name, p.devices.join(", "))).collect::<Vec<_>>().join(" | ")
            } else if r.compute.opencl_icds.is_empty() {
                "aucun driver OpenCL (ICD) installé".into()
            } else {
                format!("ICD installés ({}) mais aucun périphérique listé (clinfo absent ?)", r.compute.opencl_icds.join(", "))
            },
            bindings: fw_with_acc(r, |a| a == "OpenCL"),
        });
    }

    // Vulkan compute (llama.cpp, ncnn, MLC…).
    let vk_gpu: Vec<&VulkanDevice> = r.compute.vulkan_devices.iter().filter(|d| d.device_type.as_deref() != Some("cpu")).collect();
    if has_gpu || !vk_gpu.is_empty() {
        let runtime = has_lib("libvulkan.so");
        paths.push(AccelPath {
            name: "Vulkan compute".into(),
            hardware: has_gpu,
            driver: !vk_gpu.is_empty(),
            runtime,
            status: status(has_gpu, !vk_gpu.is_empty(), runtime),
            detail: if vk_gpu.is_empty() {
                "aucun GPU Vulkan matériel (seulement llvmpipe ou rien) ; utilisable par llama.cpp, ncnn, MLC si présent".into()
            } else {
                vk_gpu.iter().map(|d| format!("{} (Vulkan {}, {})", d.name, d.api_version.as_deref().unwrap_or("?"), d.driver.as_deref().unwrap_or("?"))).collect::<Vec<_>>().join(" | ")
            },
            bindings: Vec::new(),
        });
    }

    // Chaque NPU.
    for n in &r.npus {
        let driver = n.status.starts_with("actif") || n.driver.is_some();
        let runtime = !n.runtime_found.is_empty();
        let bindings = fw_with_acc(r, |a| {
            let a = a.to_lowercase();
            a.contains("hailo") || a.contains("edgetpu") || a.contains("usb") || a.contains("pci") || a.starts_with("npu")
        });
        let mut detail = format!("{} ; {}", n.status, if runtime { format!("runtime : {}", n.runtime_found.join(", ")) } else { "aucun runtime trouvé".into() });
        if let Some(h) = npu_hint(n.vendor.as_deref()) {
            if !runtime {
                detail.push_str(&format!(" — {}", h));
            }
        }
        paths.push(AccelPath { name: format!("NPU {}", n.name), hardware: true, driver, runtime, status: status(true, driver, runtime), detail, bindings });
    }
    paths
}

fn npu_hint(vendor: Option<&str>) -> Option<&'static str> {
    Some(match vendor? {
        "Rockchip" => "runtime et exemples : github.com/airockchip/rknn-toolkit2 (librknnrt + rknn-toolkit-lite2), LLM : github.com/airockchip/rknn-llm",
        "Hailo" => "installer HailoRT (driver PCIe + libhailort) depuis hailo.ai/developer-zone",
        "Google" => "installer libedgetpu1-std depuis coral.ai/software",
        "VeriSilicon" => "runtime fourni par le BSP du fabricant (OpenVX / TIM-VX / délégué TFLite VX)",
        "Intel" => "pip install openvino (le driver intel_vpu doit être chargé)",
        "AMD" => "Ryzen AI Software (XRT + ONNX Runtime Vitis AI)",
        "NVIDIA" => "TensorRT (inclus dans JetPack) pour exploiter le DLA",
        _ => return None,
    })
}

// ---------------------------------------------------------------- Estimation LLM

/// Q4_K_M ≈ 4,85 bits par paramètre (format le plus courant de llama.cpp / Ollama).
const Q4_BYTES_PER_PARAM: f64 = 4.85 / 8.0;
/// Marge pour le cache KV, le contexte et le runtime.
const OVERHEAD: f64 = 1.2;

fn llm_estimate(r: &Report) -> Option<LlmEstimate> {
    if r.memory.total_mb == 0 {
        return None;
    }
    let ram_budget = r.memory.available_mb as f64 / 1024.0;
    let vram_budget = r.gpus.iter().filter_map(|g| g.vram_mb).max().map(|mb| mb as f64 / 1024.0);
    let bw = r.bench.as_ref().map(|b| b.mem_read_multi_gbs).filter(|&b| b > 0.0);
    let rows = [0.5, 1.0, 1.5, 3.0, 4.0, 7.0, 8.0, 13.0, 14.0, 32.0, 70.0]
        .iter()
        .map(|&p| {
            let size = p * Q4_BYTES_PER_PARAM;
            LlmRow {
                params_b: p,
                size_gb: crate::util::round2(size),
                fits_ram: size * OVERHEAD <= ram_budget,
                fits_vram: vram_budget.map(|v| size * OVERHEAD <= v),
                max_tokens_per_s: bw.map(|b| crate::util::round1(b / size)),
            }
        })
        .collect();
    Some(LlmEstimate {
        ram_budget_gb: crate::util::round1(ram_budget),
        vram_budget_gb: vram_budget.map(crate::util::round1),
        measured_bandwidth_gbs: bw,
        bytes_per_param: Q4_BYTES_PER_PARAM,
        rows,
        method: format!(
            "Taille = paramètres × {:.3} octet (quantification Q4_K_M) ; « tient » si taille × {} ≤ RAM disponible au moment de la mesure. \
             Débit max = bande passante mémoire mesurée (lecture, tous les cœurs) ÷ taille du modèle : chaque token relit tous les poids, \
             c'est donc une borne haute pour l'inférence CPU, le débit réel est généralement 30 à 70 % de cette borne.",
            Q4_BYTES_PER_PARAM, OVERHEAD
        ),
    })
}

// ---------------------------------------------------------------- Constats

fn f(level: &str, category: &str, title: impl Into<String>, detail: impl Into<String>, hint: Option<&str>) -> Finding {
    Finding { level: level.into(), category: category.into(), title: title.into(), detail: detail.into(), hint: hint.map(String::from) }
}

fn findings(r: &Report) -> Vec<Finding> {
    let mut out = Vec::new();
    let arch = r.os.arch.clone().unwrap_or_default();
    let is_arm64 = arch == "aarch64" || arch == "arm64";

    // --- Plateforme
    if (is_arm64 || arch == "x86_64") && r.os.userland_bits == Some(32) {
        out.push(f("warn", "Système", "Espace utilisateur 32 bits sur noyau 64 bits", format!("Noyau {} mais userland 32 bits ({})", arch, r.os.package_arch.as_deref().unwrap_or("?")), Some("La plupart des wheels IA (torch, onnxruntime, rknn-lite…) n'existent qu'en 64 bits : installer une image 64 bits.")));
    }
    if arch.starts_with("armv6") || arch.starts_with("armv7") {
        out.push(f("warn", "Système", "Architecture ARM 32 bits", format!("{} : écosystème IA très limité (peu de binaires précompilés)", arch), None));
    }
    if let Some(libc) = &r.os.libc {
        if libc.starts_with("musl") {
            out.push(f("warn", "Système", "libc musl", "Les wheels Python « manylinux » ne s'installent pas sur musl (Alpine…)", Some("Préférer une distribution glibc (Debian, Ubuntu) pour l'IA.")));
        }
    }

    // --- Mémoire
    let ram_gb = r.memory.total_mb as f64 / 1024.0;
    if r.memory.total_mb > 0 && ram_gb < 2.0 {
        out.push(f("warn", "Mémoire", format!("RAM faible : {:.1} Go", ram_gb), "Compilation (Rust, C++) et modèles IA limités", Some("Ajouter du swap/zram et compiler en croisé depuis un PC.")));
    }
    if r.memory.swap_total_mb == 0 && r.memory.total_mb > 0 {
        out.push(f("info", "Mémoire", "Aucun swap", "Un pic mémoire (compilation, chargement de modèle) tuera le processus (OOM)", Some("Activer zram (paquet zram-tools / systemd-zram-generator).")));
    }

    // --- Stockage
    if let Some(kind) = &r.storage.root_device_kind {
        if kind.starts_with("Carte SD") {
            let speed = r.bench.as_ref().and_then(|b| b.disk.as_ref()).map(|d| format!(" (écriture mesurée : {} Mo/s)", d.write_mbs)).unwrap_or_default();
            out.push(f("warn", "Stockage", "Système sur carte SD", format!("{}{}", kind, speed), Some("Lent et sensible à l'usure : préférer eMMC, NVMe ou SSD USB pour le développement.")));
        }
    }
    if let Some(root) = r.storage.filesystems.iter().find(|f| f.mount == "/") {
        if root.free_gb < 5.0 {
            out.push(f("warn", "Stockage", format!("Peu d'espace libre sur / : {:.1} Go", root.free_gb), "Toolchains, conteneurs et modèles IA prennent vite plusieurs Go", None));
        }
    }

    // --- CPU
    let has = |flag: &str| r.cpu.isa_features.iter().any(|x| x.flag == flag);
    if is_arm64 {
        if has("asimddp") {
            out.push(f("ok", "CPU", "Produit scalaire int8 (dotprod) disponible", "llama.cpp, ONNX Runtime et TFLite exploitent cette extension pour l'int8", None));
        } else {
            out.push(f("info", "CPU", "Pas d'extension dotprod", "Cœurs ARMv8.0 (Cortex-A53/A72…) : inférence int8 et LLM nettement plus lents", None));
        }
    }
    if arch == "x86_64" && !has("avx2") {
        out.push(f("warn", "CPU", "Pas d'AVX2", "Beaucoup de builds IA précompilés (TensorFlow, certains llama.cpp) exigent AVX2", None));
    }
    if let Some(t) = r.cpu.temperature_c {
        if t >= 75.0 {
            out.push(f("warn", "Thermique", format!("CPU déjà à {:.0} °C", t), "Risque de throttling sous charge prolongée", Some("Ajouter un dissipateur / ventilateur.")));
        }
    }
    if r.cpu.governor.as_deref() == Some("powersave") {
        out.push(f("info", "CPU", "Gouverneur « powersave »", "Les fréquences restent basses : benchmarks et inférence ralentis", Some("Pour tester : passer le gouverneur en « performance » ou « schedutil ».")));
    }

    // --- Accélérateurs
    for p in &r.ai.paths {
        match p.status.as_str() {
            "utilisable" if p.name != "CPU" => out.push(f("ok", "Accélération IA", format!("{} : utilisable", p.name), p.detail.clone(), None)),
            "partiel" => out.push(f("warn", "Accélération IA", format!("{} : incomplet", p.name), p.detail.clone(), None)),
            _ => {}
        }
    }
    if r.gpus.is_empty() && r.npus.is_empty() {
        out.push(f("info", "Accélération IA", "Ni GPU ni NPU détecté", "Inférence sur CPU uniquement : viser des modèles quantifiés (int8/Q4) et légers", None));
    }
    // Framework installé mais qui ne voit pas l'accélérateur présent.
    if let Some(py) = &r.python {
        let nvidia = r.gpus.iter().any(|g| g.vendor.as_deref() == Some("NVIDIA"));
        for fw in &py.frameworks {
            if let Some(err) = &fw.error {
                out.push(f("warn", "Python", format!("{} installé mais import en échec", fw.name), err.clone(), None));
            } else if nvidia && fw.import_ok && fw.name == "PyTorch" && !fw.accelerators.iter().any(|a| a.starts_with("cuda")) {
                out.push(f("warn", "Python", "PyTorch ne voit pas le GPU NVIDIA", fw.details.join(", "), Some("Installer la variante CUDA de torch correspondant au driver (voir pytorch.org).")));
            } else if fw.name == "ONNX Runtime" && fw.import_ok && fw.accelerators.is_empty() && (nvidia || !r.npus.is_empty()) {
                out.push(f("info", "Python", "ONNX Runtime en mode CPU uniquement", fw.details.join(", "), Some("Paquet spécifique requis : onnxruntime-gpu (CUDA), onnxruntime-openvino, onnxruntime-qnn…")));
            }
        }
    } else {
        out.push(f("info", "Python", "Python 3 absent", "La majorité des SDK IA/NPU passent par Python", None));
    }

    // --- Vidéo
    if !r.video.hw_codecs.is_empty() {
        let enc: Vec<String> = r.video.hw_codecs.iter().filter(|c| c.direction == "encode").map(|c| c.codec.clone()).collect();
        let dec: Vec<String> = r.video.hw_codecs.iter().filter(|c| c.direction == "decode").map(|c| c.codec.clone()).collect();
        out.push(f("ok", "Vidéo", "Codecs vidéo matériels exposés", format!("décodage : {} ; encodage : {}", join_or(&dec), join_or(&enc)), None));
    }
    // Blocs codecs constructeur hors V4L2 standard (dma_heap, RGA et VIC ne sont pas des codecs).
    let vendor_codecs: Vec<&str> = r.video.vendor_nodes.iter().map(|s| s.as_str()).filter(|n| !n.contains("dma_heap") && !n.contains("RGA") && !n.contains("VIC")).collect();
    if r.video.hw_codecs.is_empty() && !vendor_codecs.is_empty() {
        out.push(f("info", "Vidéo", "Codecs matériels via API constructeur", vendor_codecs.join(" ; "), Some("Ces blocs ne passent pas par V4L2 standard : utiliser les plugins GStreamer/FFmpeg du fabricant (mpp, nvv4l2…).")));
    }
    if r.video.v4l2_devices.iter().any(|d| d.error.as_deref().is_some_and(|e| e.contains("video"))) {
        out.push(f("info", "Vidéo", "Accès refusé aux périphériques /dev/video*", "L'utilisateur n'est pas dans le groupe « video »", Some("sudo usermod -aG video $USER puis se reconnecter.")));
    }

    // --- Interfaces
    let disabled: Vec<String> = r.interfaces.dt_peripherals.iter().filter(|p| p.disabled > 0 && matches!(p.kind.as_str(), "I2C" | "SPI" | "UART" | "PWM" | "CAN" | "Caméra MIPI-CSI" | "NPU")).map(|p| format!("{} ×{}", p.kind, p.disabled)).collect();
    if !disabled.is_empty() {
        out.push(f("info", "Interfaces", "Contrôleurs présents mais désactivés dans le device-tree", disabled.join(", "), Some("Activables par overlay (armbian-config, rsetup, dtoverlay, /boot/…/config.txt selon la carte).")));
    }
    if r.board.has_device_tree && !r.interfaces.gpio_chips.is_empty() && !r.dev.tools.iter().any(|t| t.command == "gpiodetect") && !has_lib("libgpiod.so") {
        out.push(f("info", "Interfaces", "GPIO présents mais libgpiod absent", format!("{} contrôleur(s) GPIO", r.interfaces.gpio_chips.len()), Some("sudo apt install gpiod libgpiod-dev (API GPIO moderne, remplace sysfs)")));
    }

    // --- Dev
    let tool = |cmd: &str| r.dev.tools.iter().any(|t| t.command == cmd);
    if !tool("gcc") && !tool("clang") {
        out.push(f("missing", "Dev", "Aucun compilateur C/C++", "Nécessaire pour compiler la plupart des librairies natives et des paquets pip sans wheel", Some("sudo apt install build-essential")));
    }
    if r.dev.docker_daemon_access == Some(false) {
        out.push(f("info", "Dev", "Docker installé mais démon inaccessible", "Démon arrêté ou utilisateur hors du groupe docker", Some("sudo usermod -aG docker $USER (puis se reconnecter)")));
    }
    if !tool("git") {
        out.push(f("missing", "Dev", "Git absent", "", Some("sudo apt install git")));
    }
    out
}

fn join_or(v: &[String]) -> String {
    if v.is_empty() { "aucun".into() } else { v.join(", ") }
}
