//! CPU : modèle, clusters (big.LITTLE), fréquences, extensions SIMD, température.

use crate::report::{Cpu, CpuCluster, IsaFeature, Sensor};
use crate::util::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

const CPU_SYS: &str = "/sys/devices/system/cpu";

/// Un bloc « processor » de /proc/cpuinfo.
#[derive(Debug, Default)]
struct CpuEntry {
    id: usize,
    name: Option<String>,
    flags: Vec<String>,
}

pub fn probe() -> Cpu {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let (entries, global) = parse_cpuinfo(&cpuinfo);
    let lscpu_names = lscpu_model_names();

    let mut cpu = Cpu {
        logical_cores: if entries.is_empty() {
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
        } else {
            entries.len()
        },
        vendor: global.vendor.clone(),
        ..Default::default()
    };

    // Clusters : CPU regroupés par (nom de cœur, fréquence max).
    let mut groups: BTreeMap<(String, Option<u64>), Vec<usize>> = BTreeMap::new();
    for e in &entries {
        let name = e
            .name
            .clone()
            .or_else(|| (lscpu_names.len() == 1).then(|| lscpu_names[0].clone()))
            .unwrap_or_else(|| "cœur inconnu".into());
        let max = read_u64(format!("{}/cpu{}/cpufreq/cpuinfo_max_freq", CPU_SYS, e.id)).map(|k| k / 1000);
        groups.entry((name, max)).or_default().push(e.id);
    }
    for ((name, max), cpus) in groups {
        let min = read_u64(format!("{}/cpu{}/cpufreq/cpuinfo_min_freq", CPU_SYS, cpus[0])).map(|k| k / 1000);
        cpu.clusters.push(CpuCluster { name, cpus, min_mhz: min, max_mhz: max });
    }
    // Plus gros cœurs en premier.
    cpu.clusters.sort_by(|a, b| b.max_mhz.cmp(&a.max_mhz).then(b.cpus.len().cmp(&a.cpus.len())));

    cpu.model = global.model.clone().or_else(|| {
        if cpu.clusters.is_empty() {
            None
        } else {
            Some(
                cpu.clusters
                    .iter()
                    .map(|c| format!("{}× {}", c.cpus.len(), c.name))
                    .collect::<Vec<_>>()
                    .join(" + "),
            )
        }
    });

    cpu.physical_cores = count_physical_cores(cpu.logical_cores);

    let flags: BTreeSet<String> = entries.iter().flat_map(|e| e.flags.iter().cloned()).collect();
    cpu.all_flags = flags.iter().cloned().collect();
    cpu.isa_features = notable_features(&flags);
    cpu.governor = read_trim(format!("{}/cpu0/cpufreq/scaling_governor", CPU_SYS));
    cpu.temperature_c = cpu_temperature(&read_sensors());
    cpu
}

#[derive(Debug, Default)]
struct CpuGlobal {
    model: Option<String>,
    vendor: Option<String>,
}

fn parse_cpuinfo(text: &str) -> (Vec<CpuEntry>, CpuGlobal) {
    let mut entries = Vec::new();
    let mut global = CpuGlobal::default();
    let mut hardware: Option<String> = None;

    for block in text.split("\n\n") {
        let mut e = CpuEntry::default();
        let mut is_proc = false;
        let (mut implementer, mut part) = (None, None);
        for line in block.lines() {
            let Some((k, v)) = line.split_once(':') else { continue };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "processor" => {
                    if let Ok(id) = v.parse() {
                        e.id = id;
                        is_proc = true;
                    }
                }
                "model name" | "cpu model" => {
                    // Sur ARM 32 bits, « model name » vaut « ARMv7 Processor rev 3 (v7l) » : peu utile.
                    if !v.starts_with("ARMv") {
                        e.name = Some(v.to_string());
                    }
                }
                "uarch" => e.name = Some(v.to_string()),
                "vendor_id" => global.vendor = Some(v.to_string()),
                "flags" | "Features" => e.flags = v.split_whitespace().map(String::from).collect(),
                "isa" => e.flags = riscv_isa_flags(v),
                "CPU implementer" => implementer = u32::from_str_radix(v.trim_start_matches("0x"), 16).ok(),
                "CPU part" => part = u32::from_str_radix(v.trim_start_matches("0x"), 16).ok(),
                "Hardware" => hardware = Some(v.to_string()),
                _ => {}
            }
        }
        if let (Some(i), Some(p)) = (implementer, part) {
            if global.vendor.is_none() {
                global.vendor = arm_implementer(i).map(String::from);
            }
            if e.name.is_none() {
                e.name = Some(arm_part_name(i, p).unwrap_or_else(|| format!("ARM impl 0x{:02x} part 0x{:03x}", i, p)));
            }
        }
        if is_proc {
            entries.push(e);
        }
    }
    // Sur x86 le « model name » est commun ; sur ARM on construit le modèle depuis les clusters.
    let names: BTreeSet<_> = entries.iter().filter_map(|e| e.name.clone()).collect();
    if names.len() == 1 && !entries.iter().any(|e| e.name.as_deref().is_some_and(|n| n.starts_with("Cortex") || n.starts_with("Neoverse"))) {
        global.model = names.into_iter().next();
    }
    if global.model.is_none() && entries.iter().all(|e| e.name.is_none()) {
        global.model = hardware;
    }
    (entries, global)
}

/// `lscpu` décode aussi les cœurs ARM : utilisé en secours si notre table ne connaît pas le cœur.
fn lscpu_model_names() -> Vec<String> {
    run_ok("lscpu", &[])
        .map(|o| o.lines().filter_map(|l| colon_value(l, "Model name").map(String::from)).collect())
        .unwrap_or_default()
}

fn count_physical_cores(logical: usize) -> Option<usize> {
    let mut sets = BTreeSet::new();
    for i in 0..logical {
        let base = format!("{}/cpu{}/topology", CPU_SYS, i);
        let s = read_trim(format!("{}/core_cpus_list", base)).or_else(|| read_trim(format!("{}/thread_siblings_list", base)))?;
        sets.insert(s);
    }
    (!sets.is_empty()).then_some(sets.len())
}

fn arm_implementer(id: u32) -> Option<&'static str> {
    Some(match id {
        0x41 => "ARM",
        0x42 => "Broadcom",
        0x43 => "Cavium",
        0x48 => "HiSilicon",
        0x4e => "NVIDIA",
        0x51 => "Qualcomm",
        0x53 => "Samsung",
        0x56 => "Marvell",
        0x61 => "Apple",
        0x69 => "Intel",
        0x6d => "Microsoft",
        0xc0 => "Ampere",
        _ => return None,
    })
}

/// Décodage des registres MIDR (même table que `lscpu`).
fn arm_part_name(implementer: u32, part: u32) -> Option<String> {
    let name = match (implementer, part) {
        (0x41, 0xc05) => "Cortex-A5",
        (0x41, 0xc07) => "Cortex-A7",
        (0x41, 0xc08) => "Cortex-A8",
        (0x41, 0xc09) => "Cortex-A9",
        (0x41, 0xc0d) | (0x41, 0xc0e) => "Cortex-A17",
        (0x41, 0xc0f) => "Cortex-A15",
        (0x41, 0xd01) => "Cortex-A32",
        (0x41, 0xd02) => "Cortex-A34",
        (0x41, 0xd03) => "Cortex-A53",
        (0x41, 0xd04) => "Cortex-A35",
        (0x41, 0xd05) => "Cortex-A55",
        (0x41, 0xd06) => "Cortex-A65",
        (0x41, 0xd07) => "Cortex-A57",
        (0x41, 0xd08) => "Cortex-A72",
        (0x41, 0xd09) => "Cortex-A73",
        (0x41, 0xd0a) => "Cortex-A75",
        (0x41, 0xd0b) => "Cortex-A76",
        (0x41, 0xd0c) => "Neoverse-N1",
        (0x41, 0xd0d) => "Cortex-A77",
        (0x41, 0xd0e) => "Cortex-A76AE",
        (0x41, 0xd40) => "Neoverse-V1",
        (0x41, 0xd41) => "Cortex-A78",
        (0x41, 0xd42) => "Cortex-A78AE",
        (0x41, 0xd43) => "Cortex-A65AE",
        (0x41, 0xd44) => "Cortex-X1",
        (0x41, 0xd46) => "Cortex-A510",
        (0x41, 0xd47) => "Cortex-A710",
        (0x41, 0xd48) => "Cortex-X2",
        (0x41, 0xd49) => "Neoverse-N2",
        (0x41, 0xd4a) => "Neoverse-E1",
        (0x41, 0xd4b) => "Cortex-A78C",
        (0x41, 0xd4c) => "Cortex-X1C",
        (0x41, 0xd4d) => "Cortex-A715",
        (0x41, 0xd4e) => "Cortex-X3",
        (0x41, 0xd4f) => "Neoverse-V2",
        (0x41, 0xd80) => "Cortex-A520",
        (0x41, 0xd81) => "Cortex-A720",
        (0x41, 0xd82) => "Cortex-X4",
        (0x4e, 0x003) => "NVIDIA Denver 2",
        (0x4e, 0x004) => "NVIDIA Carmel",
        (0x51, 0x800) => "Kryo 2xx Gold",
        (0x51, 0x801) => "Kryo 2xx Silver",
        (0x51, 0x802) => "Kryo 3xx Gold",
        (0x51, 0x803) => "Kryo 3xx Silver",
        (0x51, 0x804) => "Kryo 4xx Gold",
        (0x51, 0x805) => "Kryo 4xx Silver",
        _ => return None,
    };
    Some(name.to_string())
}

/// `rv64imafdcv_zicsr_zba` -> [i, m, a, f, d, c, v, zicsr, zba]
fn riscv_isa_flags(isa: &str) -> Vec<String> {
    let isa = isa.to_lowercase();
    let mut parts = isa.split('_');
    let mut flags = Vec::new();
    if let Some(base) = parts.next() {
        let letters = base.trim_start_matches("rv64").trim_start_matches("rv32");
        flags.extend(letters.chars().map(|c| c.to_string()));
    }
    flags.extend(parts.map(String::from));
    flags
}

/// Extensions utiles au calcul, avec leur intérêt pratique.
fn notable_features(flags: &BTreeSet<String>) -> Vec<IsaFeature> {
    const TABLE: &[(&str, &str)] = &[
        // x86
        ("sse4_2", "SSE4.2"),
        ("avx", "AVX (SIMD 256 bits)"),
        ("avx2", "AVX2 : requis par la plupart des builds IA x86 optimisés"),
        ("fma", "FMA3 : multiplication-addition fusionnée"),
        ("f16c", "Conversion FP16"),
        ("avx512f", "AVX-512"),
        ("avx512_vnni", "AVX-512 VNNI : accélère l'inférence int8"),
        ("avx512_bf16", "AVX-512 BF16"),
        ("avx512_fp16", "AVX-512 FP16"),
        ("avx_vnni", "AVX-VNNI : accélère l'inférence int8"),
        ("amx_tile", "AMX : tuiles matricielles (Xeon récents)"),
        ("amx_int8", "AMX int8"),
        ("amx_bf16", "AMX BF16"),
        ("aes", "AES matériel"),
        ("sha_ni", "SHA matériel"),
        // ARM 64 bits
        ("asimd", "NEON / Advanced SIMD (128 bits)"),
        ("asimdhp", "Arithmétique FP16 vectorielle"),
        ("asimddp", "Produit scalaire int8 (dotprod) : gros gain pour llama.cpp et l'int8"),
        ("i8mm", "Multiplication matricielle int8 (i8mm)"),
        ("bf16", "BF16"),
        ("sve", "SVE (SIMD à longueur variable)"),
        ("sve2", "SVE2"),
        ("sme", "SME (matrices)"),
        ("atomics", "Atomiques LSE"),
        ("sha2", "SHA-2 matériel"),
        ("crc32", "CRC32 matériel"),
        // ARM 32 bits
        ("neon", "NEON (SIMD 128 bits)"),
        ("vfpv4", "VFPv4"),
        // RISC-V
        ("v", "RVV : extension vectorielle RISC-V"),
        ("zvfh", "Vecteurs FP16"),
        ("zfh", "FP16 scalaire"),
        ("zba", "Manipulation de bits (Zba)"),
        ("zbb", "Manipulation de bits (Zbb)"),
        ("xtheadvector", "Vecteurs T-Head (RVV 0.7.1)"),
    ];
    TABLE
        .iter()
        .filter(|(f, _)| flags.contains(*f))
        .map(|(f, d)| IsaFeature { flag: f.to_string(), description: d.to_string() })
        .collect()
}

/// Tous les capteurs de température (thermal zones + hwmon).
pub fn read_sensors() -> Vec<Sensor> {
    let mut out = Vec::new();
    for z in list_dir_prefix("/sys/class/thermal", "thermal_zone") {
        let base = format!("/sys/class/thermal/{}", z);
        if let (Some(name), Some(t)) = (read_trim(format!("{}/type", base)), read_trim(format!("{}/temp", base)).and_then(|t| t.parse::<f64>().ok())) {
            if t > -50_000.0 && t < 200_000.0 {
                out.push(Sensor { name, temp_c: round1(t / 1000.0) });
            }
        }
    }
    for h in list_dir_prefix("/sys/class/hwmon", "hwmon") {
        let base = format!("/sys/class/hwmon/{}", h);
        let chip = read_trim(format!("{}/name", base)).unwrap_or(h.clone());
        for f in list_dir(&base) {
            let Some(idx) = f.strip_prefix("temp").and_then(|r| r.strip_suffix("_input")) else { continue };
            let Some(t) = read_trim(format!("{}/{}", base, f)).and_then(|t| t.parse::<f64>().ok()) else { continue };
            if t <= -50_000.0 || t >= 200_000.0 {
                continue;
            }
            let name = match read_trim(format!("{}/temp{}_label", base, idx)) {
                Some(l) => format!("{} {}", chip, l),
                None => chip.clone(),
            };
            if !out.iter().any(|s: &Sensor| s.name == name) {
                out.push(Sensor { name, temp_c: round1(t / 1000.0) });
            }
        }
    }
    out
}

fn cpu_temperature(sensors: &[Sensor]) -> Option<f64> {
    const PRIORITY: &[&str] = &["x86_pkg_temp", "coretemp package", "k10temp tctl", "k10temp tdie", "cpu", "soc", "bigcore", "k10temp", "coretemp", "acpitz"];
    PRIORITY.iter().find_map(|p| sensors.iter().find(|s| s.name.to_lowercase().contains(p)).map(|s| s.temp_c))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RK3588: &str = "processor\t: 0\nBogoMIPS\t: 48.00\nFeatures\t: fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics fphp asimdhp cpuid asimdrdm lrcpc dcpop asimddp\nCPU implementer\t: 0x41\nCPU architecture: 8\nCPU variant\t: 0x2\nCPU part\t: 0xd05\nCPU revision\t: 0\n\nprocessor\t: 4\nFeatures\t: fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics fphp asimdhp cpuid asimdrdm lrcpc dcpop asimddp\nCPU implementer\t: 0x41\nCPU architecture: 8\nCPU variant\t: 0x4\nCPU part\t: 0xd0b\nCPU revision\t: 0\n";

    #[test]
    fn arm_big_little() {
        let (entries, global) = parse_cpuinfo(RK3588);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name.as_deref(), Some("Cortex-A55"));
        assert_eq!(entries[1].name.as_deref(), Some("Cortex-A76"));
        assert_eq!(global.vendor.as_deref(), Some("ARM"));
        assert!(global.model.is_none());
        let flags: BTreeSet<String> = entries[0].flags.iter().cloned().collect();
        let f = notable_features(&flags);
        assert!(f.iter().any(|x| x.flag == "asimddp"));
    }

    #[test]
    fn x86() {
        let t = "processor\t: 0\nvendor_id\t: AuthenticAMD\nmodel name\t: AMD Ryzen 5 3600 6-Core Processor\nflags\t\t: fpu sse4_2 avx avx2 fma\n\nprocessor\t: 1\nvendor_id\t: AuthenticAMD\nmodel name\t: AMD Ryzen 5 3600 6-Core Processor\nflags\t\t: fpu sse4_2 avx avx2 fma\n";
        let (entries, global) = parse_cpuinfo(t);
        assert_eq!(entries.len(), 2);
        assert_eq!(global.model.as_deref(), Some("AMD Ryzen 5 3600 6-Core Processor"));
    }

    #[test]
    fn riscv() {
        let f = riscv_isa_flags("rv64imafdcv_zicsr_zba_zbb");
        assert!(f.contains(&"v".to_string()));
        assert!(f.contains(&"zba".to_string()));
    }

    #[test]
    fn temp_priority() {
        let s = vec![
            Sensor { name: "nvme Composite".into(), temp_c: 40.0 },
            Sensor { name: "k10temp Tctl".into(), temp_c: 55.0 },
        ];
        assert_eq!(cpu_temperature(&s), Some(55.0));
    }
}
