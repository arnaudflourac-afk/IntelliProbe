//! Guide de développement : recommandations concrètes déduites des données mesurées.
//! Chaque recommandation cite les faits qui la justifient, donne les commandes
//! adaptées à cette machine (gestionnaire de paquets, architecture, versions)
//! et les pièges à éviter.

use crate::report::*;

pub fn recommend(r: &Report) -> Vec<Recommendation> {
    let c = Ctx::new(r);
    let mut out = Vec::new();
    base_tools(&c, &mut out);
    workflow(&c, &mut out);
    storage_memory(&c, &mut out);
    ai(&c, &mut out);
    llm(&c, &mut out);
    python(&c, &mut out);
    video(&c, &mut out);
    io(&c, &mut out);
    containers(&c, &mut out);
    deployment(&c, &mut out);
    let rank = |p: &str| match p {
        "essentiel" => 0,
        "recommandé" => 1,
        _ => 2,
    };
    out.sort_by_key(|x| rank(&x.priority));
    out
}

// ---------------------------------------------------------------- Contexte

struct Ctx<'a> {
    r: &'a Report,
    arch: String,
    is_sbc: bool,
    ram_gb: f64,
    cores: usize,
    headless: bool,
}

impl<'a> Ctx<'a> {
    fn new(r: &'a Report) -> Self {
        Ctx {
            r,
            arch: r.os.arch.clone().unwrap_or_default(),
            is_sbc: r.board.has_device_tree,
            ram_gb: r.memory.total_mb as f64 / 1024.0,
            cores: r.cpu.logical_cores.max(1),
            headless: r.os.desktop.is_none(),
        }
    }

    fn tool(&self, cmd: &str) -> bool {
        self.r.dev.tools.iter().chain(self.r.ai.runtimes.iter()).any(|t| t.command == cmd)
    }

    /// Librairie partagée présente (d'après le rapport, pas la machine courante).
    fn lib(&self, prefix: &str) -> bool {
        self.r.packages.all_shared_libs.iter().any(|l| l.starts_with(prefix))
    }

    fn py_pkg(&self, name: &str) -> bool {
        let n = |s: &str| s.to_lowercase().replace('_', "-");
        self.r.python.as_ref().is_some_and(|p| p.packages.iter().any(|x| n(&x.name) == n(name)))
    }

    fn framework(&self, name: &str) -> Option<&'a Framework> {
        self.r.python.as_ref()?.frameworks.iter().find(|f| f.name == name)
    }

    fn py(&self) -> String {
        self.r.python.as_ref().map(|p| p.executable.clone()).unwrap_or_else(|| "python3".into())
    }

    /// (majeur, mineur) de Python.
    fn py_version(&self) -> Option<(u32, u32)> {
        let v = self.r.python.as_ref()?.version.clone()?;
        let mut it = v.split('.');
        Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
    }

    fn compat(&self, s: &str) -> bool {
        self.r.board.dt_compatible.iter().any(|c| c.contains(s))
    }

    /// Commande d'installation adaptée au gestionnaire de paquets détecté.
    fn install(&self, pkgs: &[Pkg]) -> Option<String> {
        let mgr = self.r.os.package_manager.as_deref().unwrap_or("");
        let (cmd, pick): (&str, fn(&Pkg) -> &'static str) = if mgr.starts_with("apt") {
            ("sudo apt install -y", |p| p.apt)
        } else if mgr == "rpm" {
            (if self.tool("dnf") { "sudo dnf install -y" } else { "sudo yum install -y" }, |p| p.dnf)
        } else if mgr == "pacman" {
            ("sudo pacman -S --needed", |p| p.pacman)
        } else if mgr == "apk" {
            ("sudo apk add", |p| p.apk)
        } else {
            return None;
        };
        let names: Vec<&str> = pkgs.iter().map(pick).filter(|n| !n.is_empty()).collect();
        (!names.is_empty()).then(|| format!("{} {}", cmd, names.join(" ")))
    }

    fn install_or(&self, pkgs: &[Pkg], fallback: &str) -> String {
        self.install(pkgs).unwrap_or_else(|| fallback.to_string())
    }

    /// Cible Rust, plateforme Docker et paquet du compilateur croisé pour cette architecture.
    fn targets(&self) -> Option<(&'static str, &'static str, &'static str)> {
        Some(match self.arch.as_str() {
            "aarch64" | "arm64" => ("aarch64-unknown-linux-gnu", "linux/arm64", "gcc-aarch64-linux-gnu"),
            a if a.starts_with("armv7") => ("armv7-unknown-linux-gnueabihf", "linux/arm/v7", "gcc-arm-linux-gnueabihf"),
            a if a.starts_with("armv6") => ("arm-unknown-linux-gnueabihf", "linux/arm/v6", "gcc-arm-linux-gnueabihf"),
            "riscv64" => ("riscv64gc-unknown-linux-gnu", "linux/riscv64", "gcc-riscv64-linux-gnu"),
            "x86_64" => ("x86_64-unknown-linux-gnu", "linux/amd64", "gcc"),
            _ => return None,
        })
    }
}

/// Nom d'un paquet selon la distribution (vide = inutile / inclus ailleurs).
struct Pkg {
    apt: &'static str,
    dnf: &'static str,
    pacman: &'static str,
    apk: &'static str,
}

const fn pkg(apt: &'static str, dnf: &'static str, pacman: &'static str, apk: &'static str) -> Pkg {
    Pkg { apt, dnf, pacman, apk }
}

fn rec(domain: &str, priority: &str, title: impl Into<String>, rationale: impl Into<String>, steps: Vec<String>, avoid: Vec<String>) -> Recommendation {
    Recommendation { domain: domain.into(), priority: priority.into(), title: title.into(), rationale: rationale.into(), steps, avoid }
}

fn s(x: &str) -> String {
    x.to_string()
}

// ---------------------------------------------------------------- Base

fn base_tools(c: &Ctx, out: &mut Vec<Recommendation>) {
    let mut missing = Vec::new();
    let mut pkgs = Vec::new();
    if !c.tool("gcc") && !c.tool("clang") {
        missing.push("compilateur C/C++");
        pkgs.push(pkg("build-essential", "gcc gcc-c++ make", "base-devel", "build-base"));
    }
    if !c.tool("git") {
        missing.push("git");
        pkgs.push(pkg("git", "git", "git", "git"));
    }
    if !c.tool("cmake") {
        missing.push("CMake");
        pkgs.push(pkg("cmake", "cmake", "cmake", "cmake"));
    }
    if !c.tool("pkg-config") {
        missing.push("pkg-config");
        pkgs.push(pkg("pkg-config", "pkgconf-pkg-config", "pkgconf", "pkgconf"));
    }
    if c.r.python.as_ref().is_some_and(|p| !p.venv_available) {
        missing.push("module venv de Python");
        pkgs.push(pkg("python3-venv", "", "", ""));
    }
    if c.r.python.is_none() {
        missing.push("Python 3");
        pkgs.push(pkg("python3 python3-pip python3-venv", "python3 python3-pip", "python python-pip", "python3 py3-pip"));
    }
    if missing.is_empty() {
        return;
    }
    let steps = c.install(&pkgs).map(|i| vec![i]).unwrap_or_else(|| vec![format!("Installer : {}", missing.join(", "))]);
    out.push(rec(
        "Base",
        "essentiel",
        "Installer les outils de base manquants",
        format!("Absents de cette machine : {}. La plupart des librairies natives et des paquets pip sans wheel pour {} doivent être compilés.", missing.join(", "), c.arch),
        steps,
        vec![],
    ));
}

// ---------------------------------------------------------------- Workflow

fn workflow(c: &Ctx, out: &mut Vec<Recommendation>) {
    let r = c.r;
    let gflops = r.bench.as_ref().map(|b| b.cpu_multi_gflops);
    let on_sd = r.storage.root_device_kind.as_deref().is_some_and(|k| k.starts_with("Carte SD"));
    // Environ 1,5 Go par tâche de compilation C++/Rust lourde.
    let jobs = ((c.ram_gb / 1.5).floor() as usize).clamp(1, c.cores);

    if c.arch == "x86_64" && !c.is_sbc {
        // Un PC : la bonne machine pour compiler pour les cartes.
        let cross = &r.dev.cross_compilers;
        let mut steps = vec![
            s("rustup target add aarch64-unknown-linux-gnu   # puis : cargo build --release --target aarch64-unknown-linux-gnu"),
        ];
        if cross.is_empty() {
            steps.push(c.install_or(&[pkg("gcc-aarch64-linux-gnu g++-aarch64-linux-gnu", "gcc-aarch64-linux-gnu", "aarch64-linux-gnu-gcc", "")], "Installer un compilateur croisé aarch64"));
        }
        if c.tool("docker") {
            if !r.dev.binfmt.iter().any(|x| x == "qemu-aarch64") {
                steps.push(s("docker run --privileged --rm tonistiigi/binfmt --install arm64,arm   # émulation pour construire des images ARM"));
            }
            steps.push(s("docker buildx build --platform linux/arm64 -t mon-app:arm64 --load ."));
        }
        steps.push(s("scp target/aarch64-unknown-linux-gnu/release/mon-app user@carte:~   # déployer sur la carte"));
        out.push(rec(
            "Workflow",
            "optionnel",
            "Utiliser ce PC comme machine de build pour les cartes ARM",
            format!(
                "{} cœurs, {:.0} Go de RAM{} ; compilateurs croisés présents : {} ; émulation binfmt : {}.",
                c.cores,
                c.ram_gb,
                gflops.map(|g| format!(", {} GFLOPS mesurés", g)).unwrap_or_default(),
                if cross.is_empty() { "aucun".into() } else { cross.join(", ") },
                {
                    // Seules les architectures des cartes nous intéressent ici.
                    let b: Vec<&str> = r.dev.binfmt.iter().map(|x| x.as_str()).filter(|x| ["qemu-aarch64", "qemu-arm", "qemu-riscv64"].contains(x)).collect();
                    if b.is_empty() { "aucune pour ARM/RISC-V".to_string() } else { b.join(", ") }
                }
            ),
            steps,
            vec![s("Compiler sur la carte ce qui peut l'être ici : un build Rust/C++ est souvent 5 à 20 fois plus rapide sur PC.")],
        ));
        return;
    }

    let Some((rust_t, docker_p, gcc_pkg)) = c.targets() else { return };
    let weak = c.ram_gb < 4.0 || gflops.is_some_and(|g| g < 40.0) || on_sd;
    let facts = format!(
        "{:.1} Go de RAM, {} cœurs{}{}.",
        c.ram_gb,
        c.cores,
        gflops.map(|g| format!(", {} GFLOPS mesurés (tous cœurs)", g)).unwrap_or_default(),
        if on_sd { ", système sur carte SD" } else { "" }
    );
    let mut steps = Vec::new();
    if weak {
        steps.push(format!("Sur le PC : rustup target add {} && cargo build --release --target {}", rust_t, rust_t));
        steps.push(format!("Sur le PC (C/C++) : sudo apt install {} puis CMake avec -DCMAKE_C_COMPILER={}", gcc_pkg, gcc_pkg.trim_start_matches("gcc-").to_string() + "-gcc"));
        steps.push(format!("Sur le PC (Docker) : docker buildx build --platform {} -t mon-app --load .", docker_p));
    }
    steps.push(format!("Si tu compiles sur la carte : limiter le parallélisme à {} tâche(s) (make -j{} / CARGO_BUILD_JOBS={}) pour éviter le manque de mémoire", jobs, jobs, jobs));
    if c.headless || c.is_sbc {
        steps.push(s("Éditer depuis le PC avec VS Code « Remote - SSH » (l'éditeur reste sur le PC, le code s'exécute sur la carte)"));
    }
    if !c.tool("ccache") {
        steps.push(format!("{}   # recompilations C/C++ beaucoup plus rapides", c.install_or(&[pkg("ccache", "ccache", "ccache", "ccache")], "installer ccache")));
    }
    out.push(rec(
        "Workflow",
        if c.ram_gb < 4.0 { "essentiel" } else { "recommandé" },
        if weak { "Compiler sur un PC, déployer sur la carte" } else { "Développement natif possible, avec quelques réglages" },
        facts,
        steps,
        vec![
            s("Lancer un IDE lourd (IntelliJ, VS Code complet) sur la carte : il consomme la RAM dont tes programmes ont besoin."),
            format!("Télécharger des binaires x86_64 : il faut des builds {} ({}).", c.arch, docker_p),
        ],
    ));
}

fn storage_memory(c: &Ctx, out: &mut Vec<Recommendation>) {
    let r = c.r;
    if let Some(kind) = r.storage.root_device_kind.as_deref().filter(|k| k.starts_with("Carte SD")) {
        let fast: Vec<String> = r.storage.devices.iter().filter(|d| matches!(d.kind.as_str(), "NVMe" | "eMMC" | "SSD") || d.kind.starts_with("Disque USB")).map(|d| format!("{} ({}, {:.0} Go)", d.name, d.kind, d.size_gb)).collect();
        let speed = r.bench.as_ref().and_then(|b| b.disk.as_ref()).map(|d| format!(" Écriture mesurée : {} Mo/s.", d.write_mbs)).unwrap_or_default();
        let mut steps = Vec::new();
        if fast.is_empty() {
            steps.push(s("Ajouter un stockage rapide (NVMe si la carte a un port M.2, sinon SSD USB 3) et y installer le système ou au moins le workspace"));
        } else {
            steps.push(format!("Tu as déjà un support plus rapide : {}. Y placer le workspace, les modèles IA et les données Docker (data-root dans /etc/docker/daemon.json)", fast.join(", ")));
        }
        steps.push(s("Utiliser des cartes SD A2 / « High Endurance » si la SD reste le support principal"));
        out.push(rec("Stockage", "essentiel", "Sortir le développement de la carte SD", format!("Le système tourne sur {}.{}", kind, speed), steps, vec![s("Écrire des logs ou des bases de données en continu sur la SD : usure rapide et corruption en cas de coupure.")]));
    }
    if r.memory.swap_total_mb == 0 && r.memory.total_mb > 0 {
        let step = if r.os.package_manager.as_deref().is_some_and(|m| m.starts_with("apt")) {
            s("sudo apt install -y zram-tools && printf 'ALGO=zstd\\nPERCENT=50\\n' | sudo tee /etc/default/zramswap && sudo systemctl restart zramswap")
        } else {
            s("Installer zram-generator (systemd) et définir zram-size = ram / 2")
        };
        out.push(rec("Mémoire", if c.ram_gb < 8.0 { "essentiel" } else { "optionnel" }, "Activer un swap compressé en RAM (zram)", format!("{:.1} Go de RAM et aucun swap : un pic (compilation, chargement de modèle) fera tuer le processus.", c.ram_gb), vec![step], vec![s("Mettre un fichier de swap sur carte SD (lent et use la carte).")]));
    }
    if let Some(root) = r.storage.filesystems.iter().find(|f| f.mount == "/").filter(|f| f.free_gb < 10.0) {
        out.push(rec(
            "Stockage",
            "recommandé",
            format!("Libérer de la place sur / ({:.1} Go libres)", root.free_gb),
            "Une toolchain, un environnement Python IA ou un modèle occupent chacun plusieurs Go.",
            vec![s("docker system prune -a   # si Docker est utilisé"), s("pip cache purge ; sudo apt clean"), s("Vérifier que la partition occupe tout le support (resize2fs / growpart)")],
            vec![],
        ));
    }
}

// ---------------------------------------------------------------- IA

fn ai(c: &Ctx, out: &mut Vec<Recommendation>) {
    let r = c.r;
    let mut main_done = false;
    let prio = |done: &mut bool| {
        let p = if *done { "recommandé" } else { "essentiel" };
        *done = true;
        p
    };

    for n in &r.npus {
        let p = prio(&mut main_done);
        if let Some(x) = npu_rec(c, n, p) {
            out.push(x);
        }
    }

    let nvidia = r.gpus.iter().any(|g| g.vendor.as_deref() == Some("NVIDIA"));
    if nvidia {
        let p = prio(&mut main_done);
        out.push(if r.board.jetson.is_some() || c.compat("nvidia,tegra") { jetson(c, p) } else { cuda(c, p) });
    }
    if r.gpus.iter().any(|g| g.vendor.as_deref() == Some("Intel")) && !r.npus.iter().any(|n| n.vendor.as_deref() == Some("Intel")) {
        let has = c.py_pkg("openvino");
        out.push(rec(
            "IA",
            "optionnel",
            "iGPU Intel : OpenVINO",
            format!("GPU Intel détecté ; OpenVINO {}.", if has { "installé" } else { "absent" }),
            vec![s("pip install openvino   # puis compile_model(model, \"GPU\")"), s("Vérifier les périphériques : python -c \"import openvino as ov; print(ov.Core().available_devices)\"")],
            vec![],
        ));
    }
    if r.gpus.iter().any(|g| g.vendor.as_deref() == Some("AMD")) && r.gpus.iter().all(|g| g.vendor.as_deref() != Some("NVIDIA")) {
        out.push(rec(
            "IA",
            prio(&mut main_done),
            "GPU AMD : ROCm ou Vulkan",
            format!("GPU AMD détecté ; ROCm {}.", r.compute.rocm_version.as_deref().unwrap_or("absent")),
            vec![s("PyTorch ROCm : pip install torch --index-url https://download.pytorch.org/whl/rocm6.2 (vérifier que le GPU est dans la liste supportée par ROCm)"), s("LLM : llama.cpp compilé avec -DGGML_VULKAN=ON fonctionne sur la plupart des GPU AMD sans ROCm")],
            vec![],
        ));
    }
    // GPU intégré de SoC (Mali, Adreno, VideoCore…).
    if let Some(g) = r.gpus.iter().find(|g| matches!(g.source.as_str(), "drm" | "mali kbase")) {
        let vk = r.compute.vulkan_devices.iter().any(|d| d.device_type.as_deref() != Some("cpu"));
        let cl = r.compute.opencl_platforms.iter().any(|p| !p.devices.is_empty());
        let mut steps = Vec::new();
        if vk {
            steps.push(s("ncnn (backend Vulkan) : très bon choix pour la vision sur GPU mobile — github.com/Tencent/ncnn"));
            steps.push(s("llama.cpp avec -DGGML_VULKAN=ON pour les LLM (à comparer au CPU : sur petit GPU le CPU reste parfois plus rapide)"));
        }
        if cl {
            steps.push(s("Arm NN / Arm Compute Library ou MNN (OpenCL) pour l'inférence sur GPU Mali"));
        }
        if !vk && !cl {
            steps.push(s("Aucune API de calcul GPU exposée : installer un Mesa récent (Panfrost/Panthor → OpenCL rusticl, Vulkan PanVK) ou le pilote libmali du fabricant"));
            steps.push(s("Vérifier ensuite avec clinfo -l et vulkaninfo --summary"));
        }
        out.push(rec(
            "IA",
            if main_done { "optionnel" } else { prio(&mut main_done) },
            format!("GPU intégré : {}", g.name),
            format!("Vulkan matériel : {} ; OpenCL : {}.", if vk { "oui" } else { "non" }, if cl { "oui" } else { "non" }),
            steps,
            vec![s("Attendre de PyTorch/TensorFlow qu'ils utilisent ce GPU : ils ne supportent que CUDA/ROCm/Metal.")],
        ));
    }

    // CPU : voie de secours ou principale.
    let feats: Vec<&str> = r.cpu.isa_features.iter().map(|f| f.flag.as_str()).collect();
    let is_arm = c.arch == "aarch64" || c.arch.starts_with("arm");
    let mut steps = vec![s("ONNX Runtime (pip install onnxruntime) : format d'échange universel, exporter les modèles en ONNX")];
    if is_arm {
        steps.push(s("TFLite / LiteRT avec XNNPACK (pip install ai-edge-litert) ou ncnn : les plus rapides sur CPU ARM"));
    } else {
        steps.push(s("OpenVINO (pip install openvino) : très efficace sur CPU x86 Intel et AMD"));
    }
    steps.push(s("Quantifier en int8 (onnxruntime.quantization, export int8 TFLite) : 2 à 4× plus rapide et 4× plus léger qu'en FP32"));
    if let Some(b) = &r.bench {
        steps.push(format!("Ordre de grandeur mesuré : {} GFLOPS FP32 sur {} threads", b.cpu_multi_gflops, b.threads));
    }
    let mut avoid = vec![s("Entraîner des modèles sur cette machine si aucun GPU CUDA n'est présent : entraîner sur PC/cloud, n'exécuter ici que l'inférence.")];
    if is_arm && !feats.contains(&"asimddp") {
        avoid.push(s("Les modèles int8 lourds : sans dotprod, le gain int8 est réduit sur ces cœurs."));
    }
    out.push(rec(
        "IA",
        if main_done { "optionnel" } else { "essentiel" },
        if main_done { "Inférence CPU (voie de secours)" } else { "Inférence sur CPU : la voie principale ici" },
        format!("{} cœurs ; extensions utiles : {}.", c.cores, if feats.is_empty() { "aucune".into() } else { feats.join(", ") }),
        steps,
        avoid,
    ));

    // Frameworks installés mais cassés : commande de réparation exacte.
    if let Some(py) = &r.python {
        let mut mods = Vec::new();
        let mut others = Vec::new();
        for f in py.frameworks.iter().filter(|f| f.error.is_some()) {
            let e = f.error.as_deref().unwrap_or("");
            match e.split("No module named '").nth(1).and_then(|x| x.split('\'').next()) {
                Some(m) => mods.push(pip_name(m.split('.').next().unwrap_or(m))),
                None => others.push(format!("{} : {}", f.name, e)),
            }
        }
        mods.sort();
        mods.dedup();
        if !mods.is_empty() || !others.is_empty() {
            let mut steps = Vec::new();
            if !mods.is_empty() {
                steps.push(format!("{} -m pip install {}", c.py(), mods.join(" ")));
            }
            steps.extend(others.iter().map(|o| format!("À examiner — {}", o)));
            steps.push(format!("{} -m pip check   # liste les autres dépendances cassées", c.py()));
            let broken: Vec<&str> = py.frameworks.iter().filter(|f| f.error.is_some()).map(|f| f.name.as_str()).collect();
            out.push(rec("Python", "recommandé", "Réparer les frameworks qui ne s'importent pas", format!("Import en échec : {}.", broken.join(", ")), steps, vec![]));
        }
    }
}

fn pip_name(module: &str) -> String {
    match module {
        "cv2" => "opencv-python",
        "yaml" => "pyyaml",
        "PIL" => "pillow",
        "sklearn" => "scikit-learn",
        "skimage" => "scikit-image",
        "google" => "protobuf",
        "attr" => "attrs",
        "dateutil" => "python-dateutil",
        other => other,
    }
    .to_string()
}

fn npu_rec(c: &Ctx, n: &Npu, priority: &str) -> Option<Recommendation> {
    let r = c.r;
    let active = n.status.starts_with("actif");
    let has_rt = !n.runtime_found.is_empty();
    let base = format!(
        "{} — {}{} ; runtime trouvé : {}.",
        n.name,
        n.status,
        n.datasheet_tops.map(|t| format!(", {} TOPS (constructeur)", t)).unwrap_or_default(),
        if has_rt { n.runtime_found.join(", ") } else { "aucun".into() }
    );
    let pyv = c.py_version().map(|(a, b)| format!("cp{}{}", a, b)).unwrap_or_else(|| "cp3X".into());
    let mut steps = Vec::new();
    let mut avoid = vec![s("Compter sur PyTorch/TensorFlow pour utiliser le NPU : il faut passer par le runtime du fabricant.")];
    let title;
    match n.vendor.as_deref()? {
        "Rockchip" => {
            title = "NPU Rockchip : chaîne RKNN";
            if n.status.contains("désactivé") {
                steps.push(s("Activer le NPU dans le device-tree (overlay « npu » via rsetup / armbian-config selon l'image)"));
            } else if !active {
                steps.push(s("Utiliser une image à noyau « vendor » Rockchip (driver rknpu) : images officielles Radxa / Orange Pi / Armbian vendor"));
            }
            if !c.lib("librknnrt.so") {
                steps.push(s("Sur la carte : copier librknnrt.so depuis github.com/airockchip/rknn-toolkit2 (rknpu2/runtime/Linux/librknn_api/aarch64/) vers /usr/lib/"));
            }
            if !c.py_pkg("rknn-toolkit-lite2") {
                steps.push(format!("Sur la carte : pip install rknn_toolkit_lite2-*-{}-*-aarch64.whl (wheel du même dépôt, dossier rknn-toolkit-lite2/packages)", pyv));
            }
            steps.push(s("Sur un PC Linux x86 : pip install rknn-toolkit2, puis convertir ONNX → .rknn avec quantification int8 (fournir ~100 images de calibration)"));
            steps.push(s("Modèles prêts et exemples (YOLO, etc.) : github.com/airockchip/rknn_model_zoo"));
            if c.compat("rk3588") || c.compat("rk3576") {
                steps.push(s("LLM sur NPU : github.com/airockchip/rknn-llm (RKLLM), conversion des modèles sur PC"));
            }
            steps.push(s("Charge du NPU en direct : sudo cat /sys/kernel/debug/rknpu/load"));
            avoid.push(s("Mélanger les versions : le runtime librknnrt, rknn-toolkit-lite2 et le toolkit de conversion doivent avoir la même version."));
            avoid.push(s("Exporter YOLO avec le script Ultralytics standard : utiliser les exports adaptés au NPU du rknn_model_zoo."));
        }
        "Hailo" => {
            title = "Accélérateur Hailo : chaîne HailoRT";
            if !has_rt {
                if c.compat("raspberrypi") {
                    steps.push(s("sudo apt install hailo-all   # Raspberry Pi OS : driver + HailoRT + plugins GStreamer"));
                } else {
                    steps.push(s("Installer le driver PCIe et HailoRT depuis hailo.ai/developer-zone (même version pour les deux)"));
                }
            }
            steps.push(s("Vérifier : hailortcli fw-control identify"));
            steps.push(s("Modèles précompilés (.hef) : github.com/hailo-ai/hailo_model_zoo — conversion ONNX → HEF avec le Dataflow Compiler (PC x86)"));
            if c.compat("raspberrypi") {
                steps.push(s("Exemples prêts : github.com/hailo-ai/hailo-rpi5-examples"));
            }
            avoid.push(s("Utiliser un .hef compilé pour une autre puce (Hailo-8 ≠ Hailo-8L) ou une autre version de HailoRT."));
        }
        "Google" => {
            title = "Coral Edge TPU : TFLite int8";
            if !c.lib("libedgetpu.so") {
                steps.push(s("Installer libedgetpu1-std (dépôt coral.ai/software)"));
            }
            steps.push(s("Modèles : TFLite quantifié int8 intégral, compilé avec edgetpu_compiler (PC x86 Debian/Ubuntu)"));
            steps.push(s("Exécution : tflite_runtime / ai-edge-litert avec le délégué libedgetpu.so.1"));
            if c.py_version().is_some_and(|v| v > (3, 9)) {
                avoid.push(format!("pycoral : wheels officielles seulement jusqu'à Python 3.9 (tu as {}.{}) ; utiliser le délégué via tflite directement ou un venv Python 3.9.", c.py_version().unwrap().0, c.py_version().unwrap().1));
            }
            avoid.push(s("Modèles float ou opérations non supportées : elles retombent sur le CPU (vérifier le rapport d'edgetpu_compiler)."));
        }
        "Intel" => {
            title = "NPU Intel : OpenVINO";
            if !c.py_pkg("openvino") {
                steps.push(s("pip install openvino"));
            }
            steps.push(s("Pilote userspace NPU : github.com/intel/linux-npu-driver (paquets .deb)"));
            steps.push(s("Utiliser le périphérique « NPU » : core.compile_model(model, \"NPU\")"));
        }
        "VeriSilicon" => {
            title = "NPU VeriSilicon/Vivante : runtime du BSP";
            steps.push(s("NXP i.MX : eIQ — TFLite avec le délégué libvx_delegate.so (fourni par le BSP Yocto)"));
            steps.push(s("Amlogic (Khadas VIM3…) : KSNN / TIM-VX fournis par le fabricant de la carte"));
            steps.push(s("Modèles quantifiés int8 obligatoires pour de bonnes performances"));
        }
        "NVIDIA" => {
            title = "DLA NVIDIA : TensorRT";
            steps.push(s("/usr/src/tensorrt/bin/trtexec --onnx=model.onnx --useDLACore=0 --allowGPUFallback --int8"));
        }
        "AMD" => {
            title = "NPU AMD Ryzen AI";
            steps.push(s("Installer Ryzen AI Software (XRT + ONNX Runtime Vitis AI EP), modèles ONNX quantifiés"));
        }
        _ => return None,
    }
    if r.python.is_none() {
        steps.push(s("Installer Python 3 : les SDK NPU passent quasiment tous par Python pour la conversion et les exemples"));
    }
    Some(rec("IA", priority, title, base, steps, avoid))
}

fn cuda(c: &Ctx, priority: &str) -> Recommendation {
    let r = c.r;
    let cuda = r.compute.cuda.clone().unwrap_or_default();
    let vram = r.gpus.iter().filter_map(|g| g.vram_mb).max().unwrap_or(0);
    let torch = c.framework("PyTorch");
    let torch_ok = torch.is_some_and(|t| t.accelerators.iter().any(|a| a.starts_with("cuda")));
    let mut steps = Vec::new();

    // Index PyTorch le plus récent compatible avec le driver.
    let driver_cuda: Option<(u32, u32)> = cuda.driver_cuda_version.as_deref().and_then(|v| {
        let mut it = v.split('.');
        Some((it.next()?.parse().ok()?, it.next().unwrap_or("0").parse().ok()?))
    });
    let index = [(12, 8, "cu128"), (12, 6, "cu126"), (12, 4, "cu124"), (12, 1, "cu121"), (11, 8, "cu118")]
        .iter()
        .find(|(a, b, _)| driver_cuda.is_some_and(|d| d >= (*a, *b)))
        .map(|x| x.2);
    if torch_ok {
        steps.push(format!("PyTorch voit déjà le GPU ({}) : rien à faire", torch.and_then(|t| t.version.clone()).unwrap_or_default()));
    } else if let Some(idx) = index {
        steps.push(format!("{} -m pip install torch torchvision --index-url https://download.pytorch.org/whl/{}", c.py(), idx));
    }
    if let Some(ort) = c.framework("ONNX Runtime") {
        if !ort.accelerators.iter().any(|a| a.contains("CUDA") || a.contains("Tensorrt")) {
            steps.push(format!("{0} -m pip uninstall -y onnxruntime && {0} -m pip install onnxruntime-gpu   # ONNX Runtime tourne actuellement sur CPU", c.py()));
        }
    }
    if cuda.tensorrt.is_none() {
        steps.push(format!("Optionnel, pour l'inférence la plus rapide : {} -m pip install tensorrt (puis conversion ONNX → moteur TensorRT)", c.py()));
    }
    if cuda.toolkit_version.is_none() {
        steps.push(s("Toolkit CUDA (nvcc) absent : inutile pour PyTorch/ONNX (ils embarquent leur runtime), nécessaire seulement pour compiler du code CUDA (llama.cpp -DGGML_CUDA=ON, extensions C++)"));
    }
    rec(
        "IA",
        priority,
        "GPU NVIDIA : voie CUDA",
        format!(
            "{} avec {} Mo de VRAM ; driver {} (CUDA ≤ {}) ; PyTorch {}.",
            r.gpus.iter().find(|g| g.vendor.as_deref() == Some("NVIDIA")).map(|g| g.name.as_str()).unwrap_or("GPU NVIDIA"),
            vram,
            cuda.driver_version.as_deref().unwrap_or("?"),
            cuda.driver_cuda_version.as_deref().unwrap_or("?"),
            if torch_ok { "voit le GPU" } else if torch.is_some() { "ne voit pas le GPU" } else { "absent" }
        ),
        steps,
        vec![
            format!("Installer un toolkit CUDA plus récent que ce que supporte le driver (CUDA ≤ {}).", cuda.driver_cuda_version.as_deref().unwrap_or("?")),
            s("Avoir onnxruntime et onnxruntime-gpu installés en même temps : conflit, seul le CPU est utilisé."),
        ],
    )
}

fn jetson(c: &Ctx, priority: &str) -> Recommendation {
    let r = c.r;
    let j = r.board.jetson.clone().unwrap_or_default();
    let mut steps = vec![
        s("Conteneurs prêts (PyTorch, TensorRT, LLM, vision) adaptés à ta version de JetPack : github.com/dusty-nv/jetson-containers"),
        s("PyTorch : utiliser les wheels NVIDIA pour JetPack (pas celles de PyPI)"),
        s("TensorRT : /usr/src/tensorrt/bin/trtexec --onnx=model.onnx --fp16 --saveEngine=model.engine"),
        s("Performances max : sudo nvpmodel -m 0 && sudo jetson_clocks"),
    ];
    if !c.tool("jtop") {
        steps.push(s("Supervision : sudo pip3 install jetson-stats puis jtop"));
    }
    rec(
        "IA",
        priority,
        "NVIDIA Jetson : CUDA + TensorRT",
        format!("Jetson détecté : L4T {}, JetPack {}.", j.l4t_release.as_deref().unwrap_or("?"), j.jetpack.as_deref().unwrap_or("?")),
        steps,
        vec![s("pip install torch depuis PyPI : sur aarch64 c'est une version CPU seulement."), s("Mettre à jour vers une autre version de JetPack avec apt dist-upgrade : réinstaller proprement (SDK Manager / image).")],
    )
}

// ---------------------------------------------------------------- LLM

fn llm(c: &Ctx, out: &mut Vec<Recommendation>) {
    let r = c.r;
    let Some(l) = &r.analysis.llm else { return };
    let examples = |p: f64| -> &'static str {
        if p <= 1.5 {
            "Qwen2.5 0.5B/1.5B, Llama 3.2 1B, Gemma 3 1B"
        } else if p <= 4.0 {
            "Llama 3.2 3B, Qwen2.5 3B, Phi-3.5 mini, Gemma 3 4B"
        } else if p <= 8.0 {
            "Qwen2.5 7B, Llama 3.1 8B, Mistral 7B"
        } else if p <= 14.0 {
            "Qwen2.5 14B, Phi-4 14B"
        } else {
            "Qwen2.5 32B et plus"
        }
    };
    let mut steps = Vec::new();
    let mut facts = format!("RAM disponible {} Go", l.ram_budget_gb);
    if let Some(bw) = l.measured_bandwidth_gbs {
        facts.push_str(&format!(", bande passante mémoire mesurée {} Go/s", bw));
    }

    // GPU NVIDIA : tout dans la VRAM.
    if let Some(v) = l.vram_budget_gb {
        if let Some(row) = l.rows.iter().rev().find(|x| x.fits_vram == Some(true)) {
            facts.push_str(&format!(", VRAM {} Go", v));
            steps.push(format!("Sur GPU : modèles jusqu'à {}B en Q4 entièrement en VRAM — ex. {}", row.params_b, examples(row.params_b)));
        }
    }
    let rockchip_llm = r.npus.iter().any(|n| n.vendor.as_deref() == Some("Rockchip")) && (c.compat("rk3588") || c.compat("rk3576"));
    if rockchip_llm {
        steps.push(s("Sur NPU Rockchip : RKLLM (github.com/airockchip/rknn-llm), modèles 0,5 à 7B convertis sur PC — souvent plus rapide et moins énergivore que le CPU"));
    }
    // CPU : confortable = borne haute ≥ 8 tokens/s (≈ 3 à 5 tokens/s réels).
    let comfortable = l.rows.iter().rev().find(|x| x.fits_ram && x.max_tokens_per_s.is_none_or(|t| t >= 8.0));
    let maximum = l.rows.iter().rev().find(|x| x.fits_ram);
    match (comfortable, maximum) {
        (_, None) => {
            out.push(rec("LLM", "optionnel", "LLM local : mémoire insuffisante", format!("{} : même un modèle 0,5B quantifié ne tient pas confortablement.", facts), vec![s("Utiliser un LLM distant (API) ou un PC/serveur du réseau local")], vec![]));
            return;
        }
        (Some(cf), Some(mx)) => {
            steps.push(format!("Sur CPU, confortable : jusqu'à {}B en Q4 (≤ {} tokens/s théoriques) — ex. {}", cf.params_b, cf.max_tokens_per_s.map(|t| t.to_string()).unwrap_or("?".into()), examples(cf.params_b)));
            if mx.params_b > cf.params_b {
                steps.push(format!("Sur CPU, maximum (lent) : {}B en Q4 ({:.1} Go)", mx.params_b, mx.size_gb));
            }
        }
        (None, Some(mx)) => steps.push(format!("Sur CPU : jusqu'à {}B tient en RAM mais sera lent (≤ {} tokens/s théoriques) — viser 0,5 à 1,5B", mx.params_b, mx.max_tokens_per_s.map(|t| t.to_string()).unwrap_or("?".into()))),
    }
    if c.tool("ollama") {
        let models = &r.ai.ollama_models;
        steps.push(if models.is_empty() { s("Ollama est installé : ollama run qwen2.5:1.5b (adapter la taille selon ci-dessus)") } else { format!("Ollama est installé, modèles présents : {}", models.join(", ")) });
    } else {
        steps.push(s("Le plus simple : Ollama — curl -fsSL https://ollama.com/install.sh | sh"));
    }
    let nvidia_nvcc = r.compute.cuda.as_ref().is_some_and(|c| c.toolkit_version.is_some());
    let vk_gpu = r.compute.vulkan_devices.iter().any(|d| d.device_type.as_deref() != Some("cpu"));
    let mut build = s("Le plus performant : llama.cpp compilé sur la machine — cmake -B build -DGGML_NATIVE=ON");
    if nvidia_nvcc {
        build.push_str(" -DGGML_CUDA=ON");
    } else if vk_gpu && !r.gpus.iter().any(|g| g.vendor.as_deref() == Some("NVIDIA")) {
        build.push_str(" -DGGML_VULKAN=ON");
    }
    build.push_str(" && cmake --build build -j");
    steps.push(build);
    out.push(rec(
        "LLM",
        "optionnel",
        "LLM en local : tailles adaptées à cette machine",
        format!("{}. Estimations calculées à partir de ces mesures (voir la section LLM).", facts),
        steps,
        vec![s("Les modèles non quantifiés (FP16) : 4 fois plus gros et plus lents qu'en Q4, rarement utile en local.")],
    ));
}

// ---------------------------------------------------------------- Python

fn python(c: &Ctx, out: &mut Vec<Recommendation>) {
    let Some(py) = &c.r.python else { return };
    let ver = c.py_version();
    let mut steps = Vec::new();
    let mut avoid = vec![s("sudo pip install : casse les paquets Python du système.")];
    let sys_site = if c.is_sbc { " --system-site-packages" } else { "" };
    if c.tool("uv") {
        steps.push(format!("uv venv{} .venv && source .venv/bin/activate   # uv est installé, 10 à 100× plus rapide que pip", sys_site));
    } else {
        steps.push(format!("python3 -m venv{} .venv && source .venv/bin/activate", sys_site));
        steps.push(s("Optionnel : curl -LsSf https://astral.sh/uv/install.sh | sh (gestionnaire Python beaucoup plus rapide)"));
    }
    if c.is_sbc {
        steps.push(s("--system-site-packages permet de réutiliser les paquets installés par apt (python3-opencv, picamera2, SDK constructeur…) dans le venv"));
    }
    if let Some((a, b)) = ver {
        if (a, b) >= (3, 13) && c.arch != "x86_64" {
            avoid.push(format!("Python {}.{} sur {} : beaucoup de wheels IA/NPU (rknn-lite, tflite-runtime…) arrivent tard — garder un venv 3.10/3.11 (uv python install 3.11).", a, b, c.arch));
        }
        if (a, b) < (3, 9) {
            avoid.push(format!("Python {}.{} est trop ancien pour la plupart des librairies IA récentes.", a, b));
        }
    }
    let rationale = format!(
        "Python {} ({}){}{}.",
        py.version.as_deref().unwrap_or("?"),
        py.executable,
        if py.externally_managed { " ; la distribution bloque pip hors environnement virtuel (PEP 668)" } else { "" },
        py.virtualenv.as_ref().map(|v| format!(" ; environnement actif : {}", v)).unwrap_or_default()
    );
    out.push(rec("Python", if py.externally_managed && py.virtualenv.is_none() { "essentiel" } else { "optionnel" }, "Un environnement virtuel par projet", rationale, steps, avoid));
}

// ---------------------------------------------------------------- Vidéo

fn video(c: &Ctx, out: &mut Vec<Recommendation>) {
    let r = c.r;
    let v = &r.video;
    let cams: Vec<&V4l2Device> = v.v4l2_devices.iter().filter(|d| d.roles.iter().any(|x| x == "capture")).collect();
    if v.hw_codecs.is_empty() && cams.is_empty() && v.vendor_nodes.iter().all(|n| n.contains("dma_heap")) {
        return;
    }
    let gst = v.gstreamer.as_ref().map(|g| g.hw_elements.clone()).unwrap_or_default();
    let has_el = |p: &str| gst.iter().find(|e| e.starts_with(p)).cloned();
    let mut steps = Vec::new();
    let mut avoid = Vec::new();

    let enc: Vec<String> = v.hw_codecs.iter().filter(|x| x.direction == "encode").map(|x| x.codec.clone()).collect();
    let dec: Vec<String> = v.hw_codecs.iter().filter(|x| x.direction == "decode").map(|x| x.codec.clone()).collect();
    for e in ["mpph264enc", "nvv4l2h264enc", "v4l2h264enc", "vah264enc", "vaapih264enc", "nvh264enc"] {
        if let Some(el) = has_el(e) {
            steps.push(format!("Encodage H.264 matériel (GStreamer) : gst-launch-1.0 videotestsrc num-buffers=300 ! videoconvert ! {} ! h264parse ! mp4mux ! filesink location=test.mp4", el));
            break;
        }
    }
    if let Some(e) = v.ffmpeg.as_ref().and_then(|f| f.working_encoders.first()) {
        steps.push(format!("Encodage matériel (FFmpeg, testé OK) : ffmpeg -i entree.mp4 -c:v {} sortie.mp4", e));
    }
    if v.vendor_nodes.iter().any(|n| n.contains("mpp_service")) && has_el("mpp").is_none() {
        steps.push(s("Codecs Rockchip (MPP) présents mais sans plugin : installer gstreamer1.0-rockchip1 (dépôts Radxa/Armbian) ou ffmpeg-rockchip (github.com/nyanmisaka/ffmpeg-rockchip)"));
    }
    if r.board.jetson.is_some() {
        steps.push(s("Jetson : éléments nvv4l2h264enc / nvv4l2decoder / nvvidconv, et DeepStream pour les pipelines IA vidéo"));
    }
    if c.compat("raspberrypi") {
        steps.push(s("Caméras Raspberry Pi : rpicam-apps et Picamera2 (sudo apt install python3-picamera2), pas OpenCV VideoCapture pour les caméras CSI"));
    } else if !cams.is_empty() {
        steps.push(format!("Capture : {} — OpenCV cv2.VideoCapture(\"{}\", cv2.CAP_V4L2) ou GStreamer v4l2src", cams.iter().map(|d| format!("{} ({})", d.path, d.name)).collect::<Vec<_>>().join(", "), cams[0].path));
    }
    if let Some(cv) = c.framework("OpenCV") {
        if cv.details.iter().any(|d| d.to_lowercase().starts_with("gstreamer") && d.contains("NO")) && (!gst.is_empty() || !v.hw_codecs.is_empty()) {
            avoid.push(s("opencv-python (pip) est compilé sans GStreamer : pour les pipelines matériels, utiliser python3-opencv du système (venv avec --system-site-packages) ou GStreamer directement."));
        }
    }
    if v.v4l2_devices.iter().any(|d| d.error.as_deref().is_some_and(|e| e.contains("video"))) {
        steps.insert(0, s("sudo usermod -aG video $USER   # puis se reconnecter : accès aux /dev/video* refusé actuellement"));
    }
    avoid.push(s("Encoder en logiciel (libx264) sur une carte qui a un encodeur matériel : CPU saturé et chauffe."));
    out.push(rec(
        "Vidéo",
        "recommandé",
        "Pipelines vidéo : utiliser les blocs matériels",
        format!(
            "Décodage matériel : {} ; encodage matériel : {} ; caméras/captures : {} ; éléments GStreamer matériels : {}.",
            if dec.is_empty() { "aucun exposé".into() } else { dec.join(", ") },
            if enc.is_empty() { "aucun exposé".into() } else { enc.join(", ") },
            cams.len(),
            if gst.is_empty() { "aucun".into() } else { gst.len().to_string() }
        ),
        steps,
        avoid,
    ));
}

// ---------------------------------------------------------------- E/S

fn io(c: &Ctx, out: &mut Vec<Recommendation>) {
    let r = c.r;
    let i = &r.interfaces;
    let usb_serial = i.uarts.iter().any(|u| u.dev.contains("ttyUSB") || u.dev.contains("ttyACM"));
    if !c.is_sbc && !usb_serial {
        return;
    }
    let mut steps = Vec::new();
    // Droits : groupes manquants pour les nœuds effectivement inaccessibles.
    let mut groups: Vec<String> = i
        .access
        .iter()
        .filter(|a| !a.writable && matches!(a.kind.as_str(), "GPIO" | "I2C" | "SPI" | "UART" | "Vidéo"))
        .filter_map(|a| a.group.clone())
        .filter(|g| g != "root" && !r.os.user_groups.contains(g))
        .collect();
    groups.sort();
    groups.dedup();
    if !groups.is_empty() {
        steps.push(format!("sudo usermod -aG {} $USER   # puis se reconnecter", groups.join(",")));
    }
    let root_only: Vec<String> = i.access.iter().filter(|a| !a.writable && a.group.as_deref() == Some("root") && matches!(a.kind.as_str(), "GPIO" | "I2C" | "SPI")).map(|a| a.node.clone()).collect();
    if !root_only.is_empty() {
        steps.push(format!(
            "Nœuds réservés à root ({}) : créer une règle udev, ex. echo 'SUBSYSTEM==\"gpio\", GROUP=\"gpio\", MODE=\"0660\"' | sudo tee /etc/udev/rules.d/60-gpio.rules (idem i2c-dev, spidev) puis sudo groupadd -f gpio && sudo usermod -aG gpio $USER",
            root_only.join(", ")
        ));
    }
    if !i.gpio_chips.is_empty() && !c.tool("gpiodetect") {
        steps.push(format!("{}   # gpiodetect / gpioinfo / gpioset", c.install_or(&[pkg("gpiod libgpiod-dev", "libgpiod-utils libgpiod-devel", "libgpiod", "libgpiod")], "installer libgpiod")));
    }
    if !i.i2c_buses.is_empty() && !c.tool("i2cdetect") {
        steps.push(c.install_or(&[pkg("i2c-tools", "i2c-tools", "i2c-tools", "i2c-tools")], "installer i2c-tools"));
    }
    if !i.i2c_buses.is_empty() {
        let n: Vec<String> = i.i2c_buses.iter().map(|b| b.dev.trim_start_matches("/dev/i2c-").to_string()).collect();
        steps.push(format!("Scanner un bus I2C : i2cdetect -y {}   (bus disponibles : {})", n[0], n.join(", ")));
    }
    let pi5 = c.compat("raspberrypi,5");
    let mut libs = vec![s("Python : gpiod (libgpiod v2), smbus2 (I2C), spidev (SPI), pyserial (UART)")];
    if c.compat("raspberrypi") {
        libs.push(s("Raspberry Pi : gpiozero + lgpio (API haut niveau, compatible Pi 5)"));
    }
    libs.push(s("Rust : crates gpiocdev, i2cdev, spidev, serialport (ou linux-embedded-hal pour les drivers embedded-hal)"));
    libs.push(s("C/C++ : libgpiod, <linux/i2c-dev.h>, <linux/spi/spidev.h>, termios"));
    steps.extend(libs);
    let disabled: Vec<String> = i.dt_peripherals.iter().filter(|p| p.disabled > 0 && matches!(p.kind.as_str(), "I2C" | "SPI" | "UART" | "PWM" | "CAN" | "Caméra MIPI-CSI")).map(|p| format!("{} ×{}", p.kind, p.disabled)).collect();
    if !disabled.is_empty() {
        let tool = ["rsetup", "armbian-config", "raspi-config", "dtoverlay"].into_iter().find(|t| c.tool(t));
        steps.push(format!(
            "Contrôleurs désactivés ({}) : les activer par overlay {}",
            disabled.join(", "),
            match tool {
                Some(t) => format!("avec {}", t),
                None => "(/boot/extlinux, /boot/armbianEnv.txt ou config.txt selon l'image)".into(),
            }
        ));
    }
    let mut avoid = vec![s("L'interface /sys/class/gpio (obsolète, retirée des noyaux récents) et wiringPi (abandonné)."), s("Brancher du 5 V sur les GPIO : niveaux logiques en 3,3 V (voire 1,8 V sur certains SoC) — vérifier le schéma de la carte.")];
    if pi5 {
        avoid.push(s("RPi.GPIO sur Raspberry Pi 5 : ne fonctionne pas avec le contrôleur RP1."));
    }
    let counts = format!(
        "GPIO ×{}, I2C ×{}, SPI ×{}, UART ×{}, PWM ×{}, CAN ×{} ; groupes de {} : {}.",
        i.gpio_chips.len(),
        i.i2c_buses.len(),
        i.spi_devices.len(),
        i.uarts.len(),
        i.pwm_chips.len(),
        i.can.len(),
        r.os.user.as_deref().unwrap_or("l'utilisateur"),
        if r.os.user_groups.is_empty() { "?".into() } else { r.os.user_groups.join(", ") }
    );
    out.push(rec("E/S", if groups.is_empty() && root_only.is_empty() { "recommandé" } else { "essentiel" }, "Électronique : accès et librairies pour GPIO / I2C / SPI / UART", counts, steps, avoid));
}

// ---------------------------------------------------------------- Conteneurs

fn containers(c: &Ctx, out: &mut Vec<Recommendation>) {
    let r = c.r;
    let docker = c.tool("docker");
    let mut steps = Vec::new();
    let mut prio = "optionnel";
    if docker {
        if r.dev.docker_daemon_access == Some(false) {
            steps.push(s("sudo usermod -aG docker $USER   # puis se reconnecter (ou démarrer le service : sudo systemctl enable --now docker)"));
            prio = "recommandé";
        }
        let nvidia = r.gpus.iter().any(|g| g.vendor.as_deref() == Some("NVIDIA"));
        if nvidia && !r.dev.docker_runtimes.iter().any(|x| x == "nvidia") {
            steps.push(s("GPU dans les conteneurs : installer nvidia-container-toolkit puis sudo nvidia-ctk runtime configure --runtime=docker && sudo systemctl restart docker"));
            prio = "recommandé";
        }
        if let Some((_, plat, _)) = c.targets() {
            if c.arch != "x86_64" {
                steps.push(format!("Utiliser des images multi-arch ou {} (vérifier avec docker manifest inspect <image>)", plat));
            }
        }
        if !r.dev.tools.iter().any(|t| t.name.starts_with("Docker Compose") || t.command == "docker-compose") {
            steps.push(c.install_or(&[pkg("docker-compose-plugin", "docker-compose-plugin", "docker-compose", "docker-cli-compose")], "installer le plugin docker compose"));
        }
    } else if c.ram_gb >= 2.0 {
        steps.push(s("curl -fsSL https://get.docker.com | sh && sudo usermod -aG docker $USER"));
    } else {
        return;
    }
    if steps.is_empty() {
        return;
    }
    out.push(rec(
        "Conteneurs",
        prio,
        if docker { "Docker : finaliser la configuration" } else { "Docker pour des environnements reproductibles" },
        format!(
            "Docker {} ; démon accessible : {} ; runtimes : {}.",
            if docker { "installé" } else { "absent" },
            match r.dev.docker_daemon_access {
                Some(true) => "oui",
                Some(false) => "non",
                None => "—",
            },
            if r.dev.docker_runtimes.is_empty() { "—".into() } else { r.dev.docker_runtimes.join(", ") }
        ),
        steps,
        vec![],
    ));
}

// ---------------------------------------------------------------- Déploiement

fn deployment(c: &Ctx, out: &mut Vec<Recommendation>) {
    let r = c.r;
    if !c.is_sbc {
        return;
    }
    let mut steps = vec![s("Lancer l'application comme service systemd (Restart=always) pour qu'elle redémarre après un crash ou une coupure")];
    if !r.interfaces.watchdogs.is_empty() {
        steps.push(format!("Watchdog matériel présent ({}) : RuntimeWatchdogSec=15 dans /etc/systemd/system.conf pour redémarrer la carte si elle se fige", r.interfaces.watchdogs.join(", ")));
    }
    if r.storage.root_device_kind.as_deref().is_some_and(|k| k.starts_with("Carte SD") || k.starts_with("eMMC")) {
        steps.push(s("Réduire les écritures : journald en mémoire (Storage=volatile) ou log2ram, et système en lecture seule (overlayroot) pour la production"));
    }
    let hot = r.cpu.temperature_c.filter(|&t| t >= 60.0);
    if let Some(t) = hot {
        steps.push(format!("Température CPU déjà à {:.0} °C au repos : prévoir dissipateur + ventilation avant des charges IA prolongées", t));
    }
    if r.cpu.governor.as_deref() == Some("powersave") {
        steps.push(s("Gouverneur « powersave » : passer en « schedutil » ou « performance » (cpufrequtils) si les performances comptent"));
    }
    steps.push(s("Surveiller la température et le throttling sous charge réelle (watch -n1 cat /sys/class/thermal/thermal_zone*/temp)"));
    out.push(rec(
        "Déploiement",
        "optionnel",
        "Préparer la carte pour tourner en continu",
        format!(
            "Watchdog : {} ; système sur {} ; température CPU : {}.",
            if r.interfaces.watchdogs.is_empty() { "aucun" } else { "présent" },
            r.storage.root_device_kind.as_deref().unwrap_or("?"),
            r.cpu.temperature_c.map(|t| format!("{:.0} °C", t)).unwrap_or("inconnue".into())
        ),
        steps,
        vec![s("Couper l'alimentation brutalement sur un système en écriture : risque de corruption du système de fichiers.")],
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sbc() -> Report {
        let mut r = Report::default();
        r.board.has_device_tree = true;
        r.board.dt_compatible = vec!["radxa,rock-5b".into(), "rockchip,rk3588".into()];
        r.os.arch = Some("aarch64".into());
        r.os.package_manager = Some("apt/dpkg".into());
        r.os.user_groups = vec!["arnaud".into()];
        r.memory.total_mb = 3800;
        r.memory.available_mb = 3000;
        r.cpu.logical_cores = 8;
        r.storage.root_device_kind = Some("Carte SD (mmcblk1)".into());
        r.npus.push(Npu { name: "Rockchip RK3588 NPU".into(), vendor: Some("Rockchip".into()), status: "actif (driver lié)".into(), ..Default::default() });
        r.interfaces.gpio_chips.push(GpioChip { dev: "/dev/gpiochip0".into(), ..Default::default() });
        r.interfaces.access.push(DevAccess { kind: "GPIO".into(), node: "/dev/gpiochip0".into(), group: Some("gpio".into()), writable: false });
        r
    }

    #[test]
    fn sbc_guide() {
        let recs = recommend(&sbc());
        let titles: Vec<&str> = recs.iter().map(|x| x.title.as_str()).collect();
        assert!(titles.contains(&"NPU Rockchip : chaîne RKNN"));
        assert!(titles.contains(&"Compiler sur un PC, déployer sur la carte"));
        assert!(titles.contains(&"Sortir le développement de la carte SD"));
        let io = recs.iter().find(|x| x.domain == "E/S").unwrap();
        assert_eq!(io.priority, "essentiel");
        assert!(io.steps.iter().any(|s| s.contains("usermod -aG gpio")));
        // Les essentiels d'abord.
        assert_eq!(recs[0].priority, "essentiel");
        // Le NPU est la voie IA principale, le CPU devient la voie de secours.
        let cpu = recs.iter().find(|x| x.title.starts_with("Inférence CPU")).unwrap();
        assert_eq!(cpu.priority, "optionnel");
    }

    #[test]
    fn apt_install_command() {
        let r = sbc();
        let c = Ctx::new(&r);
        assert_eq!(c.install(&[pkg("i2c-tools", "i2c-tools", "i2c-tools", "i2c-tools")]).as_deref(), Some("sudo apt install -y i2c-tools"));
    }

    #[test]
    fn pip_names() {
        assert_eq!(pip_name("cv2"), "opencv-python");
        assert_eq!(pip_name("six"), "six");
    }
}
