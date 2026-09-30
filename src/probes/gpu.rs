//! GPU : cartes PCI (NVIDIA/AMD/Intel) et GPU intégrés des SoC
//! (Mali, VideoCore, Adreno, Vivante, PowerVR, Tegra) via DRM, sysfs et devfreq.

use super::interfaces::pci_list;
use crate::report::{Gpu, Sensor};
use crate::util::*;
use std::path::Path;

/// Drivers DRM qui pilotent un NPU et non un GPU.
pub const NPU_DRM_DRIVERS: &[&str] = &["rknpu", "rocket", "ethosu", "amdxdna", "intel_vpu", "ivpu"];

pub fn probe(sensors: &[Sensor]) -> Vec<Gpu> {
    let mut gpus = nvidia_smi();

    // 1. GPU PCI (classe 0x03xxxx : contrôleurs d'affichage).
    for d in pci_list().iter().filter(|d| d.class.starts_with("0x03")) {
        if let Some(g) = gpus.iter_mut().find(|g| g.pci_id.as_deref() == Some(d.slot.as_str())) {
            g.driver = g.driver.take().or(d.driver.clone());
            continue;
        }
        let base = format!("/sys/bus/pci/devices/{}", d.slot);
        let vram = read_u64(format!("{}/mem_info_vram_total", base)).map(|b| b / 1024 / 1024);
        gpus.push(Gpu {
            name: d.description.clone().unwrap_or_else(|| format!("GPU PCI {}", d.id)),
            vendor: pci_vendor_name(&d.id).map(String::from),
            driver: d.driver.clone(),
            source: "pci".into(),
            vram_mb: vram,
            render_node: render_node_for(Path::new(&base)),
            pci_id: Some(d.slot.clone()),
            ..Default::default()
        });
    }

    // 2. GPU intégrés exposés par DRM (nœuds de rendu d'un périphérique plateforme).
    for r in list_dir_prefix("/sys/class/drm", "renderD") {
        let dev = Path::new("/sys/class/drm").join(&r).join("device");
        if link_name(dev.join("subsystem")).as_deref() == Some("pci") {
            continue;
        }
        let driver = link_name(dev.join("driver"));
        if driver.as_deref().is_some_and(|d| NPU_DRM_DRIVERS.contains(&d)) {
            continue;
        }
        let compat = read_nul_list(dev.join("of_node/compatible"));
        gpus.push(Gpu {
            name: soc_gpu_name(driver.as_deref(), &compat),
            vendor: soc_gpu_vendor(driver.as_deref()).map(String::from),
            driver: driver.clone(),
            source: "drm".into(),
            render_node: Some(format!("/dev/dri/{}", r)),
            ..Default::default()
        });
    }

    // 3. Mali avec driver propriétaire (kbase) : pas de DRM, mais /dev/mali0.
    if Path::new("/dev/mali0").exists() && !gpus.iter().any(|g| g.driver.as_deref().is_some_and(|d| d.contains("mali"))) {
        let dev = Path::new("/sys/class/misc/mali0/device");
        let info = read_trim(dev.join("gpuinfo"));
        let compat = read_nul_list(dev.join("of_node/compatible"));
        gpus.push(Gpu {
            name: info.clone().map(|i| format!("ARM {}", i)).unwrap_or_else(|| soc_gpu_name(Some("mali"), &compat)),
            vendor: Some("ARM".into()),
            driver: link_name(dev.join("driver")).or(Some("mali_kbase (propriétaire)".into())),
            driver_version: kbase_version(),
            source: "mali kbase".into(),
            ..Default::default()
        });
    }

    // 4. NVIDIA Jetson : GPU intégré piloté par nvgpu (pas de DRM classique).
    let compat = read_nul_list("/proc/device-tree/compatible");
    if let Some(tegra) = compat.iter().find(|c| c.starts_with("nvidia,tegra")) {
        if !gpus.iter().any(|g| g.vendor.as_deref() == Some("NVIDIA")) {
            gpus.push(Gpu {
                name: tegra_gpu_name(tegra),
                vendor: Some("NVIDIA".into()),
                driver: Some("nvgpu".into()),
                source: "tegra".into(),
                ..Default::default()
            });
        }
    }

    // Fréquence max des GPU intégrés via devfreq.
    let gpu_devfreq: Vec<u64> = list_dir("/sys/class/devfreq")
        .into_iter()
        .filter(|n| n.to_lowercase().contains("gpu"))
        .filter_map(|n| max_devfreq_mhz(&format!("/sys/class/devfreq/{}", n)))
        .collect();
    for (g, f) in gpus.iter_mut().filter(|g| g.source != "pci" && g.source != "nvidia-smi").zip(gpu_devfreq) {
        g.max_freq_mhz = Some(f);
    }

    // Température des GPU non NVIDIA (capteurs « gpu » / amdgpu).
    let gpu_temps: Vec<f64> = sensors
        .iter()
        .filter(|s| {
            let n = s.name.to_lowercase();
            n.contains("gpu") && !n.contains("nvidia")
        })
        .map(|s| s.temp_c)
        .collect();
    for (g, t) in gpus.iter_mut().filter(|g| g.temperature_c.is_none()).zip(gpu_temps) {
        g.temperature_c = Some(t);
    }
    gpus
}

fn nvidia_smi() -> Vec<Gpu> {
    let full = "--query-gpu=name,memory.total,driver_version,temperature.gpu,pci.bus_id,compute_cap";
    let base = "--query-gpu=name,memory.total,driver_version,temperature.gpu,pci.bus_id";
    let out = run_ok("nvidia-smi", &[full, "--format=csv,noheader,nounits"])
        .or_else(|| run_ok("nvidia-smi", &[base, "--format=csv,noheader,nounits"]));
    let Some(out) = out else { return Vec::new() };
    out.lines().filter_map(parse_nvidia_smi_line).collect()
}

fn parse_nvidia_smi_line(line: &str) -> Option<Gpu> {
    let f: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if f.len() < 5 || f[0].is_empty() {
        return None;
    }
    let valid = |s: &str| !s.is_empty() && !s.contains("N/A") && !s.contains("Not Supported");
    Some(Gpu {
        name: f[0].to_string(),
        vendor: Some("NVIDIA".into()),
        vram_mb: f[1].parse().ok(),
        driver_version: valid(f[2]).then(|| f[2].to_string()),
        temperature_c: f[3].parse().ok(),
        pci_id: normalize_pci_bus_id(f[4]),
        compute_capability: f.get(5).filter(|s| valid(s)).map(|s| s.to_string()),
        source: "nvidia-smi".into(),
        ..Default::default()
    })
}

/// `00000000:0A:00.0` -> `0000:0a:00.0` (format sysfs).
fn normalize_pci_bus_id(s: &str) -> Option<String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let domain = u32::from_str_radix(parts[0], 16).ok()?;
    Some(format!("{:04x}:{}:{}", domain, parts[1].to_lowercase(), parts[2].to_lowercase()))
}

fn render_node_for(pci_dev: &Path) -> Option<String> {
    list_dir_prefix(pci_dev.join("drm"), "renderD").into_iter().next().map(|r| format!("/dev/dri/{}", r))
}

pub fn pci_vendor_name(id: &str) -> Option<&'static str> {
    Some(match id.split(':').next()? {
        "10de" => "NVIDIA",
        "1002" => "AMD",
        "8086" => "Intel",
        "1a03" => "ASPEED",
        "102b" => "Matrox",
        "15ad" => "VMware",
        "1af4" => "Red Hat (virtio)",
        "1234" => "QEMU",
        "5143" => "Qualcomm",
        "1e60" => "Hailo",
        "1ac1" => "Global Unichip (Coral Edge TPU)",
        _ => return None,
    })
}

fn soc_gpu_vendor(driver: Option<&str>) -> Option<&'static str> {
    Some(match driver? {
        "panfrost" | "panthor" | "lima" | "mali" => "ARM",
        "v3d" | "vc4" => "Broadcom",
        "etnaviv" | "galcore" => "Vivante / VeriSilicon",
        "msm" | "msm_drm" => "Qualcomm",
        "powervr" | "pvrsrvkm" => "Imagination",
        "tegra" | "nvgpu" => "NVIDIA",
        _ => return None,
    })
}

fn soc_gpu_name(driver: Option<&str>, compat: &[String]) -> String {
    let has = |s: &str| compat.iter().any(|c| c.contains(s));
    let mali_family = if has("valhall") {
        Some("Valhall")
    } else if has("bifrost") {
        Some("Bifrost")
    } else if has("midgard") || has("mali-t") {
        Some("Midgard")
    } else if has("utgard") || has("mali-400") || has("mali-450") {
        Some("Utgard (Mali-400/450)")
    } else {
        None
    };
    let label = match driver {
        Some("panthor") => format!("ARM Mali {} (driver libre Panthor)", mali_family.unwrap_or("Valhall CSF")),
        Some("panfrost") => format!("ARM Mali {} (driver libre Panfrost)", mali_family.unwrap_or("")),
        Some("lima") => "ARM Mali-400/450 (driver libre Lima)".into(),
        Some("mali") => format!("ARM Mali {}", mali_family.unwrap_or("")),
        Some("v3d") if has("2712") => "Broadcom VideoCore VII (V3D)".into(),
        Some("v3d") if has("2711") => "Broadcom VideoCore VI (V3D)".into(),
        Some("v3d") => "Broadcom VideoCore (V3D)".into(),
        Some("vc4") => "Broadcom VideoCore IV".into(),
        Some("etnaviv") => "Vivante GC (driver libre etnaviv)".into(),
        Some("msm") | Some("msm_drm") => "Qualcomm Adreno (driver msm)".into(),
        Some("powervr") | Some("pvrsrvkm") => "Imagination PowerVR".into(),
        Some("virtio_gpu") | Some("virtio-pci") => "GPU virtuel virtio".into(),
        Some(d) => format!("GPU (driver {})", d),
        None => "GPU intégré (driver inconnu)".into(),
    };
    let label = label.replace("  ", " ").trim().to_string();
    match compat.first() {
        Some(c) => format!("{} [{}]", label, c),
        None => label,
    }
}

fn tegra_gpu_name(compat: &str) -> String {
    let arch = if compat.contains("tegra264") {
        "Blackwell (Thor)"
    } else if compat.contains("tegra234") {
        "Ampere (Orin)"
    } else if compat.contains("tegra194") {
        "Volta (Xavier)"
    } else if compat.contains("tegra186") {
        "Pascal (TX2)"
    } else if compat.contains("tegra210") {
        "Maxwell (Nano / TX1)"
    } else {
        "iGPU"
    };
    format!("NVIDIA Tegra {} — GPU intégré CUDA", arch)
}

fn kbase_version() -> Option<String> {
    list_dir("/sys/module")
        .into_iter()
        .filter(|m| m.contains("kbase") || m == "mali")
        .find_map(|m| read_trim(format!("/sys/module/{}/version", m)).map(|v| format!("{} {}", m, v)))
}

/// Fréquence max (MHz) d'un périphérique devfreq.
pub fn max_devfreq_mhz(base: &str) -> Option<u64> {
    let avail = read_trim(format!("{}/available_frequencies", base))
        .and_then(|s| s.split_whitespace().filter_map(|f| f.parse::<u64>().ok()).max());
    avail.or_else(|| read_u64(format!("{}/max_freq", base))).map(|hz| hz / 1_000_000).filter(|&m| m > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smi_line() {
        let g = parse_nvidia_smi_line("Quadro RTX 4000, 8192, 550.54.14, 45, 00000000:0A:00.0, 7.5").unwrap();
        assert_eq!(g.vram_mb, Some(8192));
        assert_eq!(g.pci_id.as_deref(), Some("0000:0a:00.0"));
        assert_eq!(g.compute_capability.as_deref(), Some("7.5"));
        let g = parse_nvidia_smi_line("Orin (nvgpu), [N/A], 540.4.0, N/A, 00000000:00:00.0").unwrap();
        assert_eq!(g.vram_mb, None);
        assert_eq!(g.temperature_c, None);
    }

    #[test]
    fn soc_names() {
        let c = vec!["rockchip,rk3588-mali".to_string(), "arm,mali-valhall-csf".to_string()];
        assert!(soc_gpu_name(Some("panthor"), &c).contains("Valhall"));
        let c = vec!["brcm,2712-v3d".to_string()];
        assert!(soc_gpu_name(Some("v3d"), &c).contains("VideoCore VII"));
    }
}
