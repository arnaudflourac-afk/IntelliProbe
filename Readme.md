# IntelliProbe

**Savoir en une commande ce qu'une carte (SBC ou PC) sait réellement faire,**
avant de partir dans une direction de développement.

IntelliProbe inventorie le matériel, les accélérateurs IA (GPU, NPU), les environnements
de développement et les librairies **effectivement présents** sur la machine où il tourne.
Tout est mesuré ou lu sur le système : `/proc`, `/sys`, device-tree, ioctl V4L2/GPIO,
imports Python réels, micro-benchmarks. Une information introuvable est signalée comme
telle, jamais inventée.

Pensé d'abord pour les cartes ARM / RISC-V (Rockchip, Raspberry Pi, Jetson, Amlogic,
NXP, Radxa, Orange Pi, Khadas…), il fonctionne aussi sur un PC Linux.

## Ce qu'il répond

| Question | Comment c'est obtenu |
|---|---|
| Quelle carte, quel SoC, quel OS ? | device-tree (`model`, `compatible`), DMI, `/etc/os-release`, L4T (Jetson), révision Raspberry Pi |
| Userland 32 ou 64 bits, glibc ou musl ? | `getconf LONG_BIT`, `dpkg --print-architecture`, `ldd --version` |
| Quels cœurs CPU (big.LITTLE), quelles extensions SIMD ? | `/proc/cpuinfo` (décodage des cœurs ARM), cpufreq, flags : NEON, dotprod, i8mm, SVE, AVX2, AVX-512, RVV… |
| Quel GPU, avec quel driver ? | `nvidia-smi`, PCI, nœuds DRM (Mali Panfrost/Panthor, VideoCore, Adreno, Vivante…), Mali kbase, Tegra, devfreq |
| Y a-t-il un NPU, est-il activé, le runtime est-il installé ? | nœuds du device-tree (et leur `status`), `/dev/accel`, drivers DRM (RKNPU), `/dev/rknpu`, `/dev/galcore`, `/dev/hailo*`, PCI, USB (Coral, Movidius), modules noyau |
| CUDA / OpenCL / Vulkan / ROCm / VA-API utilisables ? | `nvidia-smi`, `nvcc`, headers cuDNN/TensorRT, `clinfo`, `vulkaninfo`, `vainfo`, ICD installés |
| Quels frameworks IA fonctionnent, et sur quel accélérateur ? | **import réel** de PyTorch, TensorFlow, ONNX Runtime, OpenVINO, TFLite/LiteRT, RKNN Lite, HailoRT, PyCoral, JAX, OpenCV… dans un sous-processus isolé |
| Quels codecs vidéo matériels ? | ioctl V4L2 (encodeurs/décodeurs mem2mem), VA-API, **test d'encodage réel** avec les encodeurs matériels de FFmpeg, nœuds MPP / NVENC Jetson, GStreamer |
| Quelles interfaces pour l'électronique ? | GPIO (nom + nombre de lignes), I2C, SPI, UART, CAN, PWM, USB, PCI, réseau, Bluetooth, et contrôleurs **désactivés** dans le device-tree (activables par overlay) |
| Quelles librairies puis-je utiliser ? | cache `ldconfig` complet + dossiers CUDA/Tegra/ROCm, librairies de dev (`pkg-config`), tous les paquets Python, npm global, cargo |
| Quels outils de dev ? | ~150 outils recherchés (compilateurs, compilateurs croisés, build, embarqué, conteneurs, bases, éditeurs…) avec leur version |
| Quelles performances réelles ? | micro-benchmarks : GFLOPS FP32 (1 cœur / tous), bande passante mémoire en lecture, débit disque (écriture fsync + lecture hors cache) |
| Quel LLM puis-je faire tourner ? | calcul à partir de la RAM disponible et de la bande passante **mesurée** (formule affichée dans le rapport) |

À la fin, une **analyse** liste les voies d'accélération IA (utilisable / partiel / absent)
et les constats : ce qui est prêt, ce qui manque, les pièges (userland 32 bits, système sur
carte SD, NPU sans runtime, PyTorch qui ne voit pas le GPU…).

## Installation

### Compiler sur la carte

```bash
sudo apt install -y build-essential git curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
git clone https://github.com/arnaudflourac-afk/IntelliProbe.git
cd IntelliProbe
cargo build --release
sudo cp target/release/intelliprobe /usr/local/bin/
```

Le projet n'a que 8 dépendances légères : la compilation reste raisonnable même sur une petite carte.

### Compiler sur un PC et copier sur la carte (recommandé pour les cartes lentes)

```bash
sudo apt install gcc-aarch64-linux-gnu
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
scp target/aarch64-unknown-linux-gnu/release/intelliprobe user@carte:~
```

Les linkers pour aarch64, armv7 et riscv64 sont déjà déclarés dans `.cargo/config.toml`.

## Utilisation

```bash
intelliprobe                        # analyse complète (~10 s à 1 min selon la carte)
intelliprobe -o ~/rapports/rock5b   # choisir le dossier de sortie
intelliprobe --no-bench             # sans les mesures CPU/mémoire/disque
intelliprobe --no-python-import     # sans importer les frameworks (plus rapide)
intelliprobe --json > profil.json   # JSON seul sur la sortie standard
intelliprobe -v                     # progression détaillée + durée de chaque sonde
intelliprobe --dashboard --host 0.0.0.0 --port 8080   # consulter depuis un autre poste
intelliprobe --input rapport.json --dashboard          # relire le rapport d'une autre carte
```

Fichiers produits (dossier `rapport-intelliprobe/` par défaut) :

| Fichier | Usage |
|---|---|
| `rapport.html` | dashboard autonome : s'ouvre dans n'importe quel navigateur, **sans serveur ni Internet** (idéal : `scp` depuis la carte) |
| `rapport.md` | rapport complet lisible sur GitHub/GitLab, versionnable pour comparer des cartes |
| `rapport.json` | toutes les données brutes |
| `prompt.txt` | résumé prêt à coller dans un assistant IA pour qu'il propose des choix adaptés à la carte |

Lancer en root donne quelques informations en plus (version du driver RKNPU via debugfs, par
exemple), mais ce n'est pas nécessaire. Pour voir les caméras et codecs V4L2, l'utilisateur
doit appartenir au groupe `video`.

## Architecture

```
src/
├── main.rs            CLI
├── lib.rs             orchestration (sondes en parallèle, puis mesures, puis analyse)
├── report.rs          structure du rapport
├── analysis.rs        voies d'accélération, constats, estimation LLM
├── util.rs            commandes avec timeout, lecture sysfs, versions
├── probes/
│   ├── system.rs      carte, SoC, OS
│   ├── cpu.rs         CPU, clusters, extensions SIMD, capteurs de température
│   ├── storage.rs     mémoire, zram, disques, systèmes de fichiers
│   ├── gpu.rs         GPU PCI et intégrés
│   ├── npu.rs         NPU et accélérateurs IA
│   ├── compute.rs     CUDA, OpenCL, Vulkan, ROCm, VA-API
│   ├── video.rs       V4L2 (ioctl), FFmpeg, GStreamer
│   ├── interfaces.rs  GPIO, I2C, SPI, UART, CAN, PWM, USB, PCI, réseau
│   ├── dt.rs          lecture du device-tree
│   ├── libs.rs        librairies système, pkg-config, npm, cargo
│   ├── devtools.rs    outils de développement
│   ├── python.rs      paquets et test des frameworks IA
│   ├── ai.rs          outils d'inférence (ollama, llama.cpp, TensorRT…)
│   └── bench.rs       micro-benchmarks
├── output/            terminal, Markdown, HTML, prompt
└── web/dashboard.html gabarit du dashboard (intégré au binaire)
```

Tests : `cargo test` (les analyseurs sont testés sur des extraits réels de RK3588, Raspberry Pi,
Jetson, `nvidia-smi`, `vulkaninfo`, `clinfo`…).

## Limites connues

- Linux uniquement pour l'instant (macOS/Windows : compilation non garantie, la plupart des sondes lisent `/proc` et `/sys`).
- Les TOPS des NPU ne sont affichés que lorsque la puce est identifiée sans ambiguïté (valeur constructeur).
- Le benchmark CPU utilise le SIMD de base de l'architecture (pas d'AVX2/AVX-512 spécifique) : il compare des machines entre elles, ce n'est pas un pic théorique.
- L'estimation LLM est une borne haute pour l'inférence CPU ; le débit réel est typiquement de 30 à 70 % de cette borne.

## Licence

Apache License 2.0 — développé par Arnaud Flourac.
