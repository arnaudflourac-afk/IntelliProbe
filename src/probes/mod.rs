//! Sondes système. Chaque sonde lit l'état réel de la machine
//! (/proc, /sys, device-tree, ioctl, commandes) et ne renvoie que ce qu'elle a trouvé.

pub mod ai;
pub mod bench;
pub mod compute;
pub mod cpu;
pub mod devtools;
pub mod dt;
pub mod gpu;
pub mod interfaces;
pub mod libs;
pub mod npu;
pub mod python;
pub mod storage;
pub mod system;
pub mod video;
