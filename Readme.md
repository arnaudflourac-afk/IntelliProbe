# 🧠 IntelliProbe - Intelligent Workstation Profiler

[License: Apache 2.0]
[Rust: 1.75+]
[Platform: Linux | macOS | Windows]
[Web: Dashboard]
[Docker: Ready]

> Analysez intelligemment votre workstation pour l'IA et le développement

IntelliProbe est un outil d'analyse système complet qui évalue les capacités de votre machine pour les workloads d'intelligence artificielle et de développement logiciel. Il détecte automatiquement votre matériel, les langages installés, les bibliothèques disponibles, et génère des prompts personnalisés pour les LLM comme Qwen Coder, Claude ou GPT.

---

## 📋 Table des matières

- ✨ Fonctionnalités
- 🖼️ Aperçu
- 📋 Prérequis
- 🚀 Installation
  - Linux (Debian/Ubuntu)
  - Linux (Fedora/RHEL)
  - Linux (Arch Linux)
  - Linux (Alpine)
  - macOS
  - Windows
  - Docker
  - Compilation manuelle
- 📖 Utilisation
- 🎨 Dashboard Web
- 🤖 Générateur de Prompt IA
- 📊 Exemple de sortie
- 🏗️ Architecture
- 🔧 Variables d'environnement
- 📄 Fichiers générés
- 🤝 Contribution
- 📝 Roadmap
- 📄 License
- 🙏 Remerciements

---

## ✨ Fonctionnalités

### 🔍 Analyse système complète

| Composant | Détections |
|-----------|------------|
| CPU       | Modèle, nombre de cœurs, fréquence, température, jeu d'instructions (AVX, AVX-512) |
| Mémoire   | RAM totale, Swap, disques (SSD/HDD), espace libre/occupé |
| GPU       | Modèle, VRAM, driver, température, cœurs CUDA, Tensor Cores |
| NPU       | Rockchip RK3588/RK3576, Intel NPU, AMD Ryzen AI, Google Edge TPU, Hailo, Axelera |
| Réseau    | Interfaces, configuration réseau |
| Système   | OS, version, kernel, hostname, uptime, load average, shell, environnement bureau |

### 🤖 Analyse des capacités IA
- Score IA personnalisé (0-100) basé sur CPU, RAM, GPU, NPU
- Détection des backends : PyTorch, TensorFlow, ONNX Runtime, RKNN, TensorRT, JAX
- Performance estimée : FP32/FP16/INT8 TFLOPS/TOPS, bande passante mémoire
- Latence inférence : ResNet50, YOLOv5s, BERT-base, Whisper, Stable Diffusion
- Recommandations workloads : LLM, Computer Vision, NLP, Audio, Générative, Temps réel

### 💻 Analyse des capacités développement
- Langages : Python, Node.js, Rust, C/C++, Java, Go, C#, Ruby, PHP, Swift, Kotlin
- Outils de build : make, cmake, ninja, cargo, gradle, mvn, just
- Conteneurs : Docker, Podman, Kubernetes (kubectl), Docker Compose, Buildx
- Bases de données : PostgreSQL, MySQL, Redis, SQLite, MongoDB, Cassandra
- IDEs : VS Code, IntelliJ IDEA, PyCharm, RustRover, GoLand, CLion, Vim, Neovim, Emacs
- Monitoring : htop, btop, glances, netdata

### 📚 Détection des librairies
| Type     | Détection |
|----------|-----------|
| Python   | Tous les packages pip installés (avec versions) |
| Node.js  | Packages npm globaux et locaux |
| Rust     | Crates installés globalement |
| Système  | Toutes les librairies .so (CUDA, OpenCV, FFmpeg, OpenGL, etc.) |

---

## 🖼️ Aperçu
🚀 Intelligent Workstation Profiler v2.0
🔬 Analyse IA & Développement

---

## 📋 Prérequis
| Plateforme | Prérequis |
|------------|-----------|
| Linux      | Rust 1.75+, build-essential, pkg-config, libssl-dev |
| macOS      | Rust 1.75+, Homebrew, pkg-config, openssl |
| Windows    | Rust 1.75+, Visual Studio Build Tools |
| Docker     | Docker Engine 20.10+ |

---

## 🚀 Installation

### 🐧 Linux (Debian/Ubuntu)
sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev git curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
cargo build --release --features web
sudo cp target/release/intelliprobe /usr/local/bin/
intelliprobe --dashboard

### 🐧 Linux (Fedora/RHEL)
sudo dnf install -y gcc pkg-config openssl-devel git curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
cargo build --release --features web
sudo cp target/release/intelliprobe /usr/local/bin/
intelliprobe --dashboard

### 🐧 Linux (Arch Linux)
sudo pacman -S --needed base-devel openssl git curl rust
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
cargo build --release --features web
sudo cp target/release/intelliprobe /usr/local/bin/
intelliprobe --dashboard

### 🐧 Linux (Alpine)
apk add --no-cache gcc musl-dev openssl-dev git curl rust cargo
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
cargo build --release --features web
sudo cp target/release/intelliprobe /usr/local/bin/
intelliprobe --dashboard

### 🍎 macOS
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew install pkg-config openssl git rust
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
cargo build --release --features web
sudo cp target/release/intelliprobe /usr/local/bin/
intelliprobe --dashboard

### 🪟 Windows
1. Installer Rust depuis https://rustup.rs/
2. Installer Visual Studio Build Tools avec "Desktop development with C++"
3. git clone https://github.com/yourusername/intelliprobe.git
4. cd intelliprobe
5. cargo build --release --features web
6. .\target\release\intelliprobe.exe --dashboard

### 🐳 Docker
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
docker build -t intelliprobe .
docker run --rm -p 8080:8080 -v $(pwd):/data intelliprobe --dashboard

---

## 📖 Utilisation

### Mode CLI
intelliprobe
intelliprobe --output report.json
intelliprobe --format markdown
intelliprobe --ci-mode --threshold 70

### Mode Web Dashboard
intelliprobe --dashboard

### Mode API REST
intelliprobe --api --port 3000

---

## 🏗️ Architecture
intelliprobe/
├── src/
│   ├── main.rs              # CLI et orchestration
│   ├── analyze/             # Logique d'analyse principale
│   ├── probes/              # Sondes système
│   ├── exporters/           # JSON, Markdown, HTML
│   ├── api/                 # Serveur API REST
│   └── web/                 # Dashboard web
├── Cargo.toml
└── README.md

---

## 🤝 Contribution
1. Fork le projet.
2. Créer une branche (git checkout -b feature/amazing).
3. Développer (cargo build --features all).
4. Tester (cargo test).
5. Formater (cargo fmt).
6. Ouvrir une Pull Request.

---

## 📄 License
Ce projet est sous licence Apache License 2.0.
Copyright 2025 Arnaud Flourac

---

## 🙏 Remerciements
sysinfo, axum, clap, tokio, serde, rayon.

Développé avec ❤️ par Arnaud Flourac