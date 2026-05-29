# 🧠 IntelliProbe - Intelligent Workstation Profiler

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)](https://github.com/yourusername/intelliprobe)
[![Dashboard](https://img.shields.io/badge/Web-Dashboard-00eaff.svg)](https://github.com/yourusername/intelliprobe)
[![Docker](https://img.shields.io/badge/Docker-Ready-2496ED.svg)](https://www.docker.com)

> **Analysez intelligemment votre workstation pour l'IA et le développement**

IntelliProbe est un outil d'analyse système complet qui évalue les capacités de votre machine pour les workloads d'intelligence artificielle et de développement logiciel. Il détecte automatiquement votre matériel, les langages installés, les bibliothèques disponibles, et génère des prompts personnalisés pour les LLM comme Qwen Coder, Claude ou GPT.

---

## 📋 Table des matières

- [✨ Fonctionnalités](#-fonctionnalités)
- [🖼️ Aperçu](#️-aperçu)
- [📋 Prérequis](#-prérequis)
- [🚀 Installation](#-installation)
  - [Linux (Debian/Ubuntu)](#linux-debianubuntu)
  - [Linux (Fedora/RHEL)](#linux-fedorarhel)
  - [Linux (Arch Linux)](#linux-arch-linux)
  - [Linux (Alpine)](#linux-alpine)
  - [macOS](#macos)
  - [Windows](#windows)
  - [Docker](#docker)
  - [Compilation manuelle](#compilation-manuelle)
- [📖 Utilisation](#-utilisation)
  - [Mode CLI](#mode-cli)
  - [Mode Web Dashboard](#mode-web-dashboard)
  - [Mode API REST](#mode-api-rest)
  - [Options complètes](#options-complètes)
- [🎨 Dashboard Web](#-dashboard-web)
  - [Sections disponibles](#sections-disponibles)
  - [🤖 Générateur de Prompt IA](#-générateur-de-prompt-ia)
- [📊 Exemple de sortie](#-exemple-de-sortie)
- [🏗️ Architecture](#️-architecture)
- [🔧 Variables d'environnement](#-variables-denvironnement)
- [📄 Fichiers générés](#-fichiers-générés)
- [🤝 Contribution](#-contribution)
- [📝 Roadmap](#-roadmap)
- [📄 License](#-license)
- [🙏 Remerciements](#-remerciements)

---

## ✨ Fonctionnalités

### 🔍 Analyse système complète
| Composant | Détections |
|-----------|------------|
| **CPU** | Modèle, nombre de cœurs, fréquence, température, jeu d'instructions (AVX, AVX-512) |
| **Mémoire** | RAM totale, Swap, disques (SSD/HDD), espace libre/occupé |
| **GPU** | Modèle, VRAM, driver, température, cœurs CUDA, Tensor Cores |
| **NPU** | Tous les NPU (Rockchip RK3588/RK3576, Intel NPU, AMD Ryzen AI, Google Edge TPU, Hailo, Axelera) |
| **Réseau** | Interfaces, configuration réseau |
| **Système** | OS, version, kernel, hostname, uptime, load average, shell, environnement bureau |

### 🤖 Analyse des capacités IA
- **Score IA personnalisé** (0-100) basé sur CPU, RAM, GPU, NPU
- **Détection des backends** : PyTorch, TensorFlow, ONNX Runtime, RKNN, TensorRT, JAX
- **Performance estimée** : FP32/FP16/INT8 TFLOPS/TOPS, bande passante mémoire
- **Latence inférence** : ResNet50, YOLOv5s, BERT-base, Whisper, Stable Diffusion
- **Recommandations workloads** : LLM, Computer Vision, NLP, Audio, Générative, Temps réel

### 💻 Analyse des capacités développement
- **Langages** : Python, Node.js, Rust, C/C++, Java, Go, C#, Ruby, PHP, Swift, Kotlin
- **Outils de build** : make, cmake, ninja, cargo, gradle, mvn, just
- **Conteneurs** : Docker, Podman, Kubernetes (kubectl), Docker Compose, Buildx
- **Bases de données** : PostgreSQL, MySQL, Redis, SQLite, MongoDB, Cassandra
- **IDEs** : VS Code, IntelliJ IDEA, PyCharm, RustRover, GoLand, CLion, Vim, Neovim, Emacs
- **Monitoring** : htop, btop, glances, netdata

### 📚 Détection des librairies
| Type | Détection |
|------|-----------|
| **Python** | Tous les packages pip installés (avec versions) |
| **Node.js** | Packages npm globaux et locaux |
| **Rust** | Crates installés globalement |
| **Système** | Toutes les librairies `.so` (CUDA, OpenCV, FFmpeg, OpenGL, Vulkan, OpenCL, BLAS, LAPACK, MPI, etc.) |

### 🤖 Générateur de Prompt IA
- **10+ types de projets** : Python, Node.js, Rust, C/C++, Web, ML/IA, Data Engineering, Mobile, Game Dev, Embedded, DevOps
- **Niveaux d'isolation** : Système direct / Virtual Environment / Docker
- **Formats de réponse** : Standard / Script uniquement / Détaillé
- **Instructions personnalisables** : Ajoutez vos propres contraintes
- **Export des dépendances** : JSON + TXT pour le langage sélectionné
- **Compatibilité** : Qwen Coder, Claude, GPT-4, GitHub Copilot

### 🌐 Dashboard web moderne
- Interface premium avec glassmorphism et animations
- Navigation dynamique (les sections s'adaptent aux langages installés)
- Visualisation de toutes les données en temps réel
- Export des dépendances en un clic
- Design responsive (mobile, tablette, desktop)
- Thème sombre optimisé

### 🛠️ Mode CLI
- Analyse rapide sans interface
- Export JSON, Markdown, HTML
- Mode CI/CD avec seuil d'échec
- Génération automatique de Dockerfile
- Verbose pour debugging

### 🔌 Mode API REST
- Serveur API intégré
- Endpoints pour récupérer les rapports
- Endpoints pour déclencher des analyses
- Export au format JSON
- WebSocket pour les mises à jour en temps réel

---

## 🖼️ Aperçu
╔══════════════════════════════════════════════════════════════╗
║ ███████╗██╗ ██╗███████╗██████╗ ██████╗ ██████╗ ██████╗ ███████╗
║ ██╔════╝╚██╗ ██╔╝██╔════╝██╔══██╗██╔══██╗██╔═══██╗██╔══██╗██╔════╝
║ ███████╗ ╚████╔╝ █████╗ ██████╔╝██████╔╝██║ ██║██████╔╝█████╗
║ ╚════██║ ╚██╔╝ ██╔══╝ ██╔══██╗██╔══██╗██║ ██║██╔══██╗██╔══╝
║ ███████║ ██║ ███████╗██║ ██║██████╔╝╚██████╔╝██████╔╝███████╗
║ ╚══════╝ ╚═╝ ╚══════╝╚═╝ ╚═╝╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝
║ ║
║ 🚀 Intelligent Workstation Profiler v2.0 ║
║ 🔬 Analyse IA & Développement ║
║ ║
╚══════════════════════════════════════════════════════════════╝

text

---

## 📋 Prérequis

| Plateforme | Prérequis |
|------------|-----------|
| **Linux** | Rust 1.75+, build-essential, pkg-config, libssl-dev |
| **macOS** | Rust 1.75+, Homebrew, pkg-config, openssl |
| **Windows** | Rust 1.75+, Visual Studio Build Tools |
| **Docker** | Docker Engine 20.10+ |

---

## 🚀 Installation

### 🐧 Linux (Debian/Ubuntu)

```bash
# Installation des dépendances
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev git curl

# Installation de Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Clonage et compilation
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
cargo build --release --features web

# Installation système
sudo cp target/release/intelliprobe /usr/local/bin/

# Lancement
intelliprobe --dashboard
🐧 Linux (Fedora/RHEL)
bash
# Installation des dépendances
sudo dnf install -y gcc pkg-config openssl-devel git curl

# Installation de Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Clonage et compilation
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
cargo build --release --features web

# Installation système
sudo cp target/release/intelliprobe /usr/local/bin/

# Lancement
intelliprobe --dashboard
🐧 Linux (Arch Linux)
bash
# Installation des dépendances
sudo pacman -S --needed base-devel openssl git curl rust

# Clonage et compilation
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
cargo build --release --features web

# Installation système
sudo cp target/release/intelliprobe /usr/local/bin/

# Lancement
intelliprobe --dashboard
🐧 Linux (Alpine)
bash
# Installation des dépendances
apk add --no-cache gcc musl-dev openssl-dev git curl rust cargo

# Clonage et compilation
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
cargo build --release --features web

# Installation système
sudo cp target/release/intelliprobe /usr/local/bin/

# Lancement
intelliprobe --dashboard
🍎 macOS
bash
# Installation de Homebrew (si nécessaire)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Installation des dépendances
brew install pkg-config openssl git rust

# Clonage et compilation
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe
cargo build --release --features web

# Installation système
sudo cp target/release/intelliprobe /usr/local/bin/

# Lancement
intelliprobe --dashboard
🪟 Windows
powershell
# 1. Installer Rust depuis https://rustup.rs/
# 2. Installer Visual Studio Build Tools avec "Desktop development with C++"
# 3. Ouvrir PowerShell en Administrateur

# Clonage
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe

# Compilation
cargo build --release --features web

# Lancement
.\target\release\intelliprobe.exe --dashboard
🐳 Docker
bash
# Clonage
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe

# Build de l'image
docker build -t intelliprobe .

# Lancement du dashboard
docker run --rm -p 8080:8080 -v $(pwd):/data intelliprobe --dashboard

# Lancement en CLI
docker run --rm -v $(pwd):/data intelliprobe --output /data/report.json
🔧 Compilation manuelle
bash
# Sans interface web (CLI uniquement)
cargo build --release

# Avec interface web
cargo build --release --features web

# Avec toutes les fonctionnalités
cargo build --release --features all

# Version de production (optimisée)
cargo build --profile dist --features all
📖 Utilisation
Mode CLI
bash
# Analyse simple
intelliprobe

# Analyse avec export JSON
intelliprobe --output report.json

# Export Markdown
intelliprobe --format markdown --output report.md

# Export HTML
intelliprobe --format html --output report.html

# Mode CI (sortie JSON, échoue si score < 70)
intelliprobe --ci-mode --threshold 70

# Générer Dockerfile
intelliprobe --dockerfile

# Mode verbose
intelliprobe --verbose

# Aide complète
intelliprobe --help
Mode Web Dashboard
bash
# Lancement standard
intelliprobe --dashboard

# Avec logs détaillés
RUST_LOG=debug intelliprobe --dashboard

# Sur un port spécifique (modifier dans le code)
# Ouvrir http://localhost:8080
Mode API REST
bash
# Lancement du serveur API
intelliprobe --api --port 3000

# Endpoints disponibles:
# GET  /api/v1/report           - Rapport complet
# GET  /api/v1/report/summary   - Résumé des scores
# GET  /api/v1/report/hardware  - Informations matériel
# GET  /api/v1/report/languages - Langages détectés
# POST /api/v1/analyze          - Nouvelle analyse
# GET  /api/v1/export/json      - Export JSON
# GET  /api/v1/export/markdown  - Export Markdown
# GET  /api/v1/export/html      - Export HTML
Options complètes
Option	Description
-i, --input <FILE>	Charger un rapport existant (skip probing)
-o, --output <FILE>	Fichier de sortie pour le rapport
-f, --format <FORMAT>	Format de sortie (json, markdown, html)
--dashboard	Lancer le dashboard web
--api	Lancer le serveur API
--port <PORT>	Port pour l'API (défaut: 3000)
--benchmark	Exécuter les benchmarks
--ci-mode	Mode CI (non-interactif)
--threshold <SCORE>	Code de retour non nul si score < seuil
--dockerfile	Générer un Dockerfile
--verbose	Mode verbeux
🎨 Dashboard Web
Le dashboard propose une navigation par catégories :

Sections disponibles
Section	Icône	Contenu
Synthèse	📊	Scores IA/Dev, forces, faiblesses, infos système
Matériel	💻	CPU, RAM, GPU, stockage, températures
Performance	⚡	TFLOPS, TOPS, latence inférence
NPU	🧠	Tous les NPU détectés
Backends IA	🤖	PyTorch, TensorFlow, ONNX, TensorRT
Codecs	🎬	H.264, H.265, AV1, VP9
Compute APIs	🔌	CUDA, OpenCL, Vulkan, ROCm
Workloads IA	🎯	LLM, CV, NLP, Audio, Générative
Prompt IA	🤖	Générateur de prompts personnalisés
Langages	📝	Tous les langages détectés
Build	🔧	Outils de compilation
Conteneurs	🐳	Docker, Podman, Kubernetes
Bases données	🗄️	PostgreSQL, MySQL, Redis, SQLite
Git	📎	Version Control
IDEs	💡	Environnements de développement
Monitoring	📈	Outils de surveillance
Dev Workloads	🚀	Recommandations développement
Toolchain	🛠️	Configuration outillage
LibSystem	📚	Bibliothèques système (.so)
Code	📄	Snippets de code
JSON	📋	Données brutes
🤖 Générateur de Prompt IA
Le générateur de prompt permet de :

Choisir le type de projet

🐍 Python

💚 Node.js

🦀 Rust

⚙️ C/C++

🌐 Web Development

🤖 ML/IA

📊 Data Engineering

📱 Mobile

🎮 Game Dev

🔌 Embedded

☸️ DevOps

Choisir l'isolation

🖥️ Système direct (pas d'isolation)

📦 Virtual Environment (venv/nvm/cargo)

🐳 Docker / Conteneurisation

Choisir le format

📝 Standard (code + explications)

⚡ Script uniquement

📖 Détaillé (documentation complète)

Ajouter des instructions personnalisées

Exporter les dépendances du langage sélectionné (JSON + TXT)

Copier le prompt pour l'utiliser avec Qwen Coder, Claude ou GPT

📊 Exemple de sortie
╔══════════════════════════════════════════════════════════════════════╗
║                                                                      ║ 
║                       Intelli Probe v1.0                             ║
║            Intelligent Workstation Profiler & Analyzer               ║
║                                                                      ║
╚══════════════════════════════════════════════════════════════════════╝

════════════════════════════════════════════════════════════
📊 SYSTEM ANALYSIS COMPLETE
════════════════════════════════════════════════════════════

🤖 AI SCORE: 80/100 (High)
🛠️  DEV SCORE: 100/100 (Expert)

🎯 BEST FOR AI:
  • Large Language Models (LLaMA, Mistral)
  • Stable Diffusion / IA générative
  • Computer Vision / YOLO
  • Fine-tuning de modèles

💻 DETECTED LANGUAGES:
  • Python (3.10.12)
  • Node.js (20.20.2)
  • Rust (1.96.0)

🌐 Starting dashboard...

╔════════════════════════════════════════════════════════════╗
║                    🚀 DASHBOARD READY 🚀                    ║
╠════════════════════════════════════════════════════════════╣
║                                                            ║
║     🌐 Open in your browser: http://127.0.0.1:8080              ║
║                                                            ║
║     📊 Dashboard features:                                 ║
║        • AI & Dev Scores                                   ║
║        • Hardware analysis                                 ║
║        • Language detection                                ║
║        • Code snippets                                     ║
║                                                            ║
╚════════════════════════════════════════════════════════════╝

════════════════════════════════════════════════════════════

💾 Rapports sauvegardés:
  • system_capabilities.json - Données brutes
  • analysis_result.json - Analyse complète
  • Dockerfile - Conteneur reproductible

🏗️ Architecture
text
intelliprobe/
├── src/
│   ├── main.rs              # CLI et orchestration
│   ├── lib.rs               # Point d'entrée de la bibliothèque
│   ├── analyze/
│   │   └── mod.rs           # Logique d'analyse principale
│   ├── probes/              # Sondes système
│   │   ├── cpu.rs           # Détection CPU
│   │   ├── gpu.rs           # Détection GPU
│   │   ├── memory.rs        # Détection mémoire
│   │   ├── languages.rs     # Détection langages
│   │   ├── libraries.rs     # Détection librairies
│   │   ├── benchmarks.rs    # Benchmarks
│   │   └── platform.rs      # Détection plateforme
│   ├── detectors/           # Détection multi-plateforme
│   │   ├── package_manager.rs # Gestionnaire de paquets
│   │   └── environment.rs   # Environnement
│   ├── exporters/           # Export des rapports
│   │   ├── json.rs
│   │   ├── markdown.rs
│   │   └── html.rs
│   ├── comparators/         # Comparaison avec références
│   │   └── reference.rs
│   ├── api/                 # Serveur API REST
│   │   └── routes.rs
│   └── web/                 # Dashboard web
│       ├── mod.rs
│       └── dashboard.html
├── Cargo.toml
└── README.md
🔧 Variables d'environnement
Variable	Description
RUST_LOG	Niveau de log (debug, info, warn, error)
HOME	Répertoire personnel (détection des configs)
SHELL	Shell utilisé pour les commandes
PATH	Chemins de recherche des exécutables
📄 Fichiers générés
Fichier	Description
system_capabilities.json	Données brutes des probes
analysis_result.json	Rapport d'analyse complet
Dockerfile	Généré avec --dockerfile
dependencies_*.json	Export des dépendances (JSON)
dependencies_*.txt	Export des dépendances (TXT)
🤝 Contribution
Les contributions sont les bienvenues !

bash
# Fork le projet
git clone https://github.com/yourusername/intelliprobe.git
cd intelliprobe

# Créer une branche
git checkout -b feature/amazing

# Développer
cargo build --features all
cargo test

# Formatter
cargo fmt

# Linter
cargo clippy

# Commit et push
git commit -m "Add amazing feature"
git push origin feature/amazing

# Ouvrir une Pull Request
📝 Roadmap
Support Windows natif complet

Support macOS natif complet

Benchmarks CPU/GPU en temps réel

Mode TUI (terminal UI) avec ratatui

Export PDF des rapports

Plugins système pour extensions

Intégration CI/CD (GitHub Actions, GitLab CI)

Détection des conteneurs en cours d'exécution

Monitoring en temps réel

Mode serveur multi-utilisateurs

Interface graphique native (egui/iced)

📄 License
Ce projet est sous licence Apache License 2.0 - voir le fichier LICENSE pour plus de détails.

text
Copyright 2025 Arnaud Flourac

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
🙏 Remerciements
sysinfo - Informations système

axum - Framework web

clap - CLI argument parsing

tokio - Runtime async

serde - Sérialisation

rayon - Parallélisation

⭐ Support du projet
Si ce projet vous est utile, n'hésitez pas à :

⭐ Mettre une étoile sur GitHub

🐛 Signaler les bugs

💡 Proposer des améliorations

🔁 Partager le projet

Développé avec ❤️ par Arnaud Flourac

https://img.shields.io/badge/GitHub-100000?style=for-the-badge&logo=github&logoColor=white