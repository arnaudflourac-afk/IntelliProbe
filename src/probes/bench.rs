//! Micro-benchmarks : mesures réelles de la machine (et non des estimations).
//! - CPU : FP32 multiplication-addition vectorisable, 1 cœur puis tous les cœurs
//! - Mémoire : lecture séquentielle de gros tampons (débit qui borne l'inférence LLM)
//! - Disque : écriture synchronisée puis relecture hors cache

use crate::report::{Bench, DiskBench};
use crate::util::round1;
use std::hint::black_box;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Barrier;
use std::time::{Duration, Instant};

const LANES: usize = 32;
const TARGET: Duration = Duration::from_millis(400);

pub fn run(disk_dir: &Path, available_mb: u64) -> Bench {
    let start = Instant::now();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);

    let iters = calibrate_cpu();
    let single = cpu_gflops(iters, 1);
    let multi = cpu_gflops(iters, threads);

    // Tampon bien plus gros que les caches : 256 Mo max, jamais plus d'1/8 de la RAM disponible.
    let buf_mb = (available_mb / 8).clamp(16, 256);
    let mem_single = mem_read_gbs(buf_mb as usize * 1024 * 1024, 1);
    let mem_multi = mem_read_gbs(buf_mb as usize * 1024 * 1024, threads);

    Bench {
        threads,
        cpu_single_gflops: round1(single),
        cpu_multi_gflops: round1(multi),
        mem_buffer_mb: buf_mb,
        mem_read_single_gbs: round1(mem_single),
        mem_read_multi_gbs: round1(mem_multi),
        disk: disk_bench(disk_dir),
        duration_s: round1(start.elapsed().as_secs_f64()),
    }
}

/// Noyau : LANES chaînes indépendantes de `v = v * a + b` (2 flops chacune), vectorisable.
fn cpu_kernel(iters: u64) -> f32 {
    let mut acc = [0f32; LANES];
    for (i, v) in acc.iter_mut().enumerate() {
        *v = i as f32 * 1e-3;
    }
    let a = black_box(0.999_f32);
    let b = black_box(0.001_f32);
    for _ in 0..iters {
        for v in acc.iter_mut() {
            *v = *v * a + b;
        }
    }
    black_box(acc).iter().sum()
}

fn calibrate_cpu() -> u64 {
    let mut iters = 1u64 << 16;
    loop {
        let t = Instant::now();
        black_box(cpu_kernel(iters));
        let el = t.elapsed();
        if el > Duration::from_millis(50) || iters > 1 << 34 {
            let scale = TARGET.as_secs_f64() / el.as_secs_f64().max(1e-6);
            return ((iters as f64 * scale) as u64).max(1);
        }
        iters *= 4;
    }
}

fn cpu_gflops(iters: u64, threads: usize) -> f64 {
    let barrier = Barrier::new(threads + 1);
    let elapsed = std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                barrier.wait();
                black_box(cpu_kernel(iters));
            });
        }
        barrier.wait();
        let t = Instant::now();
        // La portée attend la fin de tous les threads.
        t
    });
    let secs = elapsed.elapsed().as_secs_f64();
    (iters as f64 * LANES as f64 * 2.0 * threads as f64) / secs / 1e9
}

/// Somme de mots de 64 bits avec 8 accumulateurs indépendants (vectorisable) :
/// le processeur doit lire chaque octet, comme un LLM lit tous ses poids à chaque token.
fn read_sum(data: &[u64]) -> u64 {
    let mut acc = [0u64; 8];
    for c in data.chunks_exact(8) {
        for i in 0..8 {
            acc[i] = acc[i].wrapping_add(c[i]);
        }
    }
    acc.iter().fold(0, |a, &b| a.wrapping_add(b))
}

fn mem_read_gbs(bytes: usize, threads: usize) -> f64 {
    let data: Vec<u64> = (0..bytes / 8).map(|i| i as u64).collect();
    // Nombre de passes pour ~TARGET, estimé sur une passe mono-thread.
    let t = Instant::now();
    black_box(read_sum(&data));
    let one = t.elapsed().as_secs_f64().max(1e-6);
    let passes = ((TARGET.as_secs_f64() * threads.min(4) as f64 / one) as usize).clamp(2, 400);

    let chunk = data.len().div_ceil(threads).div_ceil(8) * 8;
    let parts: Vec<&[u64]> = data.chunks(chunk).collect();
    let barrier = Barrier::new(parts.len() + 1);
    let start = std::thread::scope(|s| {
        for part in &parts {
            let barrier = &barrier;
            s.spawn(move || {
                barrier.wait();
                for _ in 0..passes {
                    black_box(read_sum(black_box(part)));
                }
            });
        }
        barrier.wait();
        Instant::now()
    });
    let secs = start.elapsed().as_secs_f64();
    (data.len() * 8) as f64 * passes as f64 / secs / 1e9
}

fn disk_bench(dir: &Path) -> Option<DiskBench> {
    let (_, free) = super::storage::statvfs(&dir.to_string_lossy())?;
    // 256 Mo max, jamais plus de 5 % de l'espace libre.
    let size_mb = (free / 1024 / 1024 / 20).min(256);
    if size_mb < 16 {
        return None;
    }
    let path = dir.join(".intelliprobe_disk_test.bin");
    let result = (|| -> std::io::Result<DiskBench> {
        // Données pseudo-aléatoires : les systèmes de fichiers compressants ne trichent pas.
        let mut block = vec![0u8; 1024 * 1024];
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for b in block.iter_mut() {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *b = x as u8;
        }
        let t = Instant::now();
        let mut f = std::fs::File::create(&path)?;
        for _ in 0..size_mb {
            f.write_all(&block)?;
        }
        f.sync_all()?;
        let write_s = t.elapsed().as_secs_f64();
        let read_mbs = drop_cache(&f).then(|| -> std::io::Result<f64> {
            let t = Instant::now();
            let mut f = std::fs::File::open(&path)?;
            let mut n = 0usize;
            loop {
                let r = f.read(&mut block)?;
                if r == 0 {
                    break;
                }
                n += r;
            }
            Ok(n as f64 / 1024.0 / 1024.0 / t.elapsed().as_secs_f64())
        });
        Ok(DiskBench {
            path: dir.to_string_lossy().into_owned(),
            size_mb,
            write_mbs: round1(size_mb as f64 / write_s),
            read_mbs: read_mbs.and_then(|r| r.ok()).map(round1),
        })
    })();
    let _ = std::fs::remove_file(&path);
    result.ok()
}

/// Retire le fichier du cache de pages (sans root) pour que la relecture touche le support.
#[cfg(target_os = "linux")]
fn drop_cache(f: &std::fs::File) -> bool {
    use std::os::unix::io::AsRawFd;
    unsafe { libc::posix_fadvise(f.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) == 0 }
}

#[cfg(not(target_os = "linux"))]
fn drop_cache(_f: &std::fs::File) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_bench_positive() {
        let g = cpu_gflops(1 << 16, 1);
        assert!(g > 0.0);
    }

    #[test]
    fn mem_bench_positive() {
        assert!(mem_read_gbs(4 * 1024 * 1024, 2) > 0.0);
    }
}
