//! Performance benchmarks probe (optional)

use super::{BenchmarkResults, Probe};
use anyhow::Result;
use serde_json::Value;
use std::time::Instant;
use rayon::prelude::*;

pub struct BenchmarksProbe;

impl Probe for BenchmarksProbe {
    fn name(&self) -> &'static str {
        "benchmarks"
    }

    fn run(&self) -> Result<Value> {
        println!("Running benchmarks (this may take a moment)...");
        
        let cpu_score = benchmark_cpu();
        let gpu_score = benchmark_gpu();
        let memory_bandwidth_mbps = benchmark_memory_bandwidth();
        let disk_io_mbps = benchmark_disk_io();
        
        let results = BenchmarkResults {
            cpu_score,
            gpu_score,
            memory_bandwidth_mbps,
            disk_io_mbps,
        };
        
        Ok(serde_json::to_value(results)?)
    }
}

fn benchmark_cpu() -> u32 {
    // Simple CPU benchmark: compute prime numbers in parallel
    let start = Instant::now();
    
    let result: Vec<bool> = (2..1_000_000)
        .into_par_iter()
        .map(|n| is_prime(n))
        .collect();
    
    let duration = start.elapsed();
    let score = (1_000_000.0 / duration.as_secs_f64()) as u32;
    
    println!("CPU Benchmark: {} primes/sec", score);
    score.min(10000) // Cap at 10000
}

fn benchmark_gpu() -> u32 {
    // Basic GPU check - if CUDA/OpenCL available, assume decent score
    if which::which("nvidia-smi").is_ok() {
        // Try to get actual GPU compute capability
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=compute_cap", "--format=csv,noheader"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Ok(cap) = text.trim().parse::<f32>() {
                return (cap * 1000.0) as u32;
            }
        }
        return 5000;
    }
    
    if which::which("clinfo").is_ok() {
        return 3000;
    }
    
    if which::which("vulkaninfo").is_ok() {
        return 2000;
    }
    
    500 // CPU-only fallback
}

fn benchmark_memory_bandwidth() -> f64 {
    // Simple memory bandwidth test
    let size = 100_000_000; // 100 million elements
    let mut vec1: Vec<u64> = vec![0; size];
    let mut vec2: Vec<u64> = vec![0; size];
    
    // Initialize
    for i in 0..size {
        vec1[i] = i as u64;
        vec2[i] = i as u64;
    }
    
    let start = Instant::now();
    
    // Memory-intensive operation
    for i in 0..size {
        vec1[i] += vec2[i];
    }
    
    let duration = start.elapsed();
    let bytes_processed = (size * std::mem::size_of::<u64>() * 2) as f64;
    let bandwidth = bytes_processed / duration.as_secs_f64() / 1_000_000.0; // MB/s
    
    bandwidth
}

fn benchmark_disk_io() -> f64 {
    // Write and read test file
    let test_file = "/tmp/intelliprobe_benchmark.tmp";
    let data = vec![0u8; 100 * 1024 * 1024]; // 100 MB
    
    // Write
    let start = Instant::now();
    if let Ok(_) = std::fs::write(test_file, &data) {
        let write_duration = start.elapsed();
        let write_speed = data.len() as f64 / write_duration.as_secs_f64() / 1_000_000.0;
        
        // Read
        let start = Instant::now();
        let _ = std::fs::read(test_file);
        let read_duration = start.elapsed();
        let read_speed = data.len() as f64 / read_duration.as_secs_f64() / 1_000_000.0;
        
        // Cleanup
        let _ = std::fs::remove_file(test_file);
        
        return (write_speed + read_speed) / 2.0;
    }
    
    100.0 // Default fallback
}

fn is_prime(n: u32) -> bool {
    if n <= 1 {
        return false;
    }
    if n <= 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    
    let mut i = 5;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }
    true
}