//! Texte compact à coller dans un assistant IA (Claude, GPT, Qwen…) pour qu'il
//! propose des choix techniques adaptés à cette machine précise.

use super::{gb, opt};
use crate::report::Report;
use std::fmt::Write;

pub fn render(r: &Report) -> String {
    let mut o = String::new();
    o.push_str("Voici le profil réel (mesuré par IntelliProbe) de la machine cible de mon projet.\n");
    o.push_str("Tiens compte uniquement de ce qui est présent, signale ce qu'il faudrait installer, et évite les solutions incompatibles avec cette architecture.\n\n");

    let _ = writeln!(o, "Carte : {}", opt(&r.board.model));
    if let Some(soc) = &r.board.soc {
        let _ = writeln!(o, "SoC : {}", crate::probes::dt::soc_display_name(soc));
    }
    let _ = writeln!(o, "OS : {} ; noyau {} ; arch {} ; userland {} bits ; {}", opt(&r.os.pretty_name), opt(&r.os.kernel), opt(&r.os.arch), r.os.userland_bits.map(|b| b.to_string()).unwrap_or("?".into()), opt(&r.os.libc));
    let _ = writeln!(o, "CPU : {} ; {} cœurs ; extensions : {}", opt(&r.cpu.model), r.cpu.logical_cores, r.cpu.isa_features.iter().map(|f| f.flag.as_str()).collect::<Vec<_>>().join(" "));
    let _ = writeln!(o, "RAM : {} (disponible {}) ; swap {}", gb(r.memory.total_mb), gb(r.memory.available_mb), gb(r.memory.swap_total_mb));
    let _ = writeln!(o, "Stockage système : {}", opt(&r.storage.root_device_kind));
    for g in &r.gpus {
        let _ = writeln!(o, "GPU : {} (driver {}{})", g.name, opt(&g.driver), g.vram_mb.map(|v| format!(", {} Mo", v)).unwrap_or_default());
    }
    for n in &r.npus {
        let _ = writeln!(o, "NPU : {} — {} ; runtime : {}", n.name, n.status, if n.runtime_found.is_empty() { "aucun".into() } else { n.runtime_found.join(", ") });
    }
    o.push_str("\nVoies d'accélération IA :\n");
    for p in &r.ai.paths {
        let _ = writeln!(o, "- {} : {} ({})", p.name, p.status, p.detail);
    }
    if let Some(py) = &r.python {
        let _ = writeln!(o, "\nPython {} ; frameworks :", opt(&py.version));
        for f in &py.frameworks {
            let _ = writeln!(o, "- {} {} : {}{}", f.name, opt(&f.version), if f.import_ok { "import ok" } else { "import en échec" }, if f.accelerators.is_empty() { String::new() } else { format!(", accélérateurs : {}", f.accelerators.join(", ")) });
        }
    }
    let codecs: Vec<String> = r.video.hw_codecs.iter().map(|c| format!("{} {}", c.codec, if c.direction == "encode" { "enc" } else { "dec" })).collect();
    if !codecs.is_empty() {
        let _ = writeln!(o, "\nCodecs vidéo matériels : {}", codecs.join(", "));
    }
    let io: Vec<String> = [
        ("GPIO", r.interfaces.gpio_chips.len()),
        ("I2C", r.interfaces.i2c_buses.len()),
        ("SPI", r.interfaces.spi_devices.len()),
        ("UART", r.interfaces.uarts.len()),
        ("CAN", r.interfaces.can.len()),
        ("PWM", r.interfaces.pwm_chips.len()),
    ]
    .iter()
    .filter(|(_, n)| *n > 0)
    .map(|(k, n)| format!("{} ×{}", k, n))
    .collect();
    if !io.is_empty() {
        let _ = writeln!(o, "Interfaces : {}", io.join(", "));
    }
    let langs: Vec<String> = r.dev.tools.iter().filter(|t| t.category == "Langages & compilateurs").map(|t| format!("{} {}", t.name, opt(&t.version))).collect();
    let _ = writeln!(o, "Langages : {}", langs.join(", "));
    let ai: Vec<String> = r.ai.runtimes.iter().filter(|t| t.category == "IA").map(|t| t.name.clone()).collect();
    if !ai.is_empty() {
        let _ = writeln!(o, "Outils IA : {}", ai.join(", "));
    }
    if let Some(b) = &r.bench {
        let _ = writeln!(o, "Mesures : CPU {} GFLOPS (1 cœur) / {} GFLOPS (tous) ; mémoire {} Go/s ; disque {}", b.cpu_single_gflops, b.cpu_multi_gflops, b.mem_read_multi_gbs, b.disk.as_ref().map(|d| format!("{} Mo/s en écriture", d.write_mbs)).unwrap_or("non mesuré".into()));
    }
    o.push_str("\nPoints d'attention :\n");
    for f in r.analysis.findings.iter().filter(|f| f.level == "warn" || f.level == "missing") {
        let _ = writeln!(o, "- {} {}", f.title, if f.detail.is_empty() { String::new() } else { format!("({})", f.detail) });
    }
    o.push_str("\nRecommandations déjà établies pour cette machine :\n");
    for rec in r.recommendations.iter().filter(|x| x.priority != "optionnel") {
        let _ = writeln!(o, "- [{}] {} : {}", rec.domain, rec.title, rec.rationale);
    }
    o.push_str("\nMon projet : [décris ici ce que tu veux faire]\n");
    o
}
