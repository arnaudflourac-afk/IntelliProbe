//! Interfaces matérielles : GPIO, I2C, SPI, UART, CAN, PWM, USB, PCI, réseau,
//! Bluetooth, watchdog, et périphériques déclarés dans le device-tree.

use super::dt;
use crate::report::{DevAccess, DtPeripheral, GpioChip, Interfaces, NamedDev, NetIf, PciDevice, UsbDevice};
use crate::util::*;
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

pub fn probe() -> Interfaces {
    let mut i = Interfaces::default();

    i.gpio_chips = gpio_chips();
    i.i2c_buses = list_dir_prefix("/dev", "i2c-")
        .into_iter()
        .map(|d| NamedDev {
            name: read_trim(format!("/sys/class/i2c-dev/{}/name", d)).or_else(|| read_trim(format!("/sys/bus/i2c/devices/{}/name", d))),
            dev: format!("/dev/{}", d),
        })
        .collect();
    i.spi_devices = list_dir_prefix("/dev", "spidev").into_iter().map(|d| format!("/dev/{}", d)).collect();
    i.uarts = uarts();
    i.pwm_chips = list_dir_prefix("/sys/class/pwm", "pwmchip")
        .into_iter()
        .map(|p| NamedDev {
            name: read_trim(format!("/sys/class/pwm/{}/npwm", p)).map(|n| format!("{} canaux", n)),
            dev: p,
        })
        .collect();
    i.usb_devices = usb_devices().to_vec();
    i.pci_devices = pci_list().to_vec();
    let (net, can) = network();
    i.network = net;
    i.can = can;
    i.bluetooth = list_dir_prefix("/sys/class/bluetooth", "hci");
    i.watchdogs = list_dir_prefix("/dev", "watchdog").into_iter().map(|d| format!("/dev/{}", d)).collect();
    if dt::present() {
        i.dt_peripherals = dt_peripherals();
    }
    i.access = node_access(&i);
    i
}

// ---------------------------------------------------------------- Droits d'accès

/// Pour un nœud représentatif de chaque type : groupe propriétaire et droit
/// réel de l'utilisateur courant (c'est ce qui bloque le plus souvent au début).
fn node_access(i: &Interfaces) -> Vec<DevAccess> {
    let first = |dir: &str, prefix: &str| list_dir_prefix(dir, prefix).into_iter().next().map(|n| format!("{}/{}", dir, n));
    let candidates: Vec<(&str, Option<String>)> = vec![
        ("GPIO", first("/dev", "gpiochip")),
        ("I2C", first("/dev", "i2c-")),
        ("SPI", first("/dev", "spidev")),
        ("UART", i.uarts.iter().find(|u| !u.dev.contains("ttyS")).or(i.uarts.first()).map(|u| u.dev.clone())),
        ("Vidéo", first("/dev", "video")),
        ("GPU (rendu)", first("/dev/dri", "renderD")),
        ("NPU", first("/dev/accel", "accel").or_else(|| first("/dev", "rknpu")).or_else(|| first("/dev", "galcore")).or_else(|| first("/dev", "hailo")).or_else(|| first("/dev", "apex_"))),
        ("GPU Mali", first("/dev", "mali")),
    ];
    let groups = group_names();
    candidates
        .into_iter()
        .filter_map(|(kind, node)| {
            let node = node?;
            Some(DevAccess { kind: kind.into(), group: node_group(&node, &groups), writable: can_rw(&node), node })
        })
        .collect()
}

fn group_names() -> HashMap<u32, String> {
    fs::read_to_string("/etc/group")
        .map(|t| {
            t.lines()
                .filter_map(|l| {
                    let f: Vec<&str> = l.split(':').collect();
                    Some((f.get(2)?.parse().ok()?, f[0].to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(unix)]
fn node_group(path: &str, groups: &HashMap<u32, String>) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    let gid = fs::metadata(path).ok()?.gid();
    Some(groups.get(&gid).cloned().unwrap_or_else(|| gid.to_string()))
}

#[cfg(not(unix))]
fn node_group(_path: &str, _groups: &HashMap<u32, String>) -> Option<String> {
    None
}

#[cfg(unix)]
fn can_rw(path: &str) -> bool {
    let Ok(c) = std::ffi::CString::new(path) else { return false };
    unsafe { libc::access(c.as_ptr(), libc::R_OK | libc::W_OK) == 0 }
}

#[cfg(not(unix))]
fn can_rw(_path: &str) -> bool {
    false
}

// ---------------------------------------------------------------- GPIO

fn gpio_chips() -> Vec<GpioChip> {
    // Libellés lisibles sans droits via /sys/class/gpio (quand l'interface legacy existe).
    let mut sysfs: HashMap<String, (Option<String>, Option<u32>)> = HashMap::new();
    for g in list_dir_prefix("/sys/class/gpio", "gpiochip") {
        let base = format!("/sys/class/gpio/{}", g);
        // …/gpiochipN/gpio/gpiochipBASE : le grand-parent porte le nom du /dev.
        if let Ok(real) = fs::canonicalize(&base) {
            if let Some(dev) = real.parent().and_then(|p| p.parent()).and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()) {
                sysfs.insert(dev, (read_trim(format!("{}/label", base)), read_trim(format!("{}/ngpio", base)).and_then(|n| n.parse().ok())));
            }
        }
    }
    list_dir_prefix("/dev", "gpiochip")
        .into_iter()
        .map(|d| {
            let (label, lines) = gpio_chipinfo(&format!("/dev/{}", d)).or_else(|| sysfs.get(&d).cloned()).unwrap_or((None, None));
            GpioChip { dev: format!("/dev/{}", d), label, lines }
        })
        .collect()
}

/// ioctl GPIO_GET_CHIPINFO_IOCTL (API caractère GPIO v1/v2).
#[cfg(target_os = "linux")]
fn gpio_chipinfo(path: &str) -> Option<(Option<String>, Option<u32>)> {
    #[repr(C)]
    struct GpioChipInfo {
        name: [u8; 32],
        label: [u8; 32],
        lines: u32,
    }
    const GPIO_GET_CHIPINFO_IOCTL: u32 = 0x8044_B401;
    let c = std::ffi::CString::new(path).ok()?;
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if fd < 0 {
        return None;
    }
    let mut info: GpioChipInfo = unsafe { std::mem::zeroed() };
    let r = unsafe { libc::ioctl(fd, GPIO_GET_CHIPINFO_IOCTL as _, &mut info) };
    unsafe { libc::close(fd) };
    if r < 0 {
        return None;
    }
    let label = cstr(&info.label);
    Some(((!label.is_empty()).then_some(label), Some(info.lines)))
}

#[cfg(not(target_os = "linux"))]
fn gpio_chipinfo(_path: &str) -> Option<(Option<String>, Option<u32>)> {
    None
}

pub fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}

// ---------------------------------------------------------------- UART

fn uarts() -> Vec<NamedDev> {
    const PREFIXES: &[&str] = &["ttyS", "ttyAMA", "ttyFIQ", "ttyUSB", "ttyACM", "ttymxc", "ttyTHS", "ttyMSM", "ttySC", "ttyLP", "ttyGS", "ttyO"];
    list_dir("/sys/class/tty")
        .into_iter()
        .filter(|t| PREFIXES.iter().any(|p| t.starts_with(p)))
        .filter(|t| Path::new(&format!("/sys/class/tty/{}/device", t)).exists())
        .filter(|t| {
            // Les ports 8250 fantômes (ttyS0..31 sans matériel) ont type = 0.
            !t.starts_with("ttyS") || read_trim(format!("/sys/class/tty/{}/type", t)).as_deref() != Some("0")
        })
        .map(|t| NamedDev { name: link_name(format!("/sys/class/tty/{}/device/driver", t)), dev: format!("/dev/{}", t) })
        .collect()
}

// ---------------------------------------------------------------- USB

pub fn usb_devices() -> &'static [UsbDevice] {
    static CACHE: OnceLock<Vec<UsbDevice>> = OnceLock::new();
    CACHE.get_or_init(|| {
        list_dir("/sys/bus/usb/devices")
            .into_iter()
            .filter(|d| !d.contains(':'))
            .filter_map(|d| {
                let base = format!("/sys/bus/usb/devices/{}", d);
                let vid = read_trim(format!("{}/idVendor", base))?;
                let pid = read_trim(format!("{}/idProduct", base))?;
                Some(UsbDevice {
                    id: format!("{}:{}", vid, pid),
                    manufacturer: read_trim(format!("{}/manufacturer", base)),
                    product: read_trim(format!("{}/product", base)),
                    speed_mbps: read_trim(format!("{}/speed", base)).and_then(|s| s.parse::<f64>().ok()).map(|s| s as u64),
                    is_hub: read_trim(format!("{}/bDeviceClass", base)).as_deref() == Some("09"),
                })
            })
            .collect()
    })
}

// ---------------------------------------------------------------- PCI

pub fn pci_list() -> &'static [PciDevice] {
    static CACHE: OnceLock<Vec<PciDevice>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let names = lspci_names();
        list_dir("/sys/bus/pci/devices")
            .into_iter()
            .map(|slot| {
                let base = format!("/sys/bus/pci/devices/{}", slot);
                let hex = |f: &str| read_trim(format!("{}/{}", base, f)).map(|v| v.trim_start_matches("0x").to_string()).unwrap_or_default();
                let link = match (read_trim(format!("{}/current_link_speed", base)), read_trim(format!("{}/current_link_width", base))) {
                    (Some(s), Some(w)) if w != "0" && !s.contains("Unknown") => Some(format!("{} x{}", s, w)),
                    _ => None,
                };
                PciDevice {
                    id: format!("{}:{}", hex("vendor"), hex("device")),
                    class: read_trim(format!("{}/class", base)).unwrap_or_default(),
                    description: names.get(&slot).cloned(),
                    driver: link_name(format!("{}/driver", base)),
                    link,
                    slot,
                }
            })
            .collect()
    })
}

/// Noms lisibles via `lspci -mm -D` (base pci.ids) : slot -> « fabricant modèle ».
fn lspci_names() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(out) = run_ok("lspci", &["-mm", "-D"]) else { return map };
    let re = Regex::new(r#""([^"]*)""#).unwrap();
    for line in out.lines() {
        let Some(slot) = line.split_whitespace().next() else { continue };
        let q: Vec<String> = re.captures_iter(line).map(|c| c[1].to_string()).collect();
        if q.len() >= 3 {
            map.insert(slot.to_string(), format!("{} {}", q[1], q[2]));
        }
    }
    map
}

// ---------------------------------------------------------------- Réseau

fn network() -> (Vec<NetIf>, Vec<String>) {
    let mut net = Vec::new();
    let mut can = Vec::new();
    for n in list_dir("/sys/class/net") {
        let base = format!("/sys/class/net/{}", n);
        let ty = read_trim(format!("{}/type", base)).unwrap_or_default();
        if ty == "280" {
            can.push(n);
            continue;
        }
        // Les interfaces virtuelles (docker0, veth, bridges, lo) n'ont pas de « device ».
        if !Path::new(&format!("{}/device", base)).exists() {
            continue;
        }
        let kind = if Path::new(&format!("{}/wireless", base)).exists() || Path::new(&format!("{}/phy80211", base)).exists() {
            "Wi-Fi"
        } else if ty == "1" {
            "Ethernet"
        } else if n.starts_with("wwan") || ty == "519" || ty == "65534" {
            "Modem / tunnel"
        } else {
            "Autre"
        };
        net.push(NetIf {
            kind: kind.into(),
            state: read_trim(format!("{}/operstate", base)),
            speed_mbps: read_trim(format!("{}/speed", base)).and_then(|s| s.parse().ok()).filter(|&s: &i64| s > 0),
            driver: link_name(format!("{}/device/driver", base)),
            name: n,
        });
    }
    (net, can)
}

// ---------------------------------------------------------------- Device-tree

/// Compte les contrôleurs déclarés dans le device-tree, activés ou non.
/// Un contrôleur désactivé est souvent activable par un overlay.
fn dt_peripherals() -> Vec<DtPeripheral> {
    let mut counts: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    for node in dt::walk(Path::new(dt::DT_ROOT), 4) {
        let Some(kind) = peripheral_kind(&node.base, &node.compatible) else { continue };
        let e = counts.entry(kind).or_default();
        if node.enabled { e.0 += 1 } else { e.1 += 1 }
    }
    counts.into_iter().map(|(k, (en, dis))| DtPeripheral { kind: k.into(), enabled: en, disabled: dis }).collect()
}

fn peripheral_kind(base: &str, compat: &[String]) -> Option<&'static str> {
    let c = |s: &str| compat.iter().any(|x| x.contains(s));
    Some(match base {
        "i2c" => "I2C",
        "spi" | "spi2apb" => "SPI",
        "serial" | "uart" => "UART",
        "pwm" => "PWM",
        "can" => "CAN",
        "ethernet" | "gmac" => "Ethernet",
        "pcie" => "PCIe",
        "usb" | "usbdrd3" | "usbdrd" | "usb2-phy" => "USB",
        "mmc" | "sdhci" | "dwmmc" => "MMC / SD / eMMC",
        "sata" => "SATA",
        "saradc" | "adc" => "ADC",
        "i2s" | "sai" => "I2S (audio)",
        "spdif" => "S/PDIF",
        "hdmi" => "HDMI",
        "dsi" => "DSI (écran)",
        "csi" | "csi2" | "mipi-csi" | "mipi_csi" | "csi-dphy" | "mipi-dcphy" | "csi2-dphy" | "rkcif" | "cif" => "Caméra MIPI-CSI",
        "isp" | "rkisp" => "ISP (traitement d'image)",
        "gpu" => "GPU",
        "npu" => "NPU",
        "rga" => "RGA (2D)",
        "video-codec" | "vpu" | "vepu" | "vdpu" | "rkvdec" | "rkvenc" | "jpeg-decoder" | "jpeg-encoder" | "codec" => "Codec vidéo",
        "watchdog" => "Watchdog",
        "rtc" => "RTC",
        "spi-nor" | "flash" => "Flash SPI",
        _ if c("snps,dw-apb-uart") || c("arm,pl011") => "UART",
        _ => return None,
    })
}
