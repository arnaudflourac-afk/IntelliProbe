//! Structure du rapport. Chaque champ est mesuré sur la machine ;
//! une valeur inconnue est `None` / vide, jamais inventée.

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Report {
    pub meta: Meta,
    pub board: Board,
    pub os: Os,
    pub cpu: Cpu,
    pub memory: Memory,
    pub storage: Storage,
    pub gpus: Vec<Gpu>,
    pub npus: Vec<Npu>,
    pub compute: Compute,
    pub video: Video,
    pub interfaces: Interfaces,
    pub dev: DevEnv,
    pub python: Option<Python>,
    pub packages: Packages,
    pub ai: Ai,
    pub bench: Option<Bench>,
    pub analysis: Analysis,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Meta {
    pub tool_version: String,
    pub generated_at: String,
    pub duration_s: f64,
    pub run_as_root: bool,
    /// Durée de chaque sonde, pour repérer les lenteurs.
    pub probe_timings: Vec<(String, f64)>,
}

// ---------------------------------------------------------------- Carte / OS

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Board {
    /// `/proc/device-tree/model` ou DMI (PC).
    pub model: Option<String>,
    pub vendor: Option<String>,
    /// `/proc/device-tree/compatible`, du plus spécifique au plus générique.
    pub dt_compatible: Vec<String>,
    /// SoC déduit de la dernière entrée `compatible` (ex. `rockchip,rk3588`).
    pub soc: Option<String>,
    /// `/sys/devices/soc0/*` quand le noyau l'expose.
    pub soc_family: Option<String>,
    pub soc_id: Option<String>,
    pub firmware: Option<String>,
    pub has_device_tree: bool,
    pub jetson: Option<Jetson>,
    pub raspberry_pi_revision: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Jetson {
    pub l4t_release: Option<String>,
    pub jetpack: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Os {
    pub hostname: Option<String>,
    pub pretty_name: Option<String>,
    pub id: Option<String>,
    pub version_id: Option<String>,
    pub kernel: Option<String>,
    /// Architecture du noyau (`uname -m`).
    pub arch: Option<String>,
    /// 32 ou 64 bits côté espace utilisateur (`getconf LONG_BIT`).
    pub userland_bits: Option<u32>,
    /// Architecture des paquets (`dpkg --print-architecture`, etc.).
    pub package_arch: Option<String>,
    pub libc: Option<String>,
    pub init: Option<String>,
    pub virtualization: Option<String>,
    pub uptime_s: Option<u64>,
    pub load_avg: Option<[f64; 3]>,
    pub desktop: Option<String>,
    pub shell: Option<String>,
    pub package_manager: Option<String>,
    pub installed_packages: Option<usize>,
}

// ---------------------------------------------------------------- CPU / mémoire / stockage

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Cpu {
    pub model: Option<String>,
    pub vendor: Option<String>,
    pub logical_cores: usize,
    pub physical_cores: Option<usize>,
    /// Groupes de cœurs identiques (big.LITTLE sur ARM).
    pub clusters: Vec<CpuCluster>,
    /// Extensions utiles pour le calcul (AVX2, NEON, dotprod, SVE…).
    pub isa_features: Vec<IsaFeature>,
    pub all_flags: Vec<String>,
    pub governor: Option<String>,
    pub temperature_c: Option<f64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CpuCluster {
    pub name: String,
    pub cpus: Vec<usize>,
    pub min_mhz: Option<u64>,
    pub max_mhz: Option<u64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IsaFeature {
    pub flag: String,
    pub description: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Memory {
    pub total_mb: u64,
    pub available_mb: u64,
    pub swap_total_mb: u64,
    pub swap_free_mb: u64,
    pub zram: Vec<Zram>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Zram {
    pub name: String,
    pub disksize_mb: u64,
    pub algorithm: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Storage {
    pub devices: Vec<BlockDevice>,
    pub filesystems: Vec<Filesystem>,
    /// Type du support qui porte `/` (eMMC, carte SD, NVMe…).
    pub root_device_kind: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BlockDevice {
    pub name: String,
    pub kind: String,
    pub size_gb: f64,
    pub model: Option<String>,
    pub removable: bool,
    pub rotational: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Filesystem {
    pub mount: String,
    pub device: String,
    pub fstype: String,
    pub total_gb: f64,
    pub free_gb: f64,
}

// ---------------------------------------------------------------- Accélérateurs

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Gpu {
    pub name: String,
    pub vendor: Option<String>,
    pub driver: Option<String>,
    pub driver_version: Option<String>,
    /// D'où vient l'information (nvidia-smi, pci, drm, mali, devfreq…).
    pub source: String,
    pub vram_mb: Option<u64>,
    pub max_freq_mhz: Option<u64>,
    pub temperature_c: Option<f64>,
    pub compute_capability: Option<String>,
    pub render_node: Option<String>,
    pub pci_id: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Npu {
    pub name: String,
    pub vendor: Option<String>,
    /// Comment il a été trouvé (device-tree, pci, usb, /dev, driver…).
    pub source: String,
    pub driver: Option<String>,
    pub driver_version: Option<String>,
    pub device_nodes: Vec<String>,
    /// `actif`, `désactivé dans le device-tree`, `pas de driver chargé`…
    pub status: String,
    /// Valeur constructeur, uniquement si la puce est identifiée précisément.
    pub datasheet_tops: Option<f64>,
    /// Librairies/paquets du runtime trouvés sur la machine.
    pub runtime_found: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Compute {
    pub cuda: Option<Cuda>,
    pub opencl_platforms: Vec<OpenClPlatform>,
    pub opencl_icds: Vec<String>,
    pub vulkan_devices: Vec<VulkanDevice>,
    pub vulkan_icds: Vec<String>,
    pub rocm_version: Option<String>,
    pub vaapi: Option<VaApi>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Cuda {
    pub driver_version: Option<String>,
    /// Version CUDA maximale supportée par le driver.
    pub driver_cuda_version: Option<String>,
    pub toolkit_version: Option<String>,
    pub cudnn: Option<String>,
    pub tensorrt: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenClPlatform {
    pub name: String,
    pub devices: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VulkanDevice {
    pub name: String,
    pub device_type: Option<String>,
    pub api_version: Option<String>,
    pub driver: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VaApi {
    pub driver: Option<String>,
    pub profiles: Vec<String>,
}

// ---------------------------------------------------------------- Vidéo

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Video {
    pub v4l2_devices: Vec<V4l2Device>,
    /// Codecs matériels effectivement exposés (V4L2 M2M, VA-API, NVENC…).
    pub hw_codecs: Vec<HwCodec>,
    /// Nœuds spécifiques constructeur (MPP Rockchip, NVENC Jetson, VCHIQ…).
    pub vendor_nodes: Vec<String>,
    pub ffmpeg: Option<Ffmpeg>,
    pub gstreamer: Option<Gstreamer>,
    pub libcamera: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct V4l2Device {
    pub path: String,
    pub name: String,
    pub driver: Option<String>,
    pub bus: Option<String>,
    /// `capture`, `sortie`, `mem2mem`, `métadonnées`.
    pub roles: Vec<String>,
    /// Formats côté capture (ce que le périphérique produit).
    pub capture_formats: Vec<String>,
    /// Formats côté sortie (ce que le périphérique consomme).
    pub output_formats: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HwCodec {
    pub codec: String,
    /// `encode` ou `decode`.
    pub direction: String,
    pub backend: String,
    pub device: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Ffmpeg {
    pub version: Option<String>,
    pub hwaccels: Vec<String>,
    /// Encodeurs matériels compilés dans ffmpeg (pas forcément utilisables).
    pub hw_encoders: Vec<String>,
    pub hw_decoders: Vec<String>,
    /// Encodeurs matériels qui ont réellement réussi un encodage de test.
    pub working_encoders: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Gstreamer {
    pub version: Option<String>,
    pub hw_elements: Vec<String>,
}

// ---------------------------------------------------------------- Interfaces

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Interfaces {
    pub gpio_chips: Vec<GpioChip>,
    pub i2c_buses: Vec<NamedDev>,
    pub spi_devices: Vec<String>,
    pub uarts: Vec<NamedDev>,
    pub can: Vec<String>,
    pub pwm_chips: Vec<NamedDev>,
    pub usb_devices: Vec<UsbDevice>,
    pub pci_devices: Vec<PciDevice>,
    pub network: Vec<NetIf>,
    pub bluetooth: Vec<String>,
    pub watchdogs: Vec<String>,
    /// Périphériques déclarés dans le device-tree (activés / désactivés).
    pub dt_peripherals: Vec<DtPeripheral>,
    pub sensors: Vec<Sensor>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GpioChip {
    pub dev: String,
    pub label: Option<String>,
    pub lines: Option<u32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NamedDev {
    pub dev: String,
    pub name: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UsbDevice {
    pub id: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub speed_mbps: Option<u64>,
    pub is_hub: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PciDevice {
    pub slot: String,
    pub id: String,
    pub class: String,
    pub description: Option<String>,
    pub driver: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetIf {
    pub name: String,
    pub kind: String,
    pub state: Option<String>,
    pub speed_mbps: Option<i64>,
    pub driver: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DtPeripheral {
    pub kind: String,
    pub enabled: usize,
    pub disabled: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Sensor {
    pub name: String,
    pub temp_c: f64,
}

// ---------------------------------------------------------------- Dev / librairies

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DevEnv {
    /// Outils présents (avec chemin et version).
    pub tools: Vec<Tool>,
    /// Outils cherchés mais absents.
    pub missing: Vec<Tool>,
    pub cross_compilers: Vec<String>,
    pub docker_daemon_access: Option<bool>,
    pub docker_runtimes: Vec<String>,
    /// Émulation d'autres architectures (binfmt/qemu) pour builds multi-arch.
    pub binfmt: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Tool {
    pub category: String,
    pub name: String,
    pub command: String,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Python {
    pub executable: String,
    pub version: Option<String>,
    pub virtualenv: Option<String>,
    pub packages: Vec<Package>,
    pub frameworks: Vec<Framework>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Package {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
}

/// Résultat d'un vrai `import` du framework dans un sous-processus.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Framework {
    pub name: String,
    pub package: String,
    pub version: Option<String>,
    pub import_ok: bool,
    /// Accélérateurs vus par le framework (ex. `cuda:0 Quadro RTX 4000`).
    pub accelerators: Vec<String>,
    pub details: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Packages {
    pub node_global: Vec<Package>,
    pub cargo_installed: Vec<Package>,
    /// Librairies avec fichier `.pc` : utilisables pour compiler (headers installés).
    pub pkg_config: Vec<Package>,
    pub shared_libs_count: usize,
    /// Librairies remarquables (IA, GPU, vidéo, maths…) trouvées sur le système.
    pub notable_libs: Vec<SharedLib>,
    /// Toutes les librairies partagées du cache `ldconfig`.
    pub all_shared_libs: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SharedLib {
    pub soname: String,
    pub path: String,
    pub category: String,
    pub description: String,
}

// ---------------------------------------------------------------- IA

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Ai {
    /// Outils d'inférence présents (ollama, llama.cpp, trtexec, hailortcli…).
    pub runtimes: Vec<Tool>,
    pub ollama_models: Vec<String>,
    /// Pour chaque voie d'accélération : ce qui est présent, ce qui manque.
    pub paths: Vec<AccelPath>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AccelPath {
    pub name: String,
    pub hardware: bool,
    pub driver: bool,
    pub runtime: bool,
    pub bindings: Vec<String>,
    /// `utilisable`, `partiel`, `absent`.
    pub status: String,
    pub detail: String,
}

// ---------------------------------------------------------------- Mesures

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Bench {
    pub threads: usize,
    pub cpu_single_gflops: f64,
    pub cpu_multi_gflops: f64,
    pub mem_buffer_mb: u64,
    pub mem_read_single_gbs: f64,
    pub mem_read_multi_gbs: f64,
    pub disk: Option<DiskBench>,
    pub duration_s: f64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiskBench {
    pub path: String,
    pub size_mb: u64,
    pub write_mbs: f64,
    pub read_mbs: Option<f64>,
}

// ---------------------------------------------------------------- Analyse

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub llm: Option<LlmEstimate>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Finding {
    /// `ok`, `info`, `warn`, `missing`.
    pub level: String,
    pub category: String,
    pub title: String,
    pub detail: String,
    pub hint: Option<String>,
}

/// Estimations dérivées des mesures (formules explicites, pas de valeurs figées).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmEstimate {
    pub ram_budget_gb: f64,
    pub vram_budget_gb: Option<f64>,
    pub measured_bandwidth_gbs: Option<f64>,
    pub bytes_per_param: f64,
    pub rows: Vec<LlmRow>,
    pub method: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmRow {
    pub params_b: f64,
    pub size_gb: f64,
    pub fits_ram: bool,
    pub fits_vram: Option<bool>,
    pub max_tokens_per_s: Option<f64>,
}
