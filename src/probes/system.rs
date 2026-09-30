//! Carte (modèle, SoC, firmware) et système d'exploitation.

use super::dt;
use crate::report::{Board, Jetson, Os};
use crate::util::*;
use std::fs;
use std::path::Path;

const DMI: &str = "/sys/class/dmi/id";

pub fn probe_board() -> Board {
    let mut b = Board { has_device_tree: dt::present(), ..Default::default() };

    if b.has_device_tree {
        b.model = read_trim(format!("{}/model", dt::DT_ROOT));
        b.dt_compatible = read_nul_list(format!("{}/compatible", dt::DT_ROOT));
        // La dernière entrée « compatible » est généralement le SoC.
        b.soc = b.dt_compatible.last().filter(|c| c.contains(',')).cloned();
        b.vendor = b.dt_compatible.first().and_then(|c| c.split(',').next()).map(String::from);
    }

    // PC / serveurs : DMI. On ignore les valeurs de remplissage des fabricants.
    if b.model.is_none() {
        let clean = |f: &str| read_trim(format!("{}/{}", DMI, f)).filter(|v| !is_placeholder(v));
        let product = clean("product_name");
        let board = clean("board_name");
        b.vendor = clean("sys_vendor").or_else(|| clean("board_vendor"));
        b.model = match (product, board) {
            (Some(p), Some(bd)) if p != bd => Some(format!("{} (carte mère {})", p, bd)),
            (Some(p), _) => Some(p),
            (None, Some(bd)) => Some(bd),
            _ => None,
        };
    }
    b.firmware = read_trim(format!("{}/bios_version", DMI)).map(|v| {
        match read_trim(format!("{}/bios_date", DMI)) {
            Some(d) => format!("{} ({})", v, d),
            None => v,
        }
    });

    b.soc_family = read_trim("/sys/devices/soc0/family");
    b.soc_id = read_trim("/sys/devices/soc0/soc_id").or_else(|| read_trim("/sys/devices/soc0/machine"));

    // NVIDIA Jetson : version L4T et JetPack.
    if let Some(rel) = read_trim("/etc/nv_tegra_release") {
        b.jetson = Some(Jetson {
            l4t_release: parse_l4t(&rel),
            jetpack: run_ok("dpkg-query", &["-W", "-f", "${Version}", "nvidia-jetpack"]).filter(|s| !s.trim().is_empty()),
        });
    }

    // Raspberry Pi : code de révision (identifie modèle, RAM, fabricant).
    if b.dt_compatible.iter().any(|c| c.starts_with("raspberrypi")) {
        if let Ok(ci) = fs::read_to_string("/proc/cpuinfo") {
            b.raspberry_pi_revision = ci.lines().find_map(|l| colon_value(l, "Revision").map(String::from));
        }
    }
    b
}

fn is_placeholder(v: &str) -> bool {
    let l = v.to_lowercase();
    ["to be filled", "default string", "system product name", "system manufacturer", "not applicable", "o.e.m", "none"]
        .iter()
        .any(|p| l.contains(p))
}

/// `# R35 (release), REVISION: 4.1, GCID: ...` -> `R35.4.1`
fn parse_l4t(s: &str) -> Option<String> {
    let major = s.split_whitespace().find(|w| w.starts_with('R') && w[1..].chars().all(|c| c.is_ascii_digit()))?;
    let rev = s.split("REVISION:").nth(1)?.split(',').next()?.trim();
    Some(format!("{}.{}", major, rev))
}

pub fn probe_os() -> Os {
    let mut os = Os::default();
    os.hostname = read_trim("/proc/sys/kernel/hostname").or_else(|| run_ok("hostname", &[]).map(|s| s.trim().to_string()));
    if let Ok(rel) = fs::read_to_string("/etc/os-release") {
        os.pretty_name = kv_value(&rel, "PRETTY_NAME");
        os.id = kv_value(&rel, "ID");
        os.version_id = kv_value(&rel, "VERSION_ID");
    }
    if os.pretty_name.is_none() {
        os.pretty_name = Some(std::env::consts::OS.to_string());
    }
    os.kernel = read_trim("/proc/sys/kernel/osrelease").or_else(|| run_ok("uname", &["-r"]).map(|s| s.trim().into()));
    os.arch = run_ok("uname", &["-m"]).map(|s| s.trim().to_string());
    os.userland_bits = run_ok("getconf", &["LONG_BIT"]).and_then(|s| s.trim().parse().ok());
    os.package_arch = run_ok("dpkg", &["--print-architecture"])
        .or_else(|| run_ok("apk", &["--print-arch"]))
        .or_else(|| run_ok("rpm", &["--eval", "%{_arch}"]))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    os.libc = detect_libc();
    os.init = read_trim("/proc/1/comm");
    os.virtualization = detect_virtualization();
    if let Some(up) = read_trim("/proc/uptime") {
        os.uptime_s = up.split_whitespace().next().and_then(|s| s.parse::<f64>().ok()).map(|s| s as u64);
    }
    if let Some(l) = read_trim("/proc/loadavg") {
        let v: Vec<f64> = l.split_whitespace().take(3).filter_map(|x| x.parse().ok()).collect();
        if v.len() == 3 {
            os.load_avg = Some([v[0], v[1], v[2]]);
        }
    }
    os.desktop = std::env::var("XDG_CURRENT_DESKTOP").ok().or_else(|| std::env::var("DESKTOP_SESSION").ok()).filter(|s| !s.is_empty());
    os.shell = std::env::var("SHELL").ok().map(|s| {
        let name = Path::new(&s).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(s.clone());
        match tool_version(&s, &["--version"]) {
            Some(v) => format!("{} {}", name, v),
            None => name,
        }
    });
    if let Some((mgr, count)) = count_packages() {
        os.package_manager = Some(mgr.to_string());
        os.installed_packages = Some(count);
    }
    os
}

fn detect_libc() -> Option<String> {
    let out = run("ldd", &["--version"])?;
    let text = format!("{}\n{}", out.stdout, out.stderr);
    if text.to_lowercase().contains("musl") {
        return Some(match extract_version(&text) {
            Some(v) => format!("musl {}", v),
            None => "musl".into(),
        });
    }
    let first = text.lines().next()?;
    extract_version(first).map(|v| format!("glibc {}", v))
}

fn detect_virtualization() -> Option<String> {
    if Path::new("/.dockerenv").exists() {
        return Some("conteneur Docker".into());
    }
    if Path::new("/run/.containerenv").exists() {
        return Some("conteneur Podman".into());
    }
    // systemd-detect-virt renvoie « none » (code 1) sur une machine physique.
    let out = run("systemd-detect-virt", &[])?;
    let v = out.stdout.trim();
    (!v.is_empty() && v != "none").then(|| v.to_string())
}

fn count_packages() -> Option<(&'static str, usize)> {
    let count = |s: String| s.lines().filter(|l| !l.trim().is_empty()).count();
    if let Some(o) = run_ok("dpkg-query", &["-f", ".\n", "-W"]) {
        return Some(("apt/dpkg", count(o)));
    }
    if let Some(o) = run_ok("rpm", &["-qa"]) {
        return Some(("rpm", count(o)));
    }
    if let Some(o) = run_ok("pacman", &["-Qq"]) {
        return Some(("pacman", count(o)));
    }
    if let Some(o) = run_ok("apk", &["info"]) {
        return Some(("apk", count(o)));
    }
    if let Some(o) = run_ok("opkg", &["list-installed"]) {
        return Some(("opkg", count(o)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l4t() {
        let s = "# R35 (release), REVISION: 4.1, GCID: 33958178, BOARD: t186ref, EABI: aarch64";
        assert_eq!(parse_l4t(s).as_deref(), Some("R35.4.1"));
    }

    #[test]
    fn placeholders() {
        assert!(is_placeholder("To Be Filled By O.E.M."));
        assert!(!is_placeholder("ROCK 5B"));
    }
}
