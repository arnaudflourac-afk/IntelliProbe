//! IntelliProbe : profil réel d'une carte SBC ou d'une station
//! (matériel, accélérateurs IA, environnements de dev, librairies).

// Les sondes remplissent les rapports champ par champ, plus lisible ici.
#![allow(clippy::field_reassign_with_default)]

pub mod analysis;
pub mod output;
pub mod probes;
pub mod report;
pub mod util;

use report::Report;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

pub struct Options<'a> {
    pub bench: bool,
    pub python_import: bool,
    /// Dossier où mesurer le débit disque.
    pub bench_dir: &'a Path,
    pub log: &'a (dyn Fn(&str) + Sync),
}

/// Exécute toutes les sondes et l'analyse.
pub fn collect(opts: &Options) -> Report {
    let t0 = Instant::now();
    let timings = Mutex::new(Vec::new());
    let timed = |name: &str, f: &mut dyn FnMut()| {
        let t = Instant::now();
        f();
        timings.lock().unwrap().push((name.to_string(), util::round2(t.elapsed().as_secs_f64())));
    };
    let mut r = Report::default();

    // Les sondes indépendantes tournent en parallèle ; les lentes (Python, outils) à part.
    std::thread::scope(|s| {
        let hw = s.spawn(|| {
            let mut r = Report::default();
            (opts.log)("matériel (carte, CPU, mémoire, GPU, NPU, interfaces)");
            timed("carte + OS", &mut || {
                r.board = probes::system::probe_board();
                r.os = probes::system::probe_os();
            });
            timed("CPU", &mut || r.cpu = probes::cpu::probe());
            timed("mémoire + stockage", &mut || {
                r.memory = probes::storage::probe_memory();
                r.storage = probes::storage::probe_storage();
            });
            timed("interfaces", &mut || {
                r.interfaces = probes::interfaces::probe();
                r.interfaces.sensors = probes::cpu::read_sensors();
            });
            timed("GPU", &mut || r.gpus = probes::gpu::probe(&r.interfaces.sensors));
            timed("NPU", &mut || r.npus = probes::npu::probe());
            (opts.log)("API de calcul et vidéo");
            timed("API de calcul", &mut || r.compute = probes::compute::probe());
            timed("vidéo", &mut || r.video = probes::video::probe());
            r
        });
        let libs = s.spawn(|| {
            (opts.log)("librairies (ldconfig, pkg-config, npm, cargo)");
            let mut p = Default::default();
            timed("librairies", &mut || p = probes::libs::probe());
            p
        });
        let dev = s.spawn(|| {
            (opts.log)("outils de développement");
            let mut d = Default::default();
            timed("outils de dev", &mut || d = probes::devtools::probe());
            let mut ai = Default::default();
            timed("outils IA", &mut || ai = probes::ai::probe());
            (d, ai)
        });
        let py = s.spawn(|| {
            (opts.log)("Python (paquets et frameworks IA)");
            let mut py = None;
            timed("Python", &mut || py = probes::python::probe(opts.python_import, &|m| (opts.log)(&format!("python : {}", m))));
            py
        });

        let hw = hw.join().unwrap_or_default();
        r.board = hw.board;
        r.os = hw.os;
        r.cpu = hw.cpu;
        r.memory = hw.memory;
        r.storage = hw.storage;
        r.interfaces = hw.interfaces;
        r.gpus = hw.gpus;
        r.npus = hw.npus;
        r.compute = hw.compute;
        r.video = hw.video;
        r.packages = libs.join().unwrap_or_default();
        if let Ok((d, (ai, ai_missing))) = dev.join() {
            r.dev = d;
            r.dev.missing.extend(ai_missing);
            r.ai = ai;
        }
        r.python = py.join().unwrap_or_default();
    });

    // Codecs VA-API réellement annoncés par le driver.
    if let Some(va) = &r.compute.vaapi {
        for c in probes::compute::vaapi_codecs(va) {
            if !r.video.hw_codecs.iter().any(|x| x.codec == c.codec && x.direction == c.direction) {
                r.video.hw_codecs.push(c);
            }
        }
    }

    // Mesures seules sur la machine, après les sondes, pour ne pas être perturbées.
    if opts.bench {
        (opts.log)("mesures (CPU, mémoire, disque) : quelques secondes");
        let avail = r.memory.available_mb;
        let mut b = None;
        timed("mesures", &mut || b = Some(probes::bench::run(opts.bench_dir, avail)));
        r.bench = b;
    }

    analysis::analyze(&mut r);

    r.meta.tool_version = env!("CARGO_PKG_VERSION").to_string();
    r.meta.generated_at = util::now_iso8601();
    r.meta.duration_s = util::round1(t0.elapsed().as_secs_f64());
    r.meta.run_as_root = is_root();
    r.meta.probe_timings = timings.into_inner().unwrap_or_default();
    r
}

#[cfg(unix)]
fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(unix))]
fn is_root() -> bool {
    false
}
