//! Lecture du device-tree (cartes ARM/RISC-V) : nœuds, `compatible`, `status`,
//! et correspondance avec les périphériques du noyau.

use crate::util::{link_name, list_dir, read_nul_list, read_trim};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const DT_ROOT: &str = "/proc/device-tree";

#[derive(Debug, Clone)]
pub struct DtNode {
    /// Chemin relatif à la racine du device-tree (ex. `npu@fdab0000`).
    pub path: String,
    /// Nom du nœud sans l'adresse (ex. `npu`).
    pub base: String,
    pub compatible: Vec<String>,
    /// `false` si `status = "disabled"` (ou autre que okay).
    pub enabled: bool,
}

pub fn present() -> bool {
    Path::new(DT_ROOT).exists()
}

/// Parcourt les nœuds adressés (`nom@adresse`) jusqu'à `max_depth`,
/// en ignorant les sous-arbres de configuration (pinctrl, overlays, symboles).
pub fn walk(root: &Path, max_depth: usize) -> Vec<DtNode> {
    let mut out = Vec::new();
    walk_rec(root, root, 0, max_depth, &mut out);
    out
}

fn walk_rec(root: &Path, dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<DtNode>) {
    if depth >= max_depth {
        return;
    }
    for name in list_dir(dir) {
        let path = dir.join(&name);
        if !path.is_dir() {
            continue;
        }
        let base = name.split('@').next().unwrap_or(&name).to_string();
        if matches!(base.as_str(), "__symbols__" | "__fixups__" | "__local_fixups__" | "aliases" | "chosen" | "__overrides__")
            || base.starts_with("fragment")
            || base.contains("pinctrl")
            || base.starts_with("pinmux")
        {
            continue;
        }
        if name.contains('@') {
            let status = read_trim(path.join("status")).unwrap_or_else(|| "okay".into());
            out.push(DtNode {
                path: path.strip_prefix(root).unwrap_or(&path).to_string_lossy().into_owned(),
                base: base.clone(),
                compatible: read_nul_list(path.join("compatible")),
                enabled: status == "okay" || status == "ok",
            });
        }
        walk_rec(root, &path, depth + 1, max_depth, out);
    }
}

/// Associe chaque nœud du device-tree (chemin relatif) au périphérique
/// plateforme correspondant et à son driver lié, s'il y en a un.
pub fn platform_bindings() -> HashMap<String, (String, Option<String>)> {
    let mut map = HashMap::new();
    let dt_base = fs::canonicalize(DT_ROOT).unwrap_or_else(|_| PathBuf::from("/sys/firmware/devicetree/base"));
    for bus in ["/sys/bus/platform/devices", "/sys/bus/amba/devices"] {
        for dev in list_dir(bus) {
            let devpath = Path::new(bus).join(&dev);
            let Ok(node) = fs::canonicalize(devpath.join("of_node")) else { continue };
            let Ok(rel) = node.strip_prefix(&dt_base) else { continue };
            let driver = link_name(devpath.join("driver"));
            map.insert(rel.to_string_lossy().into_owned(), (dev, driver));
        }
    }
    map
}

/// Nom lisible d'un SoC à partir de son `compatible` (ex. `rockchip,rk3588` -> `Rockchip RK3588`).
pub fn soc_display_name(compat: &str) -> String {
    let (vendor, chip) = compat.split_once(',').unwrap_or(("", compat));
    let vendor = match vendor {
        "rockchip" => "Rockchip",
        "brcm" => "Broadcom",
        "nvidia" => "NVIDIA",
        "amlogic" => "Amlogic",
        "allwinner" => "Allwinner",
        "fsl" | "nxp" => "NXP",
        "qcom" => "Qualcomm",
        "mediatek" => "MediaTek",
        "ti" => "Texas Instruments",
        "starfive" => "StarFive",
        "sophgo" => "Sophgo",
        "thead" => "T-Head",
        "spacemit" => "SpacemiT",
        "samsung" => "Samsung",
        "st" => "STMicroelectronics",
        "renesas" => "Renesas",
        "xlnx" => "AMD Xilinx",
        "cix" => "CIX",
        other => other,
    };
    let chip = chip.to_uppercase();
    if vendor.is_empty() { chip } else { format!("{} {}", vendor, chip) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_fake_tree() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        fs::create_dir_all(r.join("npu@fdab0000")).unwrap();
        fs::write(r.join("npu@fdab0000/compatible"), b"rockchip,rk3588-rknpu\0").unwrap();
        fs::create_dir_all(r.join("i2c@feaa0000")).unwrap();
        fs::write(r.join("i2c@feaa0000/status"), b"disabled\0").unwrap();
        fs::create_dir_all(r.join("pinctrl/i2c0@0")).unwrap();
        fs::create_dir_all(r.join("soc/serial@7e201000")).unwrap();

        let nodes = walk(r, 3);
        let names: Vec<_> = nodes.iter().map(|n| n.path.as_str()).collect();
        assert!(names.contains(&"npu@fdab0000"));
        assert!(names.contains(&"soc/serial@7e201000"));
        assert!(!names.iter().any(|n| n.contains("pinctrl")));
        let npu = nodes.iter().find(|n| n.base == "npu").unwrap();
        assert_eq!(npu.compatible, vec!["rockchip,rk3588-rknpu"]);
        assert!(npu.enabled);
        assert!(!nodes.iter().find(|n| n.base == "i2c").unwrap().enabled);
    }

    #[test]
    fn soc_names() {
        assert_eq!(soc_display_name("rockchip,rk3588"), "Rockchip RK3588");
        assert_eq!(soc_display_name("brcm,bcm2712"), "Broadcom BCM2712");
    }
}
