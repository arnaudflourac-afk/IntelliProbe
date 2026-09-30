//! Résumé coloré dans le terminal (le détail complet est dans les fichiers).

use super::{cluster_line, gb, opt, uptime};
use crate::report::Report;
use colored::Colorize;

fn section(title: &str) {
    println!("\n{}", format!("── {} ", title).bold().cyan());
}

fn kv(k: &str, v: impl AsRef<str>) {
    println!("  {:<16} {}", format!("{}:", k).dimmed(), v.as_ref());
}

pub fn print(r: &Report) {
    section("Carte & système");
    kv("Modèle", opt(&r.board.model));
    if let Some(soc) = &r.board.soc {
        kv("SoC", crate::probes::dt::soc_display_name(soc));
    }
    if let Some(j) = &r.board.jetson {
        kv("Jetson", format!("L4T {} / JetPack {}", opt(&j.l4t_release), opt(&j.jetpack)));
    }
    kv("OS", format!("{} — noyau {}", opt(&r.os.pretty_name), opt(&r.os.kernel)));
    kv(
        "Architecture",
        format!(
            "{}{}{}",
            opt(&r.os.arch),
            r.os.userland_bits.map(|b| format!(", userland {} bits", b)).unwrap_or_default(),
            r.os.libc.as_ref().map(|l| format!(", {}", l)).unwrap_or_default()
        ),
    );
    if let Some(u) = r.os.uptime_s {
        kv("Uptime", uptime(u));
    }

    section("CPU");
    kv("Modèle", opt(&r.cpu.model));
    for c in &r.cpu.clusters {
        kv("Cluster", cluster_line(c));
    }
    if !r.cpu.isa_features.is_empty() {
        kv("Extensions", r.cpu.isa_features.iter().map(|f| f.flag.as_str()).collect::<Vec<_>>().join(" "));
    }
    if let Some(t) = r.cpu.temperature_c {
        kv("Température", format!("{:.1} °C", t));
    }

    section("Mémoire & stockage");
    kv("RAM", format!("{} ({} disponibles)", gb(r.memory.total_mb), gb(r.memory.available_mb)));
    kv("Swap", if r.memory.swap_total_mb > 0 { gb(r.memory.swap_total_mb) } else { "aucun".into() });
    for d in &r.storage.devices {
        kv("Disque", format!("{} {} {:.0} Go{}", d.name, d.kind, d.size_gb, d.model.as_ref().map(|m| format!(" ({})", m)).unwrap_or_default()));
    }
    if let Some(k) = &r.storage.root_device_kind {
        kv("Système sur", k);
    }

    section("Accélérateurs");
    if r.gpus.is_empty() {
        kv("GPU", "aucun détecté".dimmed().to_string());
    }
    for g in &r.gpus {
        let mut extra = Vec::new();
        if let Some(v) = g.vram_mb {
            extra.push(format!("{} Mo VRAM", v));
        }
        if let Some(d) = &g.driver {
            extra.push(format!("driver {}", d));
        }
        if let Some(f) = g.max_freq_mhz {
            extra.push(format!("{} MHz max", f));
        }
        kv("GPU", format!("{}{}", g.name, if extra.is_empty() { String::new() } else { format!(" — {}", extra.join(", ")) }));
    }
    if r.npus.is_empty() {
        kv("NPU", "aucun détecté".dimmed().to_string());
    }
    for n in &r.npus {
        kv("NPU", format!("{} — {}{}", n.name, n.status, n.datasheet_tops.map(|t| format!(", {} TOPS (constructeur)", t)).unwrap_or_default()));
    }

    section("Voies d'accélération IA");
    for p in &r.ai.paths {
        let st = match p.status.as_str() {
            "utilisable" => p.status.green(),
            "partiel" => p.status.yellow(),
            _ => p.status.red(),
        };
        println!("  {:<12} {}", st, p.name.bold());
        println!("  {:<12} {}", "", p.detail.dimmed());
        if !p.bindings.is_empty() {
            println!("  {:<12} {}", "", format!("via : {}", p.bindings.join(", ")).dimmed());
        }
    }

    if let Some(py) = &r.python {
        section("Python");
        kv("Interpréteur", format!("{} {} ({} paquets)", py.executable, opt(&py.version), py.packages.len()));
        for fw in &py.frameworks {
            let state = if fw.error.is_some() {
                "échec import".red().to_string()
            } else if fw.import_ok {
                "ok".green().to_string()
            } else {
                "non testé".dimmed().to_string()
            };
            let acc = if fw.accelerators.is_empty() { String::new() } else { format!(" → {}", fw.accelerators.join(", ")) };
            kv(&fw.name, format!("{} [{}]{}", opt(&fw.version), state, acc));
        }
    }

    section("Développement");
    let langs: Vec<String> = r.dev.tools.iter().filter(|t| t.category == "Langages & compilateurs").map(|t| format!("{} {}", t.name, opt(&t.version))).collect();
    kv("Langages", langs.join(", "));
    if !r.dev.cross_compilers.is_empty() {
        kv("Compil. croisés", r.dev.cross_compilers.join(", "));
    }
    kv("Outils trouvés", format!("{} (sur {} recherchés)", r.dev.tools.len(), r.dev.tools.len() + r.dev.missing.len()));
    kv("Librairies", format!("{} partagées, {} remarquables, {} avec headers (pkg-config)", r.packages.shared_libs_count, r.packages.notable_libs.len(), r.packages.pkg_config.len()));

    if let Some(b) = &r.bench {
        section("Mesures");
        kv("CPU FP32", format!("{} GFLOPS (1 cœur) / {} GFLOPS ({} threads)", b.cpu_single_gflops, b.cpu_multi_gflops, b.threads));
        kv("Mémoire", format!("{} Go/s (1 thread) / {} Go/s (tous)", b.mem_read_single_gbs, b.mem_read_multi_gbs));
        if let Some(d) = &b.disk {
            kv("Disque", format!("écriture {} Mo/s, lecture {} Mo/s ({})", d.write_mbs, d.read_mbs.map(|v| v.to_string()).unwrap_or("—".into()), d.path));
        }
    }

    if let Some(l) = &r.analysis.llm {
        section("LLM quantifiés Q4 (estimation calculée)");
        let fits: Vec<String> = l.rows.iter().filter(|x| x.fits_ram).map(|x| format!("{}B", x.params_b)).collect();
        kv("En RAM", if fits.is_empty() { "aucun modèle testé".into() } else { format!("jusqu'à {}", fits.last().unwrap()) });
        if let Some(row) = l.rows.iter().find(|x| x.params_b == 7.0) {
            if let Some(t) = row.max_tokens_per_s {
                kv("7B Q4 CPU", format!("≤ {} tokens/s (borne haute)", t));
            }
        }
    }

    section("Guide de développement");
    for rec in &r.recommendations {
        let tag = match rec.priority.as_str() {
            "essentiel" => "ESSENTIEL ".red().bold(),
            "recommandé" => "CONSEILLÉ ".yellow().bold(),
            _ => "OPTIONNEL ".dimmed(),
        };
        println!("\n  {}{} {}", tag, format!("[{}]", rec.domain).cyan(), rec.title.bold());
        println!("     {}", rec.rationale.dimmed());
        // Les optionnels restent courts dans le terminal : le détail est dans les rapports.
        if rec.priority != "optionnel" {
            for st in &rec.steps {
                println!("     → {}", st);
            }
            for a in &rec.avoid {
                println!("     {} {}", "✘ à éviter :".red(), a);
            }
        }
    }

    section("Constats");
    for f in &r.analysis.findings {
        let tag = match f.level.as_str() {
            "ok" => "  ✔".green(),
            "warn" => "  ⚠".yellow(),
            "missing" => "  ✘".red(),
            _ => "  ℹ".blue(),
        };
        println!("{} {}", tag, f.title);
        if !f.detail.is_empty() {
            println!("     {}", f.detail.dimmed());
        }
        if let Some(h) = &f.hint {
            println!("     → {}", h);
        }
    }
}
