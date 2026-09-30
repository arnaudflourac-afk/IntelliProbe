//! Librairies : cache ldconfig complet, dossiers hors cache (CUDA, Tegra, ROCm…),
//! librairies de développement (pkg-config), paquets npm globaux et binaires cargo.

use crate::report::{Package, Packages, SharedLib};
use crate::util::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::OnceLock;

/// (soname, chemin) de toutes les librairies partagées connues.
pub fn ld_cache() -> &'static [(String, String)] {
    static CACHE: OnceLock<Vec<(String, String)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut libs = BTreeMap::new();
        let ldconfig = find_exe("ldconfig").map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "ldconfig".into());
        if let Some(out) = run_ok(&ldconfig, &["-p"]) {
            for (so, path) in parse_ldconfig(&out) {
                libs.entry(so).or_insert(path);
            }
        }
        // Dossiers souvent absents du cache ldconfig.
        for dir in extra_lib_dirs() {
            for f in list_dir(&dir) {
                if f.starts_with("lib") && f.contains(".so") {
                    libs.entry(f.clone()).or_insert_with(|| format!("{}/{}", dir, f));
                }
            }
        }
        libs.into_iter().collect()
    })
}

fn parse_ldconfig(out: &str) -> Vec<(String, String)> {
    out.lines()
        .skip(1)
        .filter_map(|l| {
            let (left, path) = l.split_once("=>")?;
            let so = left.split_whitespace().next()?;
            Some((so.to_string(), path.trim().to_string()))
        })
        .collect()
}

fn extra_lib_dirs() -> Vec<String> {
    let mut dirs: Vec<String> = [
        "/usr/local/cuda/lib64",
        "/usr/local/cuda/lib",
        "/usr/lib/aarch64-linux-gnu/tegra",
        "/usr/lib/aarch64-linux-gnu/nvidia",
        "/opt/rocm/lib",
        "/usr/lib/rknpu",
        "/opt/hailo/lib",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for base in ["/usr/local/cuda/targets", "/opt/intel"] {
        for sub in list_dir(base) {
            for lib in ["lib", "runtime/lib/intel64", "runtime/lib/aarch64"] {
                dirs.push(format!("{}/{}/{}", base, sub, lib));
            }
        }
    }
    dirs.into_iter().filter(|d| Path::new(d).is_dir()).collect()
}

/// Cherche une librairie par préfixe de soname (ex. `librknnrt.so`).
pub fn find_lib(prefix: &str) -> Option<&'static (String, String)> {
    ld_cache().iter().find(|(so, _)| so.starts_with(prefix))
}

pub fn has_lib(prefix: &str) -> bool {
    find_lib(prefix).is_some()
}

/// Librairies remarquables : (préfixe du soname, catégorie, description).
pub const NOTABLE: &[(&str, &str, &str)] = &[
    // NPU / accélérateurs IA
    ("librknnrt.so", "NPU", "Runtime RKNN (NPU Rockchip)"),
    ("librknn_api.so", "NPU", "RKNN API (ancien runtime Rockchip)"),
    ("librkllmrt.so", "NPU", "RKLLM : LLM sur NPU Rockchip"),
    ("libhailort.so", "NPU", "HailoRT (Hailo-8/8L/10)"),
    ("libedgetpu.so", "NPU", "Runtime Coral Edge TPU"),
    ("libQnnHtp.so", "NPU", "Qualcomm QNN (HTP/Hexagon)"),
    ("libSNPE.so", "NPU", "Qualcomm SNPE"),
    ("libopenvino.so", "NPU", "OpenVINO (CPU/GPU/NPU Intel)"),
    ("libtim-vx.so", "NPU", "VeriSilicon TIM-VX (NPU Amlogic/NXP)"),
    ("libOpenVX.so", "NPU", "OpenVX (Vivante/VeriSilicon)"),
    ("libvx_delegate.so", "NPU", "Délégué TFLite VX (NPU NXP i.MX)"),
    ("libethosu", "NPU", "Arm Ethos-U"),
    ("libneuron_runtime", "NPU", "MediaTek NeuroPilot"),
    ("libnvdla", "NPU", "NVIDIA DLA"),
    // GPU / calcul
    ("libcuda.so", "GPU / calcul", "Driver CUDA"),
    ("libcudart.so", "GPU / calcul", "Runtime CUDA"),
    ("libcublas.so", "GPU / calcul", "cuBLAS"),
    ("libcudnn.so", "GPU / calcul", "cuDNN"),
    ("libnvinfer.so", "GPU / calcul", "TensorRT"),
    ("libamdhip64.so", "GPU / calcul", "HIP (ROCm AMD)"),
    ("librocblas.so", "GPU / calcul", "rocBLAS"),
    ("libMIOpen.so", "GPU / calcul", "MIOpen (ROCm)"),
    ("libze_loader.so", "GPU / calcul", "oneAPI Level Zero"),
    ("libOpenCL.so", "GPU / calcul", "Loader OpenCL"),
    ("libmali.so", "GPU / calcul", "Pilote Mali propriétaire (GLES/OpenCL)"),
    ("libvulkan.so", "GPU / calcul", "Loader Vulkan"),
    ("libarm_compute.so", "GPU / calcul", "Arm Compute Library (NEON/Mali)"),
    ("libarmnn.so", "GPU / calcul", "Arm NN"),
    // Graphique
    ("libGLESv2.so", "Graphique", "OpenGL ES 2/3"),
    ("libEGL.so", "Graphique", "EGL"),
    ("libGL.so", "Graphique", "OpenGL"),
    ("libgbm.so", "Graphique", "GBM (Mesa)"),
    ("libdrm.so", "Graphique", "libdrm"),
    ("libwayland-client.so", "Graphique", "Wayland"),
    ("libX11.so", "Graphique", "X11"),
    ("libSDL2", "Graphique", "SDL2"),
    ("libglfw.so", "Graphique", "GLFW"),
    ("libQt5Core.so", "Graphique", "Qt 5"),
    ("libQt6Core.so", "Graphique", "Qt 6"),
    ("libgtk-3.so", "Graphique", "GTK 3"),
    ("libgtk-4.so", "Graphique", "GTK 4"),
    // Machine learning
    ("libonnxruntime.so", "Machine learning", "ONNX Runtime (C/C++)"),
    ("libtensorflowlite_c.so", "Machine learning", "TensorFlow Lite (C)"),
    ("libtensorflowlite.so", "Machine learning", "TensorFlow Lite (C++)"),
    ("libtensorflow.so", "Machine learning", "TensorFlow (C)"),
    ("libtorch.so", "Machine learning", "LibTorch (C++)"),
    ("libncnn.so", "Machine learning", "ncnn (Tencent, optimisé ARM/Vulkan)"),
    ("libMNN.so", "Machine learning", "MNN (Alibaba)"),
    ("libpaddle_inference", "Machine learning", "Paddle Inference"),
    ("libllama.so", "Machine learning", "llama.cpp"),
    ("libggml.so", "Machine learning", "ggml"),
    ("libwhisper.so", "Machine learning", "whisper.cpp"),
    ("libdnnl.so", "Machine learning", "oneDNN"),
    // Maths
    ("libopenblas", "Maths", "OpenBLAS"),
    ("libblas.so", "Maths", "BLAS"),
    ("liblapack.so", "Maths", "LAPACK"),
    ("libmkl_rt.so", "Maths", "Intel MKL"),
    ("libfftw3f.so", "Maths", "FFTW (float)"),
    ("libfftw3.so", "Maths", "FFTW"),
    ("libgomp.so", "Maths", "OpenMP (GCC)"),
    ("libomp.so", "Maths", "OpenMP (LLVM)"),
    ("libtbb.so", "Maths", "oneTBB"),
    ("libmpi.so", "Maths", "MPI"),
    // Vision
    ("libopencv_core.so", "Vision", "OpenCV"),
    ("libopencv_dnn.so", "Vision", "OpenCV DNN"),
    ("libcamera.so", "Vision", "libcamera (caméras MIPI)"),
    ("libv4l2.so", "Vision", "libv4l2"),
    ("libturbojpeg.so", "Vision", "libjpeg-turbo"),
    ("libzbar.so", "Vision", "ZBar (codes-barres)"),
    // Vidéo
    ("libavcodec.so", "Vidéo", "FFmpeg libavcodec"),
    ("libgstreamer-1.0.so", "Vidéo", "GStreamer"),
    ("librockchip_mpp.so", "Vidéo", "Rockchip MPP (codecs matériels)"),
    ("librga.so", "Vidéo", "Rockchip RGA (2D matériel)"),
    ("libva.so", "Vidéo", "VA-API"),
    ("libvdpau.so", "Vidéo", "VDPAU"),
    ("libnvcuvid.so", "Vidéo", "NVDEC (décodage NVIDIA)"),
    ("libnvidia-encode.so", "Vidéo", "NVENC (encodage NVIDIA)"),
    ("libnvv4l2.so", "Vidéo", "V4L2 NVIDIA Jetson"),
    ("libvpl.so", "Vidéo", "Intel oneVPL (QSV)"),
    ("libmfx.so", "Vidéo", "Intel Media SDK (QSV)"),
    ("libx264.so", "Vidéo", "x264"),
    ("libx265.so", "Vidéo", "x265"),
    ("libdav1d.so", "Vidéo", "dav1d (AV1)"),
    ("libSvtAv1Enc.so", "Vidéo", "SVT-AV1"),
    ("libvpx.so", "Vidéo", "libvpx (VP8/VP9)"),
    // Audio
    ("libasound.so", "Audio", "ALSA"),
    ("libpulse.so", "Audio", "PulseAudio"),
    ("libpipewire-0.3.so", "Audio", "PipeWire"),
    ("libjack.so", "Audio", "JACK"),
    ("libportaudio.so", "Audio", "PortAudio"),
    ("libsndfile.so", "Audio", "libsndfile"),
    // Entrées/sorties matérielles
    ("libgpiod.so", "E/S matérielles", "libgpiod (GPIO)"),
    ("libi2c.so", "E/S matérielles", "libi2c"),
    ("libusb-1.0.so", "E/S matérielles", "libusb"),
    ("libserialport.so", "E/S matérielles", "libserialport"),
    ("libmodbus.so", "E/S matérielles", "libmodbus"),
    ("libiio.so", "E/S matérielles", "libiio (capteurs IIO)"),
    ("libwiringPi.so", "E/S matérielles", "WiringPi"),
    ("libpigpio.so", "E/S matérielles", "pigpio"),
    ("liblgpio.so", "E/S matérielles", "lgpio"),
    ("libmraa.so", "E/S matérielles", "MRAA"),
    ("libbluetooth.so", "E/S matérielles", "BlueZ"),
    ("libudev.so", "E/S matérielles", "libudev"),
    // Réseau
    ("libssl.so", "Réseau", "OpenSSL"),
    ("libcurl.so", "Réseau", "libcurl"),
    ("libzmq.so", "Réseau", "ZeroMQ"),
    ("libmosquitto.so", "Réseau", "Mosquitto (MQTT)"),
    ("libpaho-mqtt3", "Réseau", "Paho MQTT"),
    ("libprotobuf.so", "Réseau", "Protocol Buffers"),
    ("libgrpc.so", "Réseau", "gRPC"),
    ("libwebsockets.so", "Réseau", "libwebsockets"),
    // Données
    ("libsqlite3.so", "Données", "SQLite"),
    ("libpq.so", "Données", "PostgreSQL (client)"),
    ("libmariadb.so", "Données", "MariaDB (client)"),
    ("libmysqlclient.so", "Données", "MySQL (client)"),
    ("libhiredis.so", "Données", "Redis (hiredis)"),
    ("libduckdb.so", "Données", "DuckDB"),
    ("libhdf5.so", "Données", "HDF5"),
    ("libzstd.so", "Données", "Zstandard"),
    ("libboost_system", "Données", "Boost"),
];

pub fn probe() -> Packages {
    let cache = ld_cache();
    let mut p = Packages { shared_libs_count: cache.len(), ..Default::default() };
    p.all_shared_libs = cache.iter().map(|(so, _)| so.clone()).collect();
    for (prefix, cat, desc) in NOTABLE {
        if let Some((so, path)) = find_lib(prefix) {
            p.notable_libs.push(SharedLib { soname: so.clone(), path: path.clone(), category: cat.to_string(), description: desc.to_string() });
        }
    }
    p.pkg_config = pkg_config();
    p.node_global = npm_global();
    p.cargo_installed = cargo_installed();
    p
}

fn pkg_config() -> Vec<Package> {
    let Some(out) = run_ok("pkg-config", &["--list-all"]) else { return Vec::new() };
    let mut seen = BTreeSet::new();
    let mut list: Vec<Package> = out
        .lines()
        .filter_map(|l| {
            let (name, desc) = l.split_once(char::is_whitespace)?;
            seen.insert(name.to_string()).then(|| Package { name: name.to_string(), version: None, description: Some(desc.trim().to_string()) })
        })
        .collect();
    // Versions en un seul appel : pkg-config imprime une ligne par module, dans l'ordre.
    let names: Vec<&str> = list.iter().map(|p| p.name.as_str()).collect();
    if let Some(v) = run_ok("pkg-config", &[&["--modversion"], names.as_slice()].concat()) {
        let versions: Vec<&str> = v.lines().collect();
        if versions.len() == list.len() {
            for (p, v) in list.iter_mut().zip(versions) {
                p.version = Some(v.trim().to_string());
            }
        }
    }
    list.sort_by_key(|p| p.name.to_lowercase());
    list
}

fn npm_global() -> Vec<Package> {
    let Some(out) = run("npm", &["ls", "-g", "--depth=0", "--json"]) else { return Vec::new() };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&out.stdout) else { return Vec::new() };
    json.get("dependencies")
        .and_then(|d| d.as_object())
        .map(|deps| {
            deps.iter()
                .map(|(name, info)| Package { name: name.clone(), version: info.get("version").and_then(|v| v.as_str()).map(String::from), description: None })
                .collect()
        })
        .unwrap_or_default()
}

fn cargo_installed() -> Vec<Package> {
    let Some(out) = run_ok("cargo", &["install", "--list"]) else { return Vec::new() };
    parse_cargo_list(&out)
}

/// Lignes `ripgrep v14.1.0:` (les binaires sont indentés en dessous).
fn parse_cargo_list(out: &str) -> Vec<Package> {
    out.lines()
        .filter(|l| !l.starts_with(char::is_whitespace) && l.ends_with(':'))
        .filter_map(|l| {
            let mut it = l.trim_end_matches(':').split_whitespace();
            let name = it.next()?;
            let version = it.next().map(|v| v.trim_start_matches('v').to_string());
            Some(Package { name: name.to_string(), version, description: None })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ldconfig() {
        let out = "1234 libs found in cache `/etc/ld.so.cache'\n\tlibrknnrt.so (libc6,AArch64) => /usr/lib/librknnrt.so\n\tlibz.so.1 (libc6,AArch64) => /lib/aarch64-linux-gnu/libz.so.1\n";
        let v = parse_ldconfig(out);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], ("librknnrt.so".to_string(), "/usr/lib/librknnrt.so".to_string()));
    }

    #[test]
    fn cargo_list() {
        let out = "ripgrep v14.1.0:\n    rg\njust v1.25.0 (/home/x/just):\n    just\n";
        let v = parse_cargo_list(out);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].version.as_deref(), Some("14.1.0"));
    }
}
