//! Environnement de développement : langages, compilateurs (y compris croisés),
//! outils de build, embarqué, conteneurs, bases de données, éditeurs, supervision.

use crate::report::{DevEnv, Tool};
use crate::util::*;
use regex::Regex;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::OnceLock;

/// Définition d'un outil à rechercher.
pub struct ToolDef {
    pub category: &'static str,
    pub name: &'static str,
    /// Commandes candidates (la première trouvée est utilisée).
    pub commands: &'static [&'static str],
    /// Arguments pour obtenir la version ; `None` pour ne rien exécuter (IDE graphiques…).
    pub version_args: Option<&'static [&'static str]>,
}

const V: Option<&[&str]> = Some(&["--version"]);
const NONE: Option<&[&str]> = None;

macro_rules! t {
    ($cat:expr, $name:expr, [$($c:expr),+], $v:expr) => {
        ToolDef { category: $cat, name: $name, commands: &[$($c),+], version_args: $v }
    };
}

const LANG: &str = "Langages & compilateurs";
const PKG: &str = "Gestionnaires de paquets";
const BUILD: &str = "Build";
const DEBUG: &str = "Débogage & profilage";
const EMB: &str = "Embarqué & matériel";
const CONT: &str = "Conteneurs & virtualisation";
const VCS: &str = "Versionnement";
const DB: &str = "Bases de données";
const EDIT: &str = "Éditeurs & IDE";
const MON: &str = "Supervision";

pub const TOOLS: &[ToolDef] = &[
    t!(LANG, "GCC (C)", ["gcc"], V),
    t!(LANG, "G++ (C++)", ["g++"], V),
    t!(LANG, "Clang", ["clang"], V),
    t!(LANG, "Python", ["python3", "python"], V),
    t!(LANG, "Node.js", ["node"], V),
    t!(LANG, "Deno", ["deno"], V),
    t!(LANG, "Bun", ["bun"], V),
    t!(LANG, "Rust", ["rustc"], V),
    t!(LANG, "Go", ["go"], Some(&["version"])),
    t!(LANG, "Java", ["java"], Some(&["-version"])),
    t!(LANG, "Kotlin", ["kotlinc", "kotlin"], Some(&["-version"])),
    t!(LANG, ".NET", ["dotnet"], V),
    t!(LANG, "Ruby", ["ruby"], V),
    t!(LANG, "PHP", ["php"], V),
    t!(LANG, "Perl", ["perl"], V),
    t!(LANG, "Lua", ["lua", "lua5.4", "lua5.3", "luajit"], Some(&["-v"])),
    t!(LANG, "Julia", ["julia"], V),
    t!(LANG, "R", ["R", "Rscript"], V),
    t!(LANG, "Zig", ["zig"], Some(&["version"])),
    t!(LANG, "Swift", ["swift"], V),
    t!(LANG, "Dart", ["dart"], V),
    t!(LANG, "Nim", ["nim"], V),
    t!(LANG, "Fortran", ["gfortran"], V),
    t!(LANG, "MicroPython", ["micropython"], Some(&["-c", "import sys; print(sys.version)"])),
    t!(PKG, "apt", ["apt"], V),
    t!(PKG, "dnf", ["dnf"], V),
    t!(PKG, "pacman", ["pacman"], V),
    t!(PKG, "apk", ["apk"], V),
    t!(PKG, "opkg", ["opkg"], V),
    t!(PKG, "pip", ["pip3", "pip"], V),
    t!(PKG, "uv", ["uv"], V),
    t!(PKG, "pipx", ["pipx"], V),
    t!(PKG, "Poetry", ["poetry"], V),
    t!(PKG, "Conda", ["conda", "mamba", "micromamba"], V),
    t!(PKG, "npm", ["npm"], V),
    t!(PKG, "pnpm", ["pnpm"], V),
    t!(PKG, "Yarn", ["yarn"], V),
    t!(PKG, "Cargo", ["cargo"], V),
    t!(PKG, "rustup", ["rustup"], V),
    t!(PKG, "Snap", ["snap"], V),
    t!(PKG, "Flatpak", ["flatpak"], V),
    t!(PKG, "Nix", ["nix"], V),
    t!(PKG, "Homebrew", ["brew"], V),
    t!(PKG, "vcpkg", ["vcpkg"], Some(&["version"])),
    t!(PKG, "Conan", ["conan"], V),
    t!(BUILD, "Make", ["make"], V),
    t!(BUILD, "CMake", ["cmake"], V),
    t!(BUILD, "Ninja", ["ninja"], V),
    t!(BUILD, "Meson", ["meson"], V),
    t!(BUILD, "Bazel", ["bazel", "bazelisk"], V),
    t!(BUILD, "SCons", ["scons"], V),
    t!(BUILD, "Autoconf", ["autoconf"], V),
    t!(BUILD, "pkg-config", ["pkg-config", "pkgconf"], V),
    t!(BUILD, "Gradle", ["gradle"], V),
    t!(BUILD, "Maven", ["mvn"], V),
    t!(BUILD, "just", ["just"], V),
    t!(BUILD, "ccache", ["ccache"], V),
    t!(BUILD, "mold", ["mold"], V),
    t!(DEBUG, "GDB", ["gdb"], V),
    t!(DEBUG, "LLDB", ["lldb"], V),
    t!(DEBUG, "Valgrind", ["valgrind"], V),
    t!(DEBUG, "perf", ["perf"], V),
    t!(DEBUG, "strace", ["strace"], Some(&["-V"])),
    t!(DEBUG, "ltrace", ["ltrace"], V),
    t!(EMB, "Device Tree Compiler", ["dtc"], V),
    t!(EMB, "dtoverlay (Raspberry Pi)", ["dtoverlay"], NONE),
    t!(EMB, "i2c-tools", ["i2cdetect"], Some(&["-V"])),
    t!(EMB, "libgpiod (gpiodetect)", ["gpiodetect"], V),
    t!(EMB, "pinctrl / raspi-gpio", ["pinctrl", "raspi-gpio"], NONE),
    t!(EMB, "v4l-utils", ["v4l2-ctl"], V),
    t!(EMB, "media-ctl", ["media-ctl"], V),
    t!(EMB, "PlatformIO", ["pio", "platformio"], V),
    t!(EMB, "Arduino CLI", ["arduino-cli"], Some(&["version"])),
    t!(EMB, "ESP-IDF", ["idf.py"], V),
    t!(EMB, "esptool", ["esptool.py", "esptool"], Some(&["version"])),
    t!(EMB, "Zephyr west", ["west"], V),
    t!(EMB, "OpenOCD", ["openocd"], V),
    t!(EMB, "avrdude", ["avrdude"], Some(&["-?"])),
    t!(EMB, "dfu-util", ["dfu-util"], V),
    t!(EMB, "stlink", ["st-flash"], V),
    t!(EMB, "U-Boot tools", ["mkimage"], Some(&["-V"])),
    t!(EMB, "fastboot", ["fastboot"], V),
    t!(EMB, "adb", ["adb"], Some(&["version"])),
    t!(EMB, "rkdeveloptool", ["rkdeveloptool"], Some(&["-v"])),
    t!(EMB, "picocom", ["picocom"], Some(&["--help"])),
    t!(EMB, "minicom", ["minicom"], V),
    t!(EMB, "screen", ["screen"], Some(&["-v"])),
    t!(EMB, "armbian-config", ["armbian-config"], NONE),
    t!(EMB, "rsetup (Radxa)", ["rsetup"], NONE),
    t!(EMB, "raspi-config", ["raspi-config"], NONE),
    t!(EMB, "vcgencmd", ["vcgencmd"], Some(&["version"])),
    t!(CONT, "Docker", ["docker"], V),
    t!(CONT, "Docker Compose (v2)", ["docker"], Some(&["compose", "version"])),
    t!(CONT, "docker-compose (v1)", ["docker-compose"], V),
    t!(CONT, "Docker Buildx", ["docker"], Some(&["buildx", "version"])),
    t!(CONT, "Podman", ["podman"], V),
    t!(CONT, "nerdctl", ["nerdctl"], V),
    t!(CONT, "kubectl", ["kubectl"], Some(&["version", "--client"])),
    t!(CONT, "k3s", ["k3s"], V),
    t!(CONT, "NVIDIA Container Toolkit", ["nvidia-ctk"], V),
    t!(CONT, "QEMU", ["qemu-system-x86_64", "qemu-system-aarch64"], V),
    // Sur Ubuntu, `lxc` peut être un installeur de snap : on ne l'exécute pas.
    t!(CONT, "LXD / Incus", ["lxc", "incus"], NONE),
    t!(VCS, "Git", ["git"], V),
    t!(VCS, "Git LFS", ["git-lfs"], V),
    t!(VCS, "GitHub CLI", ["gh"], V),
    t!(VCS, "Mercurial", ["hg"], V),
    t!(VCS, "Subversion", ["svn"], V),
    t!(DB, "SQLite", ["sqlite3"], V),
    t!(DB, "PostgreSQL (client)", ["psql"], V),
    t!(DB, "PostgreSQL (serveur)", ["postgres", "pg_ctl"], V),
    t!(DB, "MySQL / MariaDB", ["mariadb", "mysql"], V),
    t!(DB, "Redis", ["redis-server", "redis-cli"], V),
    t!(DB, "MongoDB shell", ["mongosh"], V),
    t!(DB, "DuckDB", ["duckdb"], V),
    t!(DB, "InfluxDB", ["influx", "influxd"], V),
    t!(DB, "Mosquitto (MQTT)", ["mosquitto"], Some(&["-h"])),
    t!(EDIT, "VS Code", ["code"], V),
    t!(EDIT, "VSCodium", ["codium"], V),
    t!(EDIT, "Neovim", ["nvim"], V),
    t!(EDIT, "Vim", ["vim"], V),
    t!(EDIT, "Emacs", ["emacs"], V),
    t!(EDIT, "Nano", ["nano"], V),
    t!(EDIT, "Helix", ["hx"], V),
    t!(EDIT, "Micro", ["micro"], Some(&["-version"])),
    t!(EDIT, "Geany", ["geany"], NONE),
    t!(EDIT, "Zed", ["zed", "zeditor"], NONE),
    t!(EDIT, "Sublime Text", ["subl"], NONE),
    t!(EDIT, "IntelliJ IDEA", ["idea", "intellij-idea-community", "intellij-idea-ultimate"], NONE),
    t!(EDIT, "PyCharm", ["pycharm", "pycharm-community"], NONE),
    t!(EDIT, "CLion", ["clion"], NONE),
    t!(EDIT, "RustRover", ["rustrover"], NONE),
    t!(EDIT, "Thonny", ["thonny"], NONE),
    t!(EDIT, "Jupyter", ["jupyter"], V),
    t!(MON, "htop", ["htop"], V),
    t!(MON, "btop", ["btop"], V),
    t!(MON, "glances", ["glances"], V),
    t!(MON, "nvtop", ["nvtop"], V),
    t!(MON, "iotop", ["iotop"], V),
    t!(MON, "tegrastats (Jetson)", ["tegrastats"], NONE),
    t!(MON, "jtop (Jetson)", ["jtop"], NONE),
    t!(MON, "intel_gpu_top", ["intel_gpu_top"], NONE),
    t!(MON, "radeontop", ["radeontop"], NONE),
    t!(MON, "Netdata", ["netdata"], NONE),
    t!(MON, "lm-sensors", ["sensors"], V),
];

/// Recherche une liste d'outils (en parallèle, avec un nombre borné de processus).
pub fn check_tools(defs: &[ToolDef]) -> (Vec<Tool>, Vec<Tool>) {
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).clamp(2, 8);
    let results: Vec<(usize, Option<Tool>, Tool)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|w| {
                s.spawn(move || {
                    defs.iter()
                        .enumerate()
                        .skip(w)
                        .step_by(workers)
                        .map(|(i, d)| {
                            let found = check_one(d);
                            let missing = Tool { category: d.category.into(), name: d.name.into(), command: d.commands[0].into(), path: None, version: None };
                            (i, found, missing)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    });
    let mut results = results;
    results.sort_by_key(|r| r.0);
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for (_, f, m) in results {
        match f {
            Some(t) => found.push(t),
            None => missing.push(m),
        }
    }
    (found, missing)
}

fn check_one(d: &ToolDef) -> Option<Tool> {
    let (cmd, path) = d.commands.iter().find_map(|c| find_exe(c).map(|p| (*c, p)))?;
    let path_s = path.to_string_lossy().into_owned();
    let version = match d.version_args {
        Some(args) => {
            let out = run(&path_s, args)?;
            // Sous-commande absente (ex. `docker compose` sans le plugin) : l'outil n'est pas là.
            if !out.success && args.len() > 1 && !args[0].starts_with('-') {
                return None;
            }
            version_from(out.text())
        }
        None => None,
    };
    Some(Tool { category: d.category.into(), name: d.name.into(), command: cmd.into(), path: Some(path_s), version })
}

/// Version classique `x.y.z`, sinon `version: 4102` (llama.cpp et autres builds sans semver).
pub fn version_from(text: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    extract_version(text).or_else(|| {
        let re = RE.get_or_init(|| Regex::new(r"(?i)version[:\s]+([0-9][\w.\-+]*)").unwrap());
        re.captures(text).map(|c| c[1].to_string())
    })
}

pub fn probe() -> DevEnv {
    let (tools, missing) = check_tools(TOOLS);
    let mut dev = DevEnv { tools, missing, ..Default::default() };
    dev.cross_compilers = cross_compilers();

    if has_exe("docker") {
        // Le démon est-il accessible pour l'utilisateur courant (groupe docker) ?
        let info = run("docker", &["info", "--format", "{{json .Runtimes}}"]);
        dev.docker_daemon_access = Some(info.as_ref().is_some_and(|o| o.success));
        if let Some(o) = info.filter(|o| o.success) {
            if let Ok(serde_json::Value::Object(m)) = serde_json::from_str(o.stdout.trim()) {
                dev.docker_runtimes = m.keys().cloned().collect();
            }
        }
    }
    dev.binfmt = list_dir("/proc/sys/fs/binfmt_misc").into_iter().filter(|e| e.starts_with("qemu-")).collect();
    dev
}

/// Compilateurs croisés présents dans le PATH (ex. `aarch64-linux-gnu-gcc`).
fn cross_compilers() -> Vec<String> {
    let re = Regex::new(r"^[a-z0-9_]+(-[a-z0-9_]+){1,3}-(gcc|g\+\+|clang|clang\+\+)(-\d+)?$").unwrap();
    let native = std::env::consts::ARCH;
    let mut set = BTreeSet::new();
    let path = std::env::var("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        for f in list_dir(&dir) {
            if re.is_match(&f) && !f.starts_with(native) && !f.starts_with("x86_64-pc-linux") && Path::new(&dir).join(&f).is_file() {
                set.insert(f);
            }
        }
    }
    set.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(version_from("version: 4102 (a1b2c3)").as_deref(), Some("4102"));
        assert_eq!(version_from("cmake version 3.28.3").as_deref(), Some("3.28.3"));
    }
}
