//! API de calcul : CUDA, OpenCL, Vulkan, ROCm, VA-API.

use super::libs::{find_lib, has_lib};
use crate::report::{Compute, Cuda, HwCodec, OpenClPlatform, VaApi, VulkanDevice};
use crate::util::*;
use std::fs;
use std::time::Duration;

pub fn probe() -> Compute {
    let mut c = Compute::default();
    c.cuda = cuda();
    c.opencl_platforms = run_timeout("clinfo", &["-l"], Duration::from_secs(20)).map(|o| parse_clinfo_l(&o.stdout)).unwrap_or_default();
    c.opencl_icds = list_dir("/etc/OpenCL/vendors");
    c.vulkan_devices = run_timeout("vulkaninfo", &["--summary"], Duration::from_secs(20)).map(|o| parse_vulkan_summary(&o.stdout)).unwrap_or_default();
    c.vulkan_icds = ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"].iter().flat_map(list_dir).collect();
    c.rocm_version = read_trim("/opt/rocm/.info/version");
    c.vaapi = vaapi();
    c
}

fn cuda() -> Option<Cuda> {
    let smi = run_ok("nvidia-smi", &[]);
    let nvcc = find_exe("nvcc").and_then(|p| run_ok(&p.to_string_lossy(), &["--version"]));
    let libcuda = has_lib("libcuda.so");
    if smi.is_none() && nvcc.is_none() && !libcuda {
        return None;
    }
    let mut cuda = Cuda::default();
    if let Some(s) = &smi {
        cuda.driver_version = after_label(s, "Driver Version:");
        cuda.driver_cuda_version = after_label(s, "CUDA Version:");
    }
    cuda.toolkit_version = nvcc.as_deref().and_then(|t| after_label(t, "release")).map(|v| v.trim_end_matches(',').to_string());
    cuda.cudnn = header_version(&["/usr/include/cudnn_version.h", "/usr/include/x86_64-linux-gnu/cudnn_version_v9.h", "/usr/include/aarch64-linux-gnu/cudnn_version_v9.h", "/usr/include/x86_64-linux-gnu/cudnn_version_v8.h", "/usr/include/aarch64-linux-gnu/cudnn_version_v8.h", "/usr/local/cuda/include/cudnn_version.h"], "CUDNN_MAJOR", "CUDNN_MINOR", "CUDNN_PATCHLEVEL")
        .or_else(|| find_lib("libcudnn.so").and_then(|(so, _)| so_version(so)));
    cuda.tensorrt = header_version(&["/usr/include/x86_64-linux-gnu/NvInferVersion.h", "/usr/include/aarch64-linux-gnu/NvInferVersion.h", "/usr/include/NvInferVersion.h"], "NV_TENSORRT_MAJOR", "NV_TENSORRT_MINOR", "NV_TENSORRT_PATCH")
        .or_else(|| find_lib("libnvinfer.so").and_then(|(so, _)| so_version(so)));
    Some(cuda)
}

/// Premier mot après une étiquette (ex. `CUDA Version: 12.4` -> `12.4`).
fn after_label(text: &str, label: &str) -> Option<String> {
    let rest = text.split(label).nth(1)?;
    rest.split_whitespace().next().map(String::from)
}

fn header_version(paths: &[&str], major: &str, minor: &str, patch: &str) -> Option<String> {
    let text = paths.iter().find_map(|p| fs::read_to_string(p).ok())?;
    let get = |k: &str| {
        text.lines().find_map(|l| {
            let mut it = l.split_whitespace();
            (it.next() == Some("#define") && it.next() == Some(k)).then(|| it.next().map(String::from)).flatten()
        })
    };
    Some(format!("{}.{}.{}", get(major)?, get(minor)?, get(patch).unwrap_or_else(|| "0".into())))
}

/// `libcudnn.so.8.9.4` -> `8.9.4`
fn so_version(so: &str) -> Option<String> {
    let v = so.split(".so.").nth(1)?;
    (!v.is_empty()).then(|| v.to_string())
}

/// `clinfo -l` :
/// ```text
/// Platform #0: NVIDIA CUDA
///  `-- Device #0: Quadro RTX 4000
/// ```
fn parse_clinfo_l(out: &str) -> Vec<OpenClPlatform> {
    let mut platforms: Vec<OpenClPlatform> = Vec::new();
    for line in out.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("Platform #") {
            let name = rest.split_once(':').map(|(_, n)| n.trim()).unwrap_or(rest).to_string();
            platforms.push(OpenClPlatform { name, devices: Vec::new() });
        } else if let Some(idx) = line.find("Device #") {
            if let (Some(p), Some((_, name))) = (platforms.last_mut(), line[idx..].split_once(':')) {
                p.devices.push(name.trim().to_string());
            }
        }
    }
    platforms
}

/// Blocs `GPU0:` de `vulkaninfo --summary`.
fn parse_vulkan_summary(out: &str) -> Vec<VulkanDevice> {
    let mut devs: Vec<VulkanDevice> = Vec::new();
    let mut in_devices = false;
    for line in out.lines() {
        let t = line.trim();
        if t.starts_with("Devices:") {
            in_devices = true;
            continue;
        }
        if !in_devices {
            continue;
        }
        if t.starts_with("GPU") && t.ends_with(':') {
            devs.push(VulkanDevice::default());
            continue;
        }
        let Some(d) = devs.last_mut() else { continue };
        let Some((k, v)) = t.split_once('=') else { continue };
        let v = v.trim().to_string();
        match k.trim() {
            "deviceName" => d.name = v,
            "apiVersion" => d.api_version = Some(v.split_whitespace().next().unwrap_or(&v).to_string()),
            "deviceType" => d.device_type = Some(v.trim_start_matches("PHYSICAL_DEVICE_TYPE_").to_lowercase()),
            "driverName" => d.driver = Some(v),
            _ => {}
        }
    }
    devs.retain(|d| !d.name.is_empty());
    devs
}

fn vaapi() -> Option<VaApi> {
    let out = run_timeout("vainfo", &["--display", "drm"], Duration::from_secs(10)).or_else(|| run_timeout("vainfo", &[], Duration::from_secs(10)))?;
    let text = format!("{}\n{}", out.stdout, out.stderr);
    let driver = text.lines().find_map(|l| l.split("Driver version:").nth(1).map(|s| s.trim().to_string()));
    let profiles: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let (p, e) = l.split_once(':')?;
            let p = p.trim();
            p.starts_with("VAProfile").then(|| format!("{} : {}", p.trim_start_matches("VAProfile"), e.trim().trim_start_matches("VAEntrypoint")))
        })
        .collect();
    if driver.is_none() && profiles.is_empty() {
        return None;
    }
    Some(VaApi { driver, profiles })
}

/// Codecs matériels réellement annoncés par VA-API.
pub fn vaapi_codecs(va: &VaApi) -> Vec<HwCodec> {
    let mut out: Vec<HwCodec> = Vec::new();
    for p in &va.profiles {
        let Some((profile, entry)) = p.split_once(" : ") else { continue };
        let codec = if profile.starts_with("H264") {
            "H.264"
        } else if profile.starts_with("HEVC") {
            "H.265/HEVC"
        } else if profile.starts_with("VP9") {
            "VP9"
        } else if profile.starts_with("VP8") {
            "VP8"
        } else if profile.starts_with("AV1") {
            "AV1"
        } else if profile.starts_with("JPEG") {
            "JPEG"
        } else if profile.starts_with("MPEG2") {
            "MPEG-2"
        } else if profile.starts_with("VC1") {
            "VC-1"
        } else {
            continue;
        };
        let direction = if entry.starts_with("VLD") {
            "decode"
        } else if entry.starts_with("Enc") {
            "encode"
        } else {
            continue;
        };
        if !out.iter().any(|c| c.codec == codec && c.direction == direction) {
            out.push(HwCodec { codec: codec.into(), direction: direction.into(), backend: "VA-API".into(), device: None });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clinfo() {
        let p = parse_clinfo_l("Platform #0: NVIDIA CUDA\n `-- Device #0: Quadro RTX 4000\nPlatform #1: rusticl\n");
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].devices, vec!["Quadro RTX 4000"]);
    }

    #[test]
    fn vulkan() {
        let s = "==========\nVULKANINFO\n==========\n\nDevices:\n========\nGPU0:\n\tapiVersion         = 1.3.277\n\tdeviceType         = PHYSICAL_DEVICE_TYPE_DISCRETE_GPU\n\tdeviceName         = Quadro RTX 4000\n\tdriverName         = NVIDIA\nGPU1:\n\tapiVersion         = 1.3.274\n\tdeviceType         = PHYSICAL_DEVICE_TYPE_CPU\n\tdeviceName         = llvmpipe (LLVM 17.0.6, 256 bits)\n\tdriverName         = llvmpipe\n";
        let d = parse_vulkan_summary(s);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].device_type.as_deref(), Some("discrete_gpu"));
        assert_eq!(d[1].device_type.as_deref(), Some("cpu"));
    }

    #[test]
    fn va() {
        let va = VaApi { driver: None, profiles: vec!["H264Main : VLD".into(), "H264Main : EncSliceLP".into(), "HEVCMain : VLD".into()] };
        let c = vaapi_codecs(&va);
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn versions() {
        assert_eq!(so_version("libcudnn.so.8.9.4").as_deref(), Some("8.9.4"));
        assert_eq!(after_label("| NVIDIA-SMI 550.54   Driver Version: 550.54.14   CUDA Version: 12.4 |", "CUDA Version:").as_deref(), Some("12.4"));
    }
}
