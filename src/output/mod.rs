//! Sorties du rapport : terminal, JSON, Markdown, HTML autonome, prompt LLM.

pub mod html;
pub mod markdown;
pub mod prompt;
pub mod terminal;

use crate::report::Report;

pub fn to_json(r: &Report) -> String {
    serde_json::to_string_pretty(r).unwrap_or_else(|_| "{}".into())
}

/// Libellés communs aux différentes sorties.
pub fn gb(mb: u64) -> String {
    format!("{:.1} Go", mb as f64 / 1024.0)
}

pub fn opt(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("—")
}

pub fn uptime(s: u64) -> String {
    let (d, h, m) = (s / 86_400, s % 86_400 / 3600, s % 3600 / 60);
    if d > 0 {
        format!("{} j {} h {} min", d, h, m)
    } else if h > 0 {
        format!("{} h {} min", h, m)
    } else {
        format!("{} min", m)
    }
}

pub fn cluster_line(c: &crate::report::CpuCluster) -> String {
    let freq = match (c.min_mhz, c.max_mhz) {
        (Some(a), Some(b)) => format!(" @ {}–{} MHz", a, b),
        (None, Some(b)) => format!(" @ {} MHz max", b),
        _ => String::new(),
    };
    format!("{}× {}{} (cpu {})", c.cpus.len(), c.name, freq, compact_ranges(&c.cpus))
}

/// [0,1,2,3,6] -> « 0-3,6 »
pub fn compact_ranges(v: &[usize]) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < v.len() {
        let start = v[i];
        let mut end = start;
        while i + 1 < v.len() && v[i + 1] == end + 1 {
            i += 1;
            end = v[i];
        }
        out.push(if start == end { start.to_string() } else { format!("{}-{}", start, end) });
        i += 1;
    }
    out.join(",")
}

#[cfg(test)]
mod tests {
    #[test]
    fn ranges() {
        assert_eq!(super::compact_ranges(&[0, 1, 2, 3, 6]), "0-3,6");
        assert_eq!(super::compact_ranges(&[4]), "4");
    }
}
