//! Rapport Markdown complet (lisible sur GitHub/GitLab, versionnable).

use super::{cluster_line, gb, opt, uptime};
use crate::report::Report;
use std::fmt::Write;

fn esc(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

fn table(out: &mut String, headers: &[&str], rows: Vec<Vec<String>>) {
    if rows.is_empty() {
        out.push_str("_Aucun élément._\n\n");
        return;
    }
    let _ = writeln!(out, "| {} |", headers.join(" | "));
    let _ = writeln!(out, "|{}|", headers.iter().map(|_| "---").collect::<Vec<_>>().join("|"));
    for r in rows {
        let _ = writeln!(out, "| {} |", r.iter().map(|c| esc(c)).collect::<Vec<_>>().join(" | "));
    }
    out.push('\n');
}

fn details(out: &mut String, summary: &str, body: &str) {
    let _ = write!(out, "<details><summary>{}</summary>\n\n{}\n</details>\n\n", summary, body);
}

pub fn render(r: &Report) -> String {
    let mut o = String::new();
    let host = opt(&r.os.hostname);
    let _ = writeln!(o, "# IntelliProbe — {}\n", host);
    let _ = writeln!(o, "Rapport généré le {} par IntelliProbe {} en {:.1} s{}.\n", r.meta.generated_at, r.meta.tool_version, r.meta.duration_s, if r.meta.run_as_root { " (root)" } else { "" });

    // Guide en tête : c'est ce qu'on lit en premier.
    o.push_str("## Guide de développement\n\n");
    for rec in &r.recommendations {
        let icon = match rec.priority.as_str() {
            "essentiel" => "🔴",
            "recommandé" => "🟠",
            _ => "⚪",
        };
        let _ = writeln!(o, "### {} {} — {}\n", icon, rec.domain, rec.title);
        let _ = writeln!(o, "_{} · {}_\n", rec.priority, rec.rationale);
        for st in &rec.steps {
            let _ = writeln!(o, "- {}", st);
        }
        for a in &rec.avoid {
            let _ = writeln!(o, "- ❌ **À éviter :** {}", a);
        }
        o.push('\n');
    }

    o.push_str("## Constats\n\n");
    for f in &r.analysis.findings {
        let icon = match f.level.as_str() {
            "ok" => "✅",
            "warn" => "⚠️",
            "missing" => "❌",
            _ => "ℹ️",
        };
        let _ = write!(o, "- {} **{}** — {}", icon, f.title, f.detail);
        if let Some(h) = &f.hint {
            let _ = write!(o, "  \n  → {}", h);
        }
        o.push('\n');
    }
    o.push('\n');

    o.push_str("## Voies d'accélération IA\n\n");
    table(
        &mut o,
        &["Voie", "État", "Matériel", "Driver", "Runtime", "Utilisée par", "Détail"],
        r.ai.paths
            .iter()
            .map(|p| {
                let yn = |b: bool| if b { "oui" } else { "non" }.to_string();
                vec![p.name.clone(), p.status.clone(), yn(p.hardware), yn(p.driver), yn(p.runtime), p.bindings.join(", "), p.detail.clone()]
            })
            .collect(),
    );

    // --- Carte / OS
    o.push_str("## Carte et système\n\n");
    let mut rows = vec![
        vec!["Modèle".into(), opt(&r.board.model).into()],
        vec!["SoC".into(), r.board.soc.as_deref().map(crate::probes::dt::soc_display_name).unwrap_or("—".into())],
        vec!["Compatible (device-tree)".into(), r.board.dt_compatible.join(", ")],
        vec!["Firmware".into(), opt(&r.board.firmware).into()],
        vec!["OS".into(), opt(&r.os.pretty_name).into()],
        vec!["Noyau".into(), opt(&r.os.kernel).into()],
        vec!["Architecture".into(), format!("{} (userland {} bits, paquets {})", opt(&r.os.arch), r.os.userland_bits.map(|b| b.to_string()).unwrap_or("?".into()), opt(&r.os.package_arch))],
        vec!["libc".into(), opt(&r.os.libc).into()],
        vec!["Init".into(), opt(&r.os.init).into()],
        vec!["Virtualisation".into(), r.os.virtualization.clone().unwrap_or("aucune (matériel réel)".into())],
        vec!["Paquets installés".into(), match (&r.os.package_manager, r.os.installed_packages) { (Some(m), Some(n)) => format!("{} ({})", n, m), _ => "—".into() }],
        vec!["Uptime".into(), r.os.uptime_s.map(uptime).unwrap_or("—".into())],
        vec!["Charge".into(), r.os.load_avg.map(|l| format!("{} {} {}", l[0], l[1], l[2])).unwrap_or("—".into())],
        vec!["Bureau".into(), opt(&r.os.desktop).into()],
        vec!["Shell".into(), opt(&r.os.shell).into()],
    ];
    if let Some(j) = &r.board.jetson {
        rows.push(vec!["Jetson".into(), format!("L4T {}, JetPack {}", opt(&j.l4t_release), opt(&j.jetpack))]);
    }
    if let Some(rev) = &r.board.raspberry_pi_revision {
        rows.push(vec!["Révision Raspberry Pi".into(), rev.clone()]);
    }
    table(&mut o, &["Élément", "Valeur"], rows);

    // --- CPU
    o.push_str("## CPU\n\n");
    let mut rows = vec![
        vec!["Modèle".into(), opt(&r.cpu.model).into()],
        vec!["Fabricant".into(), opt(&r.cpu.vendor).into()],
        vec!["Cœurs".into(), format!("{} logiques{}", r.cpu.logical_cores, r.cpu.physical_cores.map(|p| format!(", {} physiques", p)).unwrap_or_default())],
        vec!["Gouverneur".into(), opt(&r.cpu.governor).into()],
        vec!["Température".into(), r.cpu.temperature_c.map(|t| format!("{:.1} °C", t)).unwrap_or("—".into())],
    ];
    for c in &r.cpu.clusters {
        rows.push(vec!["Cluster".into(), cluster_line(c)]);
    }
    table(&mut o, &["Élément", "Valeur"], rows);
    table(&mut o, &["Extension", "Intérêt"], r.cpu.isa_features.iter().map(|f| vec![format!("`{}`", f.flag), f.description.clone()]).collect());
    details(&mut o, &format!("Tous les flags CPU ({})", r.cpu.all_flags.len()), &r.cpu.all_flags.join(" "));

    // --- Mémoire / stockage
    o.push_str("## Mémoire et stockage\n\n");
    let mut rows = vec![
        vec!["RAM totale".into(), gb(r.memory.total_mb)],
        vec!["RAM disponible".into(), gb(r.memory.available_mb)],
        vec!["Swap".into(), format!("{} ({} libres)", gb(r.memory.swap_total_mb), gb(r.memory.swap_free_mb))],
    ];
    for z in &r.memory.zram {
        rows.push(vec![format!("zram {}", z.name), format!("{} Mo, {}", z.disksize_mb, opt(&z.algorithm))]);
    }
    rows.push(vec!["Système installé sur".into(), opt(&r.storage.root_device_kind).into()]);
    table(&mut o, &["Élément", "Valeur"], rows);
    table(
        &mut o,
        &["Disque", "Type", "Taille", "Modèle", "Amovible"],
        r.storage.devices.iter().map(|d| vec![d.name.clone(), d.kind.clone(), format!("{:.1} Go", d.size_gb), opt(&d.model).into(), if d.removable { "oui" } else { "non" }.into()]).collect(),
    );
    table(
        &mut o,
        &["Montage", "Périphérique", "FS", "Total", "Libre"],
        r.storage.filesystems.iter().map(|f| vec![f.mount.clone(), f.device.clone(), f.fstype.clone(), format!("{:.1} Go", f.total_gb), format!("{:.1} Go", f.free_gb)]).collect(),
    );

    // --- GPU / NPU / calcul
    o.push_str("## GPU\n\n");
    table(
        &mut o,
        &["Nom", "Driver", "VRAM", "Fréq. max", "Temp.", "Source"],
        r.gpus
            .iter()
            .map(|g| {
                vec![
                    g.name.clone(),
                    format!("{} {}", opt(&g.driver), g.driver_version.as_deref().unwrap_or("")).trim().to_string(),
                    g.vram_mb.map(|v| format!("{} Mo", v)).unwrap_or("—".into()),
                    g.max_freq_mhz.map(|v| format!("{} MHz", v)).unwrap_or("—".into()),
                    g.temperature_c.map(|v| format!("{:.0} °C", v)).unwrap_or("—".into()),
                    g.source.clone(),
                ]
            })
            .collect(),
    );
    o.push_str("## NPU / accélérateurs IA\n\n");
    table(
        &mut o,
        &["Nom", "État", "Driver", "Nœuds", "Runtime trouvé", "TOPS (constructeur)", "Source"],
        r.npus
            .iter()
            .map(|n| {
                vec![
                    n.name.clone(),
                    n.status.clone(),
                    format!("{} {}", opt(&n.driver), n.driver_version.as_deref().unwrap_or("")).trim().to_string(),
                    n.device_nodes.join(", "),
                    if n.runtime_found.is_empty() { "aucun".into() } else { n.runtime_found.join(", ") },
                    n.datasheet_tops.map(|t| t.to_string()).unwrap_or("—".into()),
                    n.source.clone(),
                ]
            })
            .collect(),
    );
    o.push_str("## API de calcul\n\n");
    let c = &r.compute;
    let mut rows = Vec::new();
    if let Some(cu) = &c.cuda {
        rows.push(vec!["CUDA".into(), format!("driver {}, CUDA max {}, toolkit {}, cuDNN {}, TensorRT {}", opt(&cu.driver_version), opt(&cu.driver_cuda_version), opt(&cu.toolkit_version), opt(&cu.cudnn), opt(&cu.tensorrt))]);
    }
    for p in &c.opencl_platforms {
        rows.push(vec!["OpenCL".into(), format!("{} : {}", p.name, p.devices.join(", "))]);
    }
    if !c.opencl_icds.is_empty() {
        rows.push(vec!["OpenCL ICD".into(), c.opencl_icds.join(", ")]);
    }
    for d in &c.vulkan_devices {
        rows.push(vec!["Vulkan".into(), format!("{} ({}, API {}, {})", d.name, opt(&d.device_type), opt(&d.api_version), opt(&d.driver))]);
    }
    if !c.vulkan_icds.is_empty() {
        rows.push(vec!["Vulkan ICD".into(), c.vulkan_icds.join(", ")]);
    }
    if let Some(v) = &c.rocm_version {
        rows.push(vec!["ROCm".into(), v.clone()]);
    }
    if let Some(va) = &c.vaapi {
        rows.push(vec!["VA-API".into(), format!("{} — {}", opt(&va.driver), va.profiles.join(", "))]);
    }
    table(&mut o, &["API", "Détail"], rows);

    // --- Vidéo
    o.push_str("## Vidéo\n\n");
    table(
        &mut o,
        &["Codec", "Sens", "Backend", "Périphérique"],
        r.video.hw_codecs.iter().map(|c| vec![c.codec.clone(), if c.direction == "encode" { "encodage" } else { "décodage" }.into(), c.backend.clone(), opt(&c.device).into()]).collect(),
    );
    table(
        &mut o,
        &["Périphérique", "Nom", "Driver", "Rôles", "Formats produits", "Formats consommés"],
        r.video
            .v4l2_devices
            .iter()
            .map(|d| vec![d.path.clone(), d.name.clone(), d.error.clone().unwrap_or_else(|| opt(&d.driver).into()), d.roles.join(", "), d.capture_formats.join(" "), d.output_formats.join(" ")])
            .collect(),
    );
    let mut rows: Vec<Vec<String>> = r.video.vendor_nodes.iter().map(|n| vec!["Nœud constructeur".into(), n.clone()]).collect();
    if let Some(f) = &r.video.ffmpeg {
        rows.push(vec!["FFmpeg".into(), format!("{} — encodeurs matériels testés avec succès : {} ; hwaccels : {} ; encodeurs HW compilés : {} ; décodeurs HW compilés : {}", opt(&f.version), if f.working_encoders.is_empty() { "aucun".into() } else { f.working_encoders.join(", ") }, f.hwaccels.join(", "), f.hw_encoders.join(", "), f.hw_decoders.join(", "))]);
    }
    if let Some(g) = &r.video.gstreamer {
        rows.push(vec!["GStreamer".into(), format!("{} — éléments matériels : {}", opt(&g.version), g.hw_elements.join(", "))]);
    }
    rows.push(vec!["libcamera".into(), if r.video.libcamera { "présent" } else { "absent" }.into()]);
    table(&mut o, &["Élément", "Détail"], rows);

    // --- Interfaces
    o.push_str("## Interfaces matérielles\n\n");
    let i = &r.interfaces;
    let mut rows = Vec::new();
    for g in &i.gpio_chips {
        rows.push(vec!["GPIO".into(), format!("{} {} {}", g.dev, opt(&g.label), g.lines.map(|l| format!("({} lignes)", l)).unwrap_or_default())]);
    }
    for b in &i.i2c_buses {
        rows.push(vec!["I2C".into(), format!("{} {}", b.dev, opt(&b.name))]);
    }
    for s in &i.spi_devices {
        rows.push(vec!["SPI".into(), s.clone()]);
    }
    for u in &i.uarts {
        rows.push(vec!["UART".into(), format!("{} ({})", u.dev, opt(&u.name))]);
    }
    for c in &i.can {
        rows.push(vec!["CAN".into(), c.clone()]);
    }
    for p in &i.pwm_chips {
        rows.push(vec!["PWM".into(), format!("{} {}", p.dev, opt(&p.name))]);
    }
    for w in &i.watchdogs {
        rows.push(vec!["Watchdog".into(), w.clone()]);
    }
    for b in &i.bluetooth {
        rows.push(vec!["Bluetooth".into(), b.clone()]);
    }
    for n in &i.network {
        rows.push(vec![n.kind.clone(), format!("{} — {}{}{}", n.name, opt(&n.state), n.speed_mbps.map(|s| format!(", {} Mb/s", s)).unwrap_or_default(), n.driver.as_ref().map(|d| format!(", driver {}", d)).unwrap_or_default())]);
    }
    table(&mut o, &["Type", "Détail"], rows);
    if !i.dt_peripherals.is_empty() {
        o.push_str("### Contrôleurs déclarés dans le device-tree\n\n");
        table(&mut o, &["Contrôleur", "Activés", "Désactivés"], i.dt_peripherals.iter().map(|p| vec![p.kind.clone(), p.enabled.to_string(), p.disabled.to_string()]).collect());
    }
    o.push_str("### USB\n\n");
    table(
        &mut o,
        &["ID", "Fabricant", "Produit", "Vitesse"],
        i.usb_devices.iter().filter(|u| !u.is_hub).map(|u| vec![u.id.clone(), opt(&u.manufacturer).into(), opt(&u.product).into(), u.speed_mbps.map(|s| format!("{} Mb/s", s)).unwrap_or("—".into())]).collect(),
    );
    o.push_str("### PCI\n\n");
    table(
        &mut o,
        &["Slot", "ID", "Classe", "Description", "Driver", "Lien"],
        i.pci_devices.iter().map(|p| vec![p.slot.clone(), p.id.clone(), p.class.clone(), opt(&p.description).into(), opt(&p.driver).into(), opt(&p.link).into()]).collect(),
    );
    o.push_str("### Capteurs de température\n\n");
    table(&mut o, &["Capteur", "°C"], i.sensors.iter().map(|s| vec![s.name.clone(), format!("{:.1}", s.temp_c)]).collect());

    // --- Dev
    o.push_str("## Environnement de développement\n\n");
    table(
        &mut o,
        &["Catégorie", "Outil", "Version", "Chemin"],
        r.dev.tools.iter().chain(r.ai.runtimes.iter()).map(|t| vec![t.category.clone(), t.name.clone(), opt(&t.version).into(), opt(&t.path).into()]).collect(),
    );
    if !r.dev.cross_compilers.is_empty() {
        let _ = writeln!(o, "**Compilateurs croisés :** {}\n", r.dev.cross_compilers.join(", "));
    }
    if let Some(a) = r.dev.docker_daemon_access {
        let _ = writeln!(o, "**Démon Docker accessible :** {}{}\n", if a { "oui" } else { "non" }, if r.dev.docker_runtimes.is_empty() { String::new() } else { format!(" (runtimes : {})", r.dev.docker_runtimes.join(", ")) });
    }
    if !r.dev.binfmt.is_empty() {
        let _ = writeln!(o, "**Émulation binfmt (builds multi-arch) :** {}\n", r.dev.binfmt.join(", "));
    }
    if !r.ai.ollama_models.is_empty() {
        let _ = writeln!(o, "**Modèles Ollama :** {}\n", r.ai.ollama_models.join(", "));
    }
    let missing: Vec<String> = r.dev.missing.iter().map(|t| format!("{} ({})", t.name, t.category)).collect();
    details(&mut o, &format!("Outils recherchés mais absents ({})", missing.len()), &missing.join(", "));

    // --- Python
    if let Some(py) = &r.python {
        let _ = writeln!(o, "## Python {}\n", opt(&py.version));
        let _ = writeln!(o, "Interpréteur : `{}`{}\n", py.executable, py.virtualenv.as_ref().map(|v| format!(" (environnement : `{}`)", v)).unwrap_or_default());
        table(
            &mut o,
            &["Framework", "Version", "Import", "Accélérateurs vus", "Détails"],
            py.frameworks
                .iter()
                .map(|f| vec![f.name.clone(), opt(&f.version).into(), if f.import_ok { "ok".into() } else { f.error.clone().unwrap_or("non testé".into()) }, f.accelerators.join(", "), f.details.join(" ; ")])
                .collect(),
        );
        let list: String = py.packages.iter().map(|p| format!("- {} {}\n", p.name, opt(&p.version))).collect();
        details(&mut o, &format!("Tous les paquets Python ({})", py.packages.len()), &list);
    }

    // --- Librairies
    o.push_str("## Librairies\n\n");
    let _ = writeln!(o, "{} librairies partagées dans le cache système.\n", r.packages.shared_libs_count);
    table(&mut o, &["Catégorie", "Librairie", "Description", "Chemin"], r.packages.notable_libs.iter().map(|l| vec![l.category.clone(), l.soname.clone(), l.description.clone(), l.path.clone()]).collect());
    let pc: String = r.packages.pkg_config.iter().map(|p| format!("- **{}** {} — {}\n", p.name, opt(&p.version), opt(&p.description))).collect();
    details(&mut o, &format!("Librairies de développement (pkg-config, headers installés) : {}", r.packages.pkg_config.len()), &pc);
    if !r.packages.node_global.is_empty() {
        let l: String = r.packages.node_global.iter().map(|p| format!("- {} {}\n", p.name, opt(&p.version))).collect();
        details(&mut o, &format!("Paquets npm globaux ({})", r.packages.node_global.len()), &l);
    }
    if !r.packages.cargo_installed.is_empty() {
        let l: String = r.packages.cargo_installed.iter().map(|p| format!("- {} {}\n", p.name, opt(&p.version))).collect();
        details(&mut o, &format!("Binaires cargo ({})", r.packages.cargo_installed.len()), &l);
    }
    details(&mut o, &format!("Toutes les librairies partagées ({})", r.packages.all_shared_libs.len()), &r.packages.all_shared_libs.join(" "));

    // --- Mesures
    if let Some(b) = &r.bench {
        o.push_str("## Mesures\n\n");
        let mut rows = vec![
            vec!["CPU FP32, 1 cœur".into(), format!("{} GFLOPS", b.cpu_single_gflops)],
            vec![format!("CPU FP32, {} threads", b.threads), format!("{} GFLOPS", b.cpu_multi_gflops)],
            vec!["Lecture mémoire, 1 thread".into(), format!("{} Go/s", b.mem_read_single_gbs)],
            vec!["Lecture mémoire, tous threads".into(), format!("{} Go/s", b.mem_read_multi_gbs)],
        ];
        if let Some(d) = &b.disk {
            rows.push(vec![format!("Disque ({}, {} Mo)", d.path, d.size_mb), format!("écriture {} Mo/s, lecture {} Mo/s", d.write_mbs, d.read_mbs.map(|v| v.to_string()).unwrap_or("—".into()))]);
        }
        table(&mut o, &["Mesure", "Résultat"], rows);
        o.push_str("_CPU : noyau FP32 multiplication-addition vectorisé par le compilateur (SIMD de base de l'architecture, sans AVX2/AVX-512 spécifiques). Mémoire : lecture séquentielle de gros tampons (ce que fait un LLM à chaque token). Disque : écriture synchronisée (fsync), relecture après éviction du cache._\n\n");
    }
    if let Some(l) = &r.analysis.llm {
        o.push_str("## LLM quantifiés (estimation calculée)\n\n");
        let _ = writeln!(o, "RAM disponible : {} Go{}.\n", l.ram_budget_gb, l.vram_budget_gb.map(|v| format!(" — VRAM max : {} Go", v)).unwrap_or_default());
        table(
            &mut o,
            &["Paramètres", "Taille Q4", "Tient en RAM", "Tient en VRAM", "Débit CPU max"],
            l.rows
                .iter()
                .map(|x| {
                    vec![
                        format!("{}B", x.params_b),
                        format!("{:.2} Go", x.size_gb),
                        if x.fits_ram { "oui" } else { "non" }.into(),
                        x.fits_vram.map(|v| if v { "oui" } else { "non" }.to_string()).unwrap_or("—".into()),
                        x.max_tokens_per_s.map(|t| format!("≤ {} tok/s", t)).unwrap_or("—".into()),
                    ]
                })
                .collect(),
        );
        let _ = writeln!(o, "_{}_\n", l.method);
    }
    o
}
