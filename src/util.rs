//! Utilitaires partagés par toutes les sondes : lecture de fichiers système,
//! exécution de commandes avec timeout, extraction de versions.

use regex::Regex;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, OnceLock};
use std::time::{Duration, Instant};

/// Délai par défaut pour une commande externe.
pub const CMD_TIMEOUT: Duration = Duration::from_secs(10);

/// Sortie d'une commande terminée.
pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

impl CmdOutput {
    /// stdout, ou stderr si stdout est vide (beaucoup d'outils écrivent leur version sur stderr).
    pub fn text(&self) -> &str {
        if self.stdout.trim().is_empty() { &self.stderr } else { &self.stdout }
    }
}

/// Exécute une commande avec un timeout. Renvoie `None` si le programme est
/// introuvable, ne démarre pas, ou dépasse le délai (il est alors tué).
pub fn run_timeout(program: &str, args: &[&str], timeout: Duration) -> Option<CmdOutput> {
    run_with(program, args, timeout, &[], None)
}

/// Variante avec variables d'environnement et dossier de travail.
pub fn run_with(program: &str, args: &[&str], timeout: Duration, envs: &[(&str, &str)], cwd: Option<&Path>) -> Option<CmdOutput> {
    let mut cmd = Command::new(program);
    cmd.args(args).env("LC_ALL", "C").env("LANG", "C").envs(envs.iter().copied());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    // Lecture dans des threads pour ne pas bloquer si la sortie remplit le pipe.
    let reader = |mut pipe: Box<dyn Read + Send>| {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            let _ = tx.send(buf);
        });
        rx
    };
    let rx_out = reader(Box::new(child.stdout.take()?));
    let rx_err = reader(Box::new(child.stderr.take()?));

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => break None,
        }
    };

    let status = status?;
    // Un sous-processus lancé en arrière-plan peut garder le pipe ouvert :
    // on n'attend pas la fin de sa sortie au-delà d'un court délai.
    let collect = |rx: mpsc::Receiver<Vec<u8>>| String::from_utf8_lossy(&rx.recv_timeout(Duration::from_secs(2)).unwrap_or_default()).into_owned();
    let stdout = collect(rx_out);
    let stderr = collect(rx_err);
    Some(CmdOutput { stdout, stderr, success: status.success() })
}

/// Exécute une commande avec le délai par défaut.
pub fn run(program: &str, args: &[&str]) -> Option<CmdOutput> {
    run_timeout(program, args, CMD_TIMEOUT)
}

/// stdout d'une commande qui a réussi, sinon `None`.
pub fn run_ok(program: &str, args: &[&str]) -> Option<String> {
    run(program, args).filter(|o| o.success).map(|o| o.stdout)
}

/// Chemin d'un exécutable dans le PATH, ou dans une liste de chemins connus hors PATH.
pub fn find_exe(name: &str) -> Option<PathBuf> {
    if let Ok(p) = which::which(name) {
        return Some(p);
    }
    const EXTRA: &[&str] = &["/sbin", "/usr/sbin", "/usr/local/sbin", "/usr/local/cuda/bin", "/opt/rocm/bin"];
    EXTRA.iter().map(|d| Path::new(d).join(name)).find(|p| p.is_file())
}

pub fn has_exe(name: &str) -> bool {
    find_exe(name).is_some()
}

/// Contenu d'un fichier, sans espaces ni octets NUL de fin (device-tree).
pub fn read_trim(path: impl AsRef<Path>) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let s = String::from_utf8_lossy(&bytes);
    let s = s.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    if s.is_empty() { None } else { Some(s.to_string()) }
}

pub fn read_u64(path: impl AsRef<Path>) -> Option<u64> {
    read_trim(path)?.parse().ok()
}

/// Liste de chaînes séparées par NUL (format des propriétés device-tree comme `compatible`).
pub fn read_nul_list(path: impl AsRef<Path>) -> Vec<String> {
    fs::read(path)
        .map(|b| {
            b.split(|&c| c == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Noms des entrées d'un dossier, triés (ordre naturel : video2 avant video10).
pub fn list_dir(path: impl AsRef<Path>) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(path)
        .map(|rd| rd.flatten().filter_map(|e| e.file_name().into_string().ok()).collect())
        .unwrap_or_default();
    names.sort_by(|a, b| natural_cmp(a, b));
    names
}

/// Noms du dossier commençant par `prefix`.
pub fn list_dir_prefix(path: impl AsRef<Path>, prefix: &str) -> Vec<String> {
    list_dir(path).into_iter().filter(|n| n.starts_with(prefix)).collect()
}

/// Nom de la cible d'un lien symbolique (ex. `device/driver` -> `panfrost`).
pub fn link_name(path: impl AsRef<Path>) -> Option<String> {
    fs::read_link(path).ok()?.file_name()?.to_str().map(String::from)
}

/// Comparaison « naturelle » : les suites de chiffres sont comparées numériquement.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn key(s: &str) -> Vec<(String, u64)> {
        let mut out = Vec::new();
        let mut text = String::new();
        let mut num = String::new();
        for c in s.chars() {
            if c.is_ascii_digit() {
                num.push(c);
            } else {
                if !num.is_empty() {
                    out.push((std::mem::take(&mut text), num.parse().unwrap_or(0)));
                    num.clear();
                }
                text.push(c);
            }
        }
        out.push((text, num.parse().unwrap_or(0)));
        out
    }
    key(a).cmp(&key(b))
}

/// Première version de la forme `1.2` ou `1.2.3` trouvée dans un texte.
pub fn extract_version(text: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(\d+\.\d+(?:\.\d+)?(?:[-+~][0-9A-Za-z.]+)?)").unwrap());
    re.captures(text).map(|c| c[1].to_string())
}

/// Version d'un outil : lance `cmd args` et extrait le numéro de version.
pub fn tool_version(cmd: &str, args: &[&str]) -> Option<String> {
    let out = run(cmd, args)?;
    extract_version(out.text())
}

/// Valeur d'une clé `CLE=valeur` (format os-release), guillemets retirés.
pub fn kv_value(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches('"').trim_matches('\'').to_string())
    })
}

/// Valeur d'une ligne `Clé : valeur` (format /proc/cpuinfo, /proc/meminfo).
pub fn colon_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let (k, v) = line.split_once(':')?;
    (k.trim() == key).then(|| v.trim())
}

pub fn bytes_to_gb(b: u64) -> f64 {
    b as f64 / 1_000_000_000.0
}

pub fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

pub fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Date/heure UTC au format ISO 8601, sans dépendance externe.
pub fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Algorithme « civil_from_days » de Howard Hinnant.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(extract_version("Python 3.12.3").as_deref(), Some("3.12.3"));
        assert_eq!(extract_version("go version go1.22.2 linux/arm64").as_deref(), Some("1.22.2"));
        assert_eq!(extract_version("openjdk version \"17.0.9\" 2023-10-17").as_deref(), Some("17.0.9"));
        assert_eq!(extract_version("Lua 5.4").as_deref(), Some("5.4"));
        assert_eq!(extract_version("no version"), None);
    }

    #[test]
    fn natural_sort() {
        let mut v = vec!["video10", "video2", "video1"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["video1", "video2", "video10"]);
    }

    #[test]
    fn run_missing_and_timeout() {
        assert!(run("commande-qui-n-existe-pas-42", &[]).is_none());
        let t = Instant::now();
        assert!(run_timeout("sleep", &["5"], Duration::from_millis(200)).is_none());
        assert!(t.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn background_child_does_not_block() {
        // Le shell se termine mais laisse `sleep` avec le pipe stdout ouvert.
        let t = Instant::now();
        let out = run("sh", &["-c", "echo ok; sleep 30 &"]).unwrap();
        assert!(out.success);
        assert!(t.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn iso_date() {
        let d = now_iso8601();
        assert_eq!(d.len(), 20);
        assert!(d.starts_with("20"));
    }
}
