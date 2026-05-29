🧠 IntelliProbe - Intelligent Workstation Profiler

Analysez intelligemment votre workstation pour l'IA et le développement.

IntelliProbe est un outil d'analyse système complet qui évalue les capacités de votre machine pour les workloads d'intelligence artificielle et de développement logiciel. Il détecte automatiquement votre matériel, les langages installés, les 
bibliothèques disponibles, et génère des prompts personnalisés pour les LLM (Qwen Coder, Claude, GPT).


📋 SOMMAIRE

Fonctionnalités

Aperçu

Prérequis

Installation

Utilisation

Architecture

Roadmap

License & Remerciements

✨ FONCTIONNALITÉS

🔍 Analyse système complète

CPU : Modèle, cœurs, fréquence, température, instructions (AVX, AVX-512)

Mémoire : RAM, Swap, disques (SSD/HDD)

GPU : Modèle, VRAM, drivers, CUDA, Tensor Cores

NPU : Support Rockchip, Intel, AMD Ryzen AI, Edge TPU, Hailo, Axelera

Système : OS, Kernel, Uptime, Load, Environnement bureau

🤖 Analyse IA et Développement
Score IA (0-100) basé sur le matériel

Détection de backends (PyTorch, TensorFlow, ONNX, JAX, etc.)

Analyse des langages, outils de build, conteneurs et bases de données

Générateur de prompts IA personnalisés (10+ types de projets)

🚀 INSTALLATION

🐧 Linux (Debian/Ubuntu)

Bash

sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev git curl

curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh -s -- -y

source ~/.cargo/env

git clone [https://github.com/arnaudflourac-afk/IntelliProbe.git](https://github.com/arnaudflourac-afk/IntelliProbe.git)

cd intelliprobe

cargo build --release --features web

sudo cp target/release/intelliprobe /usr/local/bin/

🍎 macOS

Bash

/bin/bash -c "$(curl -fsSL [https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh](https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh))"

brew install pkg-config openssl git rust

git clone [https://github.com/arnaudflourac-afk/IntelliProbe.git](https://github.com/arnaudflourac-afk/IntelliProbe.git)

cd intelliprobe

cargo build --release --features web

sudo cp target/release/intelliprobe /usr/local/bin/

🪟 Windows

Installer Rust depuis rustup.rs

Installer Visual Studio Build Tools (avec "Desktop development with C++")

git clone https://github.com/arnaudflourac-afk/IntelliProbe.git

cd intelliprobe

cargo build --release --features web

.\target\release\intelliprobe.exe --dashboard

📖 UTILISATION

CLI

Analyse simple : intelliprobe

Export JSON : intelliprobe --output report.json

CI/CD : intelliprobe --ci-mode --threshold 70

Web & API

Dashboard : intelliprobe --dashboard

API REST : intelliprobe --api --port 3000

🏗️ ARCHITECTURE

Plaintext

intelliprobe/

├── src/main.rs              # CLI et orchestration

├── src/analyze/             # Logique d'analyse principale

├── src/probes/              # Sondes système

├── src/exporters/           # JSON, Markdown, HTML

├── src/api/                 # Serveur API REST

└── src/web/                 # Interface Dashboard

📄 LICENSE & REMERCIEMENTS

Ce projet est sous licence Apache License 2.0.

Remerciements : sysinfo, axum, clap, tokio, serde, rayon.

Développé avec ❤️ par Arnaud Flourac
