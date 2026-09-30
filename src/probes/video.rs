//! Vidéo : périphériques V4L2 (caméras, codecs mem2mem) interrogés par ioctl,
//! nœuds constructeur (MPP, NVENC…), ffmpeg, GStreamer, libcamera.

use crate::report::{Ffmpeg, Gstreamer, HwCodec, V4l2Device, Video};
use crate::util::*;
use std::path::Path;
use std::time::Duration;

pub fn probe() -> Video {
    let mut v = Video::default();
    for d in list_dir_prefix("/dev", "video") {
        if d.len() <= 5 || !d[5..].chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let path = format!("/dev/{}", d);
        let mut dev = V4l2Device {
            name: read_trim(format!("/sys/class/video4linux/{}/name", d)).unwrap_or_default(),
            path: path.clone(),
            ..Default::default()
        };
        query_v4l2(&mut dev);
        v.hw_codecs.extend(codecs_of(&dev));
        v.v4l2_devices.push(dev);
    }

    const VENDOR_NODES: &[(&str, &str)] = &[
        ("mpp_service", "Rockchip MPP (codecs matériels)"),
        ("rga", "Rockchip RGA (accélération 2D)"),
        ("vpu_service", "Rockchip VPU (ancien)"),
        ("hevc_service", "Rockchip HEVC (ancien)"),
        ("nvhost-msenc", "NVIDIA Jetson NVENC"),
        ("nvhost-nvenc1", "NVIDIA Jetson NVENC (2e)"),
        ("nvhost-nvdec", "NVIDIA Jetson NVDEC"),
        ("nvhost-nvjpg", "NVIDIA Jetson NVJPG"),
        ("nvhost-vic", "NVIDIA Jetson VIC (conversion d'images)"),
        ("vchiq", "Raspberry Pi VideoCore (VCHIQ)"),
        ("cedar_dev", "Allwinner Cedar (codecs)"),
        ("mxc_hantro", "NXP Hantro VPU"),
        ("mxc_hantro_vc8000e", "NXP Hantro VC8000E (encodeur)"),
        ("amvideo", "Amlogic vidéo"),
        ("dma_heap", "DMA-BUF heaps (zéro-copie)"),
    ];
    for (node, label) in VENDOR_NODES {
        if Path::new(&format!("/dev/{}", node)).exists() {
            v.vendor_nodes.push(format!("/dev/{} : {}", node, label));
        }
    }

    v.ffmpeg = ffmpeg();
    for e in v.ffmpeg.iter().flat_map(|f| f.working_encoders.iter()) {
        if let Some(codec) = ffmpeg_codec(e) {
            v.hw_codecs.push(HwCodec { codec: codec.into(), direction: "encode".into(), backend: format!("FFmpeg {} (encodage testé avec succès)", e), device: None });
        }
    }
    v.gstreamer = gstreamer();
    v.libcamera = has_exe("rpicam-hello") || has_exe("libcamera-hello") || has_exe("cam") || super::libs::has_lib("libcamera.so");
    v
}

// V4L2 : constantes de videodev2.h
const CAP_VIDEO_CAPTURE: u32 = 0x0000_0001;
const CAP_VIDEO_OUTPUT: u32 = 0x0000_0002;
const CAP_VIDEO_CAPTURE_MPLANE: u32 = 0x0000_1000;
const CAP_VIDEO_OUTPUT_MPLANE: u32 = 0x0000_2000;
const CAP_VIDEO_M2M_MPLANE: u32 = 0x0000_4000;
const CAP_VIDEO_M2M: u32 = 0x0000_8000;
const CAP_META_CAPTURE: u32 = 0x0080_0000;
const CAP_DEVICE_CAPS: u32 = 0x8000_0000;

#[cfg(target_os = "linux")]
fn query_v4l2(dev: &mut V4l2Device) {
    #[repr(C)]
    struct Capability {
        driver: [u8; 16],
        card: [u8; 32],
        bus_info: [u8; 32],
        version: u32,
        capabilities: u32,
        device_caps: u32,
        reserved: [u32; 3],
    }
    #[repr(C)]
    struct FmtDesc {
        index: u32,
        type_: u32,
        flags: u32,
        description: [u8; 32],
        pixelformat: u32,
        mbus_code: u32,
        reserved: [u32; 3],
    }
    const VIDIOC_QUERYCAP: u32 = 0x8068_5600;
    const VIDIOC_ENUM_FMT: u32 = 0xC040_5602;

    let Ok(c) = std::ffi::CString::new(dev.path.clone()) else { return };
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK | libc::O_CLOEXEC) };
    if fd < 0 {
        let err = std::io::Error::last_os_error();
        dev.error = Some(if err.raw_os_error() == Some(libc::EACCES) {
            "accès refusé (ajouter l'utilisateur au groupe « video »)".into()
        } else {
            err.to_string()
        });
        return;
    }
    let mut cap: Capability = unsafe { std::mem::zeroed() };
    if unsafe { libc::ioctl(fd, VIDIOC_QUERYCAP as _, &mut cap) } < 0 {
        dev.error = Some(std::io::Error::last_os_error().to_string());
        unsafe { libc::close(fd) };
        return;
    }
    let caps = if cap.capabilities & CAP_DEVICE_CAPS != 0 { cap.device_caps } else { cap.capabilities };
    dev.driver = Some(super::interfaces::cstr(&cap.driver));
    dev.bus = Some(super::interfaces::cstr(&cap.bus_info)).filter(|s| !s.is_empty());
    if dev.name.is_empty() {
        dev.name = super::interfaces::cstr(&cap.card);
    }
    dev.roles = roles(caps);

    let enum_fmts = |buf_type: u32| -> Vec<String> {
        let mut out = Vec::new();
        for index in 0..64 {
            let mut f: FmtDesc = unsafe { std::mem::zeroed() };
            f.index = index;
            f.type_ = buf_type;
            if unsafe { libc::ioctl(fd, VIDIOC_ENUM_FMT as _, &mut f) } < 0 {
                break;
            }
            out.push(fourcc(f.pixelformat));
        }
        out
    };
    if caps & (CAP_VIDEO_CAPTURE | CAP_VIDEO_M2M) != 0 {
        dev.capture_formats.extend(enum_fmts(1));
    }
    if caps & (CAP_VIDEO_CAPTURE_MPLANE | CAP_VIDEO_M2M_MPLANE) != 0 {
        dev.capture_formats.extend(enum_fmts(9));
    }
    if caps & (CAP_VIDEO_OUTPUT | CAP_VIDEO_M2M) != 0 {
        dev.output_formats.extend(enum_fmts(2));
    }
    if caps & (CAP_VIDEO_OUTPUT_MPLANE | CAP_VIDEO_M2M_MPLANE) != 0 {
        dev.output_formats.extend(enum_fmts(10));
    }
    dev.capture_formats.dedup();
    dev.output_formats.dedup();
    unsafe { libc::close(fd) };
}

#[cfg(not(target_os = "linux"))]
fn query_v4l2(_dev: &mut V4l2Device) {}

fn roles(caps: u32) -> Vec<String> {
    let mut r = Vec::new();
    if caps & (CAP_VIDEO_M2M | CAP_VIDEO_M2M_MPLANE) != 0 {
        r.push("mem2mem".to_string());
    } else {
        if caps & (CAP_VIDEO_CAPTURE | CAP_VIDEO_CAPTURE_MPLANE) != 0 {
            r.push("capture".into());
        }
        if caps & (CAP_VIDEO_OUTPUT | CAP_VIDEO_OUTPUT_MPLANE) != 0 {
            r.push("sortie".into());
        }
    }
    if caps & CAP_META_CAPTURE != 0 {
        r.push("métadonnées".into());
    }
    r
}

fn fourcc(v: u32) -> String {
    v.to_le_bytes().iter().map(|&b| if b.is_ascii_graphic() { b as char } else { ' ' }).collect::<String>().trim().to_string()
}

/// Nom du codec pour un format compressé V4L2 (et s'il s'agit d'une API stateless).
fn compressed_codec(fourcc: &str) -> Option<(&'static str, bool)> {
    Some(match fourcc {
        "H264" | "AVC1" => ("H.264", false),
        "S264" => ("H.264", true),
        "HEVC" | "H265" => ("H.265/HEVC", false),
        "S265" => ("H.265/HEVC", true),
        "VP80" => ("VP8", false),
        "VP8F" => ("VP8", true),
        "VP90" => ("VP9", false),
        "VP9F" => ("VP9", true),
        "AV01" => ("AV1", false),
        "AV1F" => ("AV1", true),
        "MJPG" | "JPEG" => ("JPEG/MJPEG", false),
        "MPG2" => ("MPEG-2", false),
        "MG2S" => ("MPEG-2", true),
        "MPG4" => ("MPEG-4", false),
        "H263" => ("H.263", false),
        "VC1G" | "VC1L" => ("VC-1", false),
        _ => return None,
    })
}

/// Un périphérique mem2mem qui consomme du compressé décode ; qui en produit encode.
fn codecs_of(dev: &V4l2Device) -> Vec<HwCodec> {
    if !dev.roles.iter().any(|r| r == "mem2mem") {
        return Vec::new();
    }
    let mut out: Vec<HwCodec> = Vec::new();
    let mut add = |fmts: &[String], direction: &str| {
        for f in fmts {
            if let Some((codec, stateless)) = compressed_codec(f) {
                let backend = format!("V4L2 {} ({})", if stateless { "stateless" } else { "M2M" }, dev.driver.as_deref().unwrap_or("?"));
                if !out.iter().any(|c| c.codec == codec && c.direction == direction) {
                    out.push(HwCodec { codec: codec.into(), direction: direction.into(), backend, device: Some(dev.path.clone()) });
                }
            }
        }
    };
    add(&dev.output_formats, "decode");
    add(&dev.capture_formats, "encode");
    out
}

const HW_SUFFIXES: &[&str] = &["_nvenc", "_cuvid", "_vaapi", "_qsv", "_v4l2m2m", "_rkmpp", "_amf", "_videotoolbox", "_omx", "_mediacodec", "_vulkan", "_mf", "_nvmpi"];

fn ffmpeg() -> Option<Ffmpeg> {
    let version = run_ok("ffmpeg", &["-hide_banner", "-version"])?.lines().next().and_then(extract_version);
    let hwaccels = run_ok("ffmpeg", &["-hide_banner", "-hwaccels"])
        .map(|o| o.lines().skip_while(|l| !l.contains("Hardware acceleration methods")).skip(1).map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default();
    let list = |arg: &str| -> Vec<String> {
        run_ok("ffmpeg", &["-hide_banner", arg])
            .map(|o| {
                o.lines()
                    .filter_map(|l| {
                        let mut it = l.split_whitespace();
                        let _flags = it.next()?;
                        let name = it.next()?;
                        HW_SUFFIXES.iter().any(|s| name.ends_with(s)).then(|| name.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let hw_encoders = list("-encoders");
    let working_encoders = test_encoders(&hw_encoders);
    Some(Ffmpeg { version, hwaccels, hw_encoders, hw_decoders: list("-decoders"), working_encoders })
}

/// Encodeur compilé dans ffmpeg dont le matériel semble présent (évite des tests inutiles).
fn encoder_plausible(e: &str) -> bool {
    if e.ends_with("_nvenc") {
        Path::new("/dev/nvidia0").exists()
    } else if e.ends_with("_vaapi") {
        !list_dir_prefix("/dev/dri", "renderD").is_empty()
    } else if e.ends_with("_v4l2m2m") {
        !list_dir_prefix("/dev", "video").is_empty()
    } else if e.ends_with("_rkmpp") {
        Path::new("/dev/mpp_service").exists()
    } else if e.ends_with("_amf") {
        super::libs::has_lib("libamfrt64.so")
    } else {
        false
    }
}

/// Encode quelques images de test avec chaque encodeur plausible : seul un succès compte.
fn test_encoders(encoders: &[String]) -> Vec<String> {
    let candidates: Vec<&String> = encoders.iter().filter(|e| encoder_plausible(e)).collect();
    let render = list_dir_prefix("/dev/dri", "renderD").into_iter().next().map(|r| format!("/dev/dri/{}", r));
    std::thread::scope(|s| {
        let handles: Vec<_> = candidates
            .iter()
            .map(|e| {
                let render = render.clone();
                s.spawn(move || {
                    let mut args: Vec<String> = ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=320x240:rate=10"].iter().map(|x| x.to_string()).collect();
                    if e.ends_with("_vaapi") {
                        args.extend(["-vaapi_device".into(), render.unwrap_or_default(), "-vf".into(), "format=nv12,hwupload".into()]);
                    } else if !e.ends_with("_nvenc") {
                        args.extend(["-pix_fmt".into(), "yuv420p".into()]);
                    }
                    args.extend(["-c:v", e.as_str(), "-frames:v", "5", "-f", "null", "-"].iter().map(|x| x.to_string()));
                    let refs: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
                    run_timeout("ffmpeg", &refs, Duration::from_secs(20)).is_some_and(|o| o.success).then(|| e.to_string())
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok().flatten()).collect()
    })
}

/// Codec d'un encodeur ffmpeg (`hevc_nvenc` -> `H.265/HEVC`).
fn ffmpeg_codec(encoder: &str) -> Option<&'static str> {
    Some(match encoder.split('_').next()? {
        "h264" => "H.264",
        "hevc" => "H.265/HEVC",
        "av1" => "AV1",
        "vp9" => "VP9",
        "vp8" => "VP8",
        "mjpeg" => "JPEG/MJPEG",
        "mpeg2" => "MPEG-2",
        "mpeg4" => "MPEG-4",
        _ => return None,
    })
}

fn gstreamer() -> Option<Gstreamer> {
    let version = run_ok("gst-inspect-1.0", &["--version"])?.lines().next().and_then(extract_version);
    const PATTERNS: &[&str] = &["v4l2", "mpp", "nvv4l2", "nvh26", "nvav1", "nvjpeg", "nvvidconv", "nvarguscamera", "vaapi", "qsv", "msdk", "omx", "libcamera", "rga", "nvdec", "nvenc", "vulkan", "hailo", "rknn", "tensor_filter", "edgetpu"];
    let hw_elements = run_timeout("gst-inspect-1.0", &[], Duration::from_secs(30))
        .map(|o| {
            let mut v: Vec<String> = o
                .stdout
                .lines()
                .filter_map(|l| {
                    let mut parts = l.splitn(3, ':');
                    let _plugin = parts.next()?;
                    let element = parts.next()?.trim();
                    let is_va = element.starts_with("va") && ["h26", "vp8", "vp9", "av1", "jpeg", "mpeg2"].iter().any(|c| element[2..].starts_with(c));
                    (is_va || PATTERNS.iter().any(|p| element.contains(p))).then(|| element.to_string())
                })
                .collect();
            v.sort();
            v.dedup();
            v
        })
        .unwrap_or_default();
    Some(Gstreamer { version, hw_elements })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fourccs() {
        assert_eq!(fourcc(u32::from_le_bytes(*b"H264")), "H264");
        assert_eq!(compressed_codec("HEVC").unwrap().0, "H.265/HEVC");
        assert!(compressed_codec("NV12").is_none());
    }

    #[test]
    fn m2m_decoder() {
        let dev = V4l2Device {
            path: "/dev/video19".into(),
            roles: vec!["mem2mem".into()],
            driver: Some("rpi-hevc-dec".into()),
            output_formats: vec!["S265".into()],
            capture_formats: vec!["NC12".into()],
            ..Default::default()
        };
        let c = codecs_of(&dev);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].direction, "decode");
        assert_eq!(c[0].codec, "H.265/HEVC");
    }

    #[test]
    fn capture_is_not_codec() {
        let dev = V4l2Device { roles: vec!["capture".into()], capture_formats: vec!["MJPG".into()], ..Default::default() };
        assert!(codecs_of(&dev).is_empty());
    }
}
