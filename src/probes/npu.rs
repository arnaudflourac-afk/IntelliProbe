//! NPU / accélérateurs IA. Sources croisées : device-tree, /dev/accel (DRM accel),
//! drivers DRM, nœuds constructeur (/dev/rknpu, /dev/galcore…), PCI, USB, modules noyau.

use super::dt;
use super::interfaces::{pci_list, usb_devices};
use crate::report::Npu;
use crate::util::*;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub fn probe() -> Vec<Npu> {
    let modules = loaded_modules();
    let soc_compat = read_nul_list(format!("{}/compatible", dt::DT_ROOT));
    let mut npus: Vec<Npu> = Vec::new();

    // 1. Device-tree : le SoC déclare-t-il un NPU, est-il activé, un driver est-il lié ?
    if dt::present() {
        let bindings = dt::platform_bindings();
        let mut seen_compat = HashSet::new();
        for node in dt::walk(Path::new(dt::DT_ROOT), 3) {
            if !is_npu_node(&node.base, &node.compatible) {
                continue;
            }
            let key = node.compatible.first().cloned().unwrap_or(node.base.clone());
            // Les NPU multi-cœurs (RK3588 mainline) ont un nœud par cœur : on les regroupe.
            if !seen_compat.insert(key.clone()) {
                if let Some(n) = npus.iter_mut().find(|n| n.source.contains(&key)) {
                    n.name = bump_core_count(&n.name);
                }
                continue;
            }
            let binding = bindings.get(&node.path);
            let driver = binding.and_then(|(_, d)| d.clone());
            let status = if !node.enabled {
                "désactivé dans le device-tree (activable par overlay)".to_string()
            } else if driver.is_some() {
                "actif (driver lié)".to_string()
            } else {
                "déclaré mais aucun driver lié".to_string()
            };
            let (name, vendor, tops) = identify_soc_npu(&key, &soc_compat);
            let devfreq = binding.and_then(|(dev, _)| super::gpu::max_devfreq_mhz(&format!("/sys/class/devfreq/{}", dev)));
            npus.push(Npu {
                name: match devfreq {
                    Some(f) => format!("{} (jusqu'à {} MHz)", name, f),
                    None => name,
                },
                vendor,
                source: format!("device-tree {}", key),
                driver,
                status,
                datasheet_tops: tops,
                ..Default::default()
            });
        }
    }

    // 2. Périphériques DRM accel (/dev/accel/accelN) : Intel NPU, AMD XDNA, Rocket, Ethos-U…
    for a in list_dir_prefix("/sys/class/accel", "accel") {
        let dev = Path::new("/sys/class/accel").join(&a).join("device");
        let driver = link_name(dev.join("driver"));
        let (name, vendor) = accel_driver_name(driver.as_deref());
        attach_or_push(&mut npus, Npu {
            name: name.into(),
            vendor: vendor.map(String::from),
            source: "/dev/accel".into(),
            driver: driver.clone(),
            device_nodes: vec![format!("/dev/accel/{}", a)],
            status: "actif (nœud /dev/accel présent)".into(),
            ..Default::default()
        });
    }

    // 3. Driver NPU exposé comme nœud DRM (RKNPU vendor ≥ 0.9).
    for r in list_dir_prefix("/sys/class/drm", "renderD") {
        let driver = link_name(format!("/sys/class/drm/{}/device/driver", r));
        if matches!(driver.as_deref(), Some("rknpu") | Some("RKNPU")) {
            attach_or_push(&mut npus, Npu {
                name: "Rockchip NPU".into(),
                vendor: Some("Rockchip".into()),
                source: "drm".into(),
                driver: driver.clone(),
                device_nodes: vec![format!("/dev/dri/{}", r)],
                status: "actif (driver RKNPU chargé)".into(),
                ..Default::default()
            });
        }
    }

    // 4. Nœuds constructeur.
    const NODES: &[(&str, &str, &str)] = &[
        ("rknpu", "Rockchip NPU", "Rockchip"),
        ("galcore", "VeriSilicon/Vivante NPU (galcore)", "VeriSilicon"),
        ("vipcore", "VeriSilicon VIP NPU", "VeriSilicon"),
        ("hailo", "Hailo", "Hailo"),
        ("apex_", "Google Coral Edge TPU (PCIe)", "Google"),
        ("nvhost-nvdla", "NVIDIA DLA", "NVIDIA"),
        ("axl", "Axelera Metis", "Axelera"),
        ("metis", "Axelera Metis", "Axelera"),
    ];
    for d in list_dir("/dev") {
        let Some((_, name, vendor)) = NODES.iter().find(|(p, _, _)| d.starts_with(p)) else { continue };
        attach_or_push(&mut npus, Npu {
            name: name.to_string(),
            vendor: Some(vendor.to_string()),
            source: "/dev".into(),
            device_nodes: vec![format!("/dev/{}", d)],
            status: "nœud présent".into(),
            ..Default::default()
        });
    }

    // 5. Accélérateurs PCI (classe 0x12 : processing accelerator) et IDs connus.
    for d in pci_list() {
        let vendor = d.id.split(':').next().unwrap_or("");
        let known = matches!(vendor, "1e60" | "1ac1");
        if !d.class.starts_with("0x12") && !known {
            continue;
        }
        let mut n = Npu {
            name: d.description.clone().unwrap_or_else(|| format!("Accélérateur PCI {}", d.id)),
            vendor: super::gpu::pci_vendor_name(&d.id).map(String::from),
            source: format!("pci {}", d.slot),
            driver: d.driver.clone(),
            status: if d.driver.is_some() { "actif (driver lié)".into() } else { "présent, aucun driver lié".into() },
            ..Default::default()
        };
        if vendor == "1e60" {
            let (arch, tops) = hailo_identify();
            if let Some(a) = arch {
                n.name = format!("Hailo {}", a);
            }
            n.datasheet_tops = tops;
        } else if vendor == "1ac1" {
            n.name = "Google Coral Edge TPU (PCIe/M.2)".into();
            n.datasheet_tops = Some(4.0);
        }
        attach_or_push(&mut npus, n);
    }

    // 6. Accélérateurs USB.
    for u in usb_devices() {
        let found = match u.id.as_str() {
            "1a6e:089a" | "18d1:9302" => Some(("Google Coral Edge TPU (USB)", "Google", Some(4.0))),
            "03e7:2485" | "03e7:f63b" => Some(("Intel Movidius Myriad X (NCS2)", "Intel", None)),
            "03e7:2150" => Some(("Intel Movidius Myriad 2 (NCS)", "Intel", None)),
            _ => None,
        };
        if let Some((name, vendor, tops)) = found {
            npus.push(Npu {
                name: name.into(),
                vendor: Some(vendor.into()),
                source: format!("usb {}", u.id),
                status: "branché".into(),
                datasheet_tops: tops,
                ..Default::default()
            });
        }
    }

    // Driver chargé mais aucune autre trace : on le signale quand même.
    for (module, name, vendor) in [("rknpu", "Rockchip NPU", "Rockchip"), ("galcore", "VeriSilicon/Vivante NPU", "VeriSilicon"), ("hailo_pci", "Hailo", "Hailo"), ("apex", "Google Coral Edge TPU", "Google")] {
        if modules.contains(module) && !npus.iter().any(|n| n.vendor.as_deref() == Some(vendor)) {
            npus.push(Npu { name: name.into(), vendor: Some(vendor.into()), source: format!("module {}", module), driver: Some(module.into()), status: "module noyau chargé".into(), ..Default::default() });
        }
    }

    // Version du driver quand le noyau l'expose.
    for n in &mut npus {
        if n.driver_version.is_none() {
            n.driver_version = driver_version(n.vendor.as_deref(), n.driver.as_deref());
        }
        n.device_nodes.sort();
        n.device_nodes.dedup();
    }
    npus
}

/// Fusionne avec un NPU du même fabricant déjà trouvé (ex. nœud DT + /dev), sinon ajoute.
fn attach_or_push(npus: &mut Vec<Npu>, new: Npu) {
    if let Some(n) = npus.iter_mut().find(|n| n.vendor.is_some() && n.vendor == new.vendor) {
        n.device_nodes.extend(new.device_nodes);
        if n.driver.is_none() {
            n.driver = new.driver;
        }
        if !n.status.starts_with("actif") && new.status.starts_with("actif") {
            n.status = new.status;
        }
        if n.datasheet_tops.is_none() {
            n.datasheet_tops = new.datasheet_tops;
        }
        if !n.source.contains(&new.source) {
            n.source = format!("{} + {}", n.source, new.source);
        }
    } else {
        npus.push(new);
    }
}

fn is_npu_node(base: &str, compat: &[String]) -> bool {
    let base_match = matches!(base, "npu" | "rknpu" | "galcore" | "vipnano" | "ethosu" | "ethos-u" | "nna") || base.starts_with("nvdla");
    let compat_match = compat.iter().any(|c| {
        let c = c.to_lowercase();
        c.contains("rknpu") || c.contains("rknn") || c.contains("-npu") || c.contains(",npu") || c.contains("galcore") || c.contains("ethos") || c.contains("nvdla") || c.contains("vipnano")
    });
    base_match || compat_match
}

fn bump_core_count(name: &str) -> String {
    // « X » -> « X ×2 cœurs » -> « X ×3 cœurs »
    if let Some((head, tail)) = name.rsplit_once(" ×") {
        let n: u32 = tail.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(1);
        format!("{} ×{} cœurs", head, n + 1)
    } else {
        format!("{} ×2 cœurs", name)
    }
}

/// Identification du NPU d'un SoC. Les TOPS ne sont donnés que pour les puces
/// identifiées sans ambiguïté (valeurs des fiches constructeur).
fn identify_soc_npu(node_compat: &str, soc_compat: &[String]) -> (String, Option<String>, Option<f64>) {
    let soc = |s: &str| soc_compat.iter().any(|c| c.ends_with(s));
    if node_compat.contains("rockchip") || node_compat.contains("rknpu") || node_compat.contains("rknn") {
        let (chip, tops) = if soc(",rk3588") || soc(",rk3588s") {
            ("RK3588", Some(6.0))
        } else if soc(",rk3576") {
            ("RK3576", Some(6.0))
        } else if soc(",rk3399pro") {
            ("RK3399Pro", Some(3.0))
        } else if soc(",rv1126") {
            ("RV1126", Some(2.0))
        } else {
            (soc_compat.last().map(|c| c.split(',').nth(1).unwrap_or(c)).unwrap_or("?"), None)
        };
        return (format!("Rockchip {} NPU", chip.to_uppercase()), Some("Rockchip".into()), tops);
    }
    if node_compat.contains("nvdla") {
        return ("NVIDIA DLA (Deep Learning Accelerator)".into(), Some("NVIDIA".into()), None);
    }
    if node_compat.contains("ethos") {
        return ("Arm Ethos-U".into(), Some("ARM".into()), None);
    }
    if node_compat.contains("galcore") || node_compat.contains("vip") || node_compat.contains("amlogic") || node_compat.contains("fsl") || node_compat.contains("nxp") {
        let tops = if soc(",a311d") {
            Some(5.0)
        } else if soc(",imx8mp") {
            Some(2.3)
        } else {
            None
        };
        return (format!("VeriSilicon/Vivante NPU [{}]", node_compat), Some("VeriSilicon".into()), tops);
    }
    (format!("NPU [{}]", node_compat), node_compat.split(',').next().map(String::from), None)
}

fn accel_driver_name(driver: Option<&str>) -> (&'static str, Option<&'static str>) {
    match driver {
        Some("intel_vpu") => ("Intel NPU (Core Ultra)", Some("Intel")),
        Some("amdxdna") => ("AMD XDNA NPU (Ryzen AI)", Some("AMD")),
        Some("rocket") => ("Rockchip NPU (driver libre Rocket)", Some("Rockchip")),
        Some("ethosu") => ("Arm Ethos-U", Some("ARM")),
        Some("qaic") => ("Qualcomm Cloud AI 100", Some("Qualcomm")),
        Some("habanalabs") => ("Intel Gaudi", Some("Intel")),
        _ => ("Accélérateur DRM accel", None),
    }
}

/// `hailortcli fw-control identify` donne l'architecture exacte (HAILO8, HAILO8L…).
fn hailo_identify() -> (Option<String>, Option<f64>) {
    let Some(out) = run_ok("hailortcli", &["fw-control", "identify"]) else { return (None, None) };
    let arch = out.lines().find_map(|l| colon_value(l, "Device Architecture").map(String::from));
    let tops = match arch.as_deref() {
        Some("HAILO8") => Some(26.0),
        Some("HAILO8L") => Some(13.0),
        Some("HAILO10H") => Some(40.0),
        _ => None,
    };
    (arch, tops)
}

fn loaded_modules() -> HashSet<String> {
    fs::read_to_string("/proc/modules")
        .map(|t| t.lines().filter_map(|l| l.split_whitespace().next().map(String::from)).collect())
        .unwrap_or_default()
}

fn driver_version(vendor: Option<&str>, driver: Option<&str>) -> Option<String> {
    if vendor == Some("Rockchip") {
        // debugfs nécessite root ; /sys/module est lisible par tous.
        for p in ["/sys/kernel/debug/rknpu/version", "/proc/debug/rknpu/version", "/sys/module/rknpu/version"] {
            if let Some(v) = read_trim(p) {
                return Some(v);
            }
        }
    }
    if vendor == Some("Hailo") {
        if let Some(v) = read_trim("/sys/module/hailo_pci/version") {
            return Some(v);
        }
    }
    driver.and_then(|d| read_trim(format!("/sys/module/{}/version", d.replace('-', "_"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npu_nodes() {
        assert!(is_npu_node("npu", &[]));
        assert!(is_npu_node("xyz", &["rockchip,rk3588-rknpu".into()]));
        assert!(!is_npu_node("gpu", &["arm,mali-valhall-csf".into()]));
        assert!(!is_npu_node("i2c", &["rockchip,rk3399-i2c".into()]));
    }

    #[test]
    fn rk3588() {
        let soc = vec!["radxa,rock-5b".to_string(), "rockchip,rk3588".to_string()];
        let (name, vendor, tops) = identify_soc_npu("rockchip,rk3588-rknpu", &soc);
        assert_eq!(name, "Rockchip RK3588 NPU");
        assert_eq!(vendor.as_deref(), Some("Rockchip"));
        assert_eq!(tops, Some(6.0));
        // Puce non identifiée précisément : pas de TOPS inventés.
        let soc = vec!["rockchip,rk3568".to_string()];
        assert_eq!(identify_soc_npu("rockchip,rk3568-rknpu", &soc).2, None);
    }

    #[test]
    fn cores() {
        let n = bump_core_count("Rockchip RK3588 NPU");
        assert_eq!(n, "Rockchip RK3588 NPU ×2 cœurs");
        assert_eq!(bump_core_count(&n), "Rockchip RK3588 NPU ×3 cœurs");
    }
}
