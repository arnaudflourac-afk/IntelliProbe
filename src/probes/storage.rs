//! Mémoire, swap/zram, périphériques de stockage et systèmes de fichiers.

use crate::report::{BlockDevice, Filesystem, Memory, Storage, Zram};
use crate::util::*;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub fn probe_memory() -> Memory {
    let text = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let kb = |key: &str| -> u64 {
        text.lines()
            .find_map(|l| colon_value(l, key))
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    };
    let mut m = Memory {
        total_mb: kb("MemTotal") / 1024,
        available_mb: kb("MemAvailable") / 1024,
        swap_total_mb: kb("SwapTotal") / 1024,
        swap_free_mb: kb("SwapFree") / 1024,
        zram: Vec::new(),
    };
    for z in list_dir_prefix("/sys/block", "zram") {
        let base = format!("/sys/block/{}", z);
        let size = read_u64(format!("{}/disksize", base)).unwrap_or(0);
        if size == 0 {
            continue;
        }
        // Format : « lzo lzo-rle [zstd] » -> l'algorithme actif est entre crochets.
        let algo = read_trim(format!("{}/comp_algorithm", base))
            .and_then(|a| a.split_whitespace().find(|w| w.starts_with('[')).map(|w| w.trim_matches(|c| c == '[' || c == ']').to_string()));
        m.zram.push(Zram { name: z, disksize_mb: size / 1024 / 1024, algorithm: algo });
    }
    m
}

pub fn probe_storage() -> Storage {
    let mut s = Storage::default();
    for name in list_dir("/sys/block") {
        if ["loop", "ram", "zram", "dm-", "md", "sr", "nbd", "fd"].iter().any(|p| name.starts_with(p))
            || name.contains("boot")
            || name.ends_with("rpmb")
        {
            continue;
        }
        let base = format!("/sys/block/{}", name);
        let sectors = read_u64(format!("{}/size", base)).unwrap_or(0);
        if sectors == 0 {
            continue;
        }
        let rotational = read_u64(format!("{}/queue/rotational", base)) == Some(1);
        s.devices.push(BlockDevice {
            kind: device_kind(&name, rotational),
            size_gb: round1(bytes_to_gb(sectors * 512)),
            model: read_trim(format!("{}/device/model", base)).or_else(|| read_trim(format!("{}/device/name", base))),
            removable: read_u64(format!("{}/removable", base)) == Some(1),
            rotational,
            name,
        });
    }

    s.filesystems = mounted_filesystems();
    if let Some(root) = s.filesystems.iter().find(|f| f.mount == "/") {
        let disk = disk_of(&root.device);
        s.root_device_kind = disk
            .as_deref()
            .and_then(|d| s.devices.iter().find(|b| b.name == d).map(|b| format!("{} ({})", b.kind, d)))
            .or_else(|| (root.fstype == "overlay" || root.fstype == "tmpfs").then(|| format!("{} (système en mémoire / overlay)", root.fstype)));
    }
    s
}

fn device_kind(name: &str, rotational: bool) -> String {
    let base = format!("/sys/block/{}", name);
    if name.starts_with("nvme") {
        return "NVMe".into();
    }
    if name.starts_with("mmcblk") {
        return match read_trim(format!("{}/device/type", base)).as_deref() {
            Some("MMC") => "eMMC".into(),
            Some("SD") => "Carte SD".into(),
            Some(other) => format!("MMC ({})", other),
            None => "MMC/SD".into(),
        };
    }
    if name.starts_with("mtdblock") || name.starts_with("ubi") {
        return "Flash MTD (NAND/NOR)".into();
    }
    if name.starts_with("vd") || name.starts_with("xvd") {
        return "Disque virtuel".into();
    }
    let real = fs::canonicalize(&base).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    if real.contains("/usb") {
        return if rotational { "Disque USB (HDD)".into() } else { "Disque USB (flash/SSD)".into() };
    }
    if rotational { "HDD".into() } else { "SSD".into() }
}

/// Disque physique qui porte un périphérique de montage (/dev/nvme0n1p2 -> nvme0n1),
/// y compris à travers LVM/LUKS (dm-*) et /dev/root.
fn disk_of(device: &str) -> Option<String> {
    let mut name = if device == "/dev/root" {
        root_dev_from_stat()?
    } else {
        let real = fs::canonicalize(device).ok()?;
        real.file_name()?.to_string_lossy().into_owned()
    };
    for _ in 0..4 {
        let class = Path::new("/sys/class/block").join(&name);
        if !class.exists() {
            return None;
        }
        // LVM / LUKS : on suit le premier périphérique sous-jacent.
        if let Some(slave) = list_dir(class.join("slaves")).into_iter().next() {
            name = slave;
            continue;
        }
        if class.join("partition").exists() {
            let real = fs::canonicalize(&class).ok()?;
            return real.parent()?.file_name().map(|n| n.to_string_lossy().into_owned());
        }
        return Some(name);
    }
    None
}

#[cfg(target_os = "linux")]
fn root_dev_from_stat() -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    let dev = fs::metadata("/").ok()?.dev();
    let (maj, min) = (libc::major(dev), libc::minor(dev));
    let real = fs::canonicalize(format!("/sys/dev/block/{}:{}", maj, min)).ok()?;
    real.file_name().map(|n| n.to_string_lossy().into_owned())
}

#[cfg(not(target_os = "linux"))]
fn root_dev_from_stat() -> Option<String> {
    None
}

const REAL_FS: &[&str] = &[
    "ext2", "ext3", "ext4", "xfs", "btrfs", "f2fs", "vfat", "exfat", "ntfs", "ntfs3", "fuseblk", "zfs", "jfs", "bcachefs", "nfs", "nfs4", "cifs", "smb3", "overlay", "squashfs", "erofs", "ubifs", "jffs2",
];

fn mounted_filesystems() -> Vec<Filesystem> {
    let text = fs::read_to_string("/proc/self/mounts").unwrap_or_default();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 3 {
            continue;
        }
        let (device, mount, fstype) = (f[0], unescape_mount(f[1]), f[2]);
        if !REAL_FS.contains(&fstype) {
            continue;
        }
        // Montages techniques (snaps, conteneurs) : bruit sans intérêt.
        if (fstype == "squashfs" || fstype == "overlay") && mount != "/" {
            continue;
        }
        if mount.starts_with("/snap") || mount.starts_with("/var/lib/docker") || mount.starts_with("/var/lib/containers") || mount.starts_with("/run/") {
            continue;
        }
        if !seen.insert(device.to_string()) && mount != "/" {
            continue;
        }
        let Some((total, free)) = statvfs(&mount) else { continue };
        out.push(Filesystem { mount, device: device.to_string(), fstype: fstype.to_string(), total_gb: round1(bytes_to_gb(total)), free_gb: round1(bytes_to_gb(free)) });
    }
    out
}

/// /proc/mounts encode les espaces en `\040`.
fn unescape_mount(s: &str) -> String {
    s.replace("\\040", " ").replace("\\011", "\t")
}

#[cfg(unix)]
pub fn statvfs(path: &str) -> Option<(u64, u64)> {
    let c = std::ffi::CString::new(path).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let frsize = st.f_frsize as u64;
    Some((st.f_blocks as u64 * frsize, st.f_bavail as u64 * frsize))
}

#[cfg(not(unix))]
pub fn statvfs(_path: &str) -> Option<(u64, u64)> {
    None
}
