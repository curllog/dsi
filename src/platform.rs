use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
    pub libc: Libc,
    pub wsl: WslStatus,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Os {
    Linux,
    MacOS,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Arch {
    X64,
    Arm64,
    Arm,
    X86,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Libc {
    Glibc,
    Musl,
    None, // macOS
}
#[derive(Debug, Clone, PartialEq)]
pub enum WslStatus {
    None,
    Wsl1,
    Wsl2,
}

impl Platform {
    pub fn detect() -> Self {
        Platform {
            os: detect_os(),
            arch: detect_arch(),
            libc: detect_libc(),
            wsl: detect_wsl(),
        }
    }

    pub fn rid(&self) -> String {
        let os_part = match (&self.os, &self.libc) {
            (Os::Linux, Libc::Musl) => "linux-musl",
            (Os::Linux, _) => "linux",
            (Os::MacOS, _) => "osx",
        };

        let arch_part = match self.arch {
            Arch::X64 => "x64",
            Arch::Arm64 => "arm64",
            Arch::Arm => "arm",
            Arch::X86 => "x86",
        };

        format!("{}-{}", os_part, arch_part)
    }
    pub fn display_name(&self) -> String {
        let libc_str = match self.libc {
            Libc::Glibc => " (glibc)",
            Libc::Musl => " (musl)",
            Libc::None => "",
        };
        format!("{}{}", self.rid(), libc_str)
    }
}

// ─── Individual Detectors ─────────────────────────────────────────────────

/// Detect OS using Rust's compile-time constant.
fn detect_os() -> Os {
    match std::env::consts::OS {
        "linux" => Os::Linux,
        "macos" => Os::MacOS,
        other => panic!("Unsupported OS: {}", other),
    }
}

/// Detect CPU architecture using Rust's compile-time constant.
fn detect_arch() -> Arch {
    match std::env::consts::ARCH {
        "x86_64" => Arch::X64,
        "aarch64" => Arch::Arm64,
        "arm" => Arch::Arm,
        "x86" => Arch::X86,
        other => panic!("Unsupported architecture: {}", other),
    }
}

/// Detect whether the system uses musl or glibc (Linux only).
///
/// Strategy:
/// 1. Run `ldd --version` — musl prints to stderr and includes "musl"
/// 2. Check if the musl dynamic linker exists on disk
/// 3. Default to glibc (the vast majority of Linux distros)
fn detect_libc() -> Libc {
    if std::env::consts::OS != "linux" {
        return Libc::None;
    }

    // Method 1: check ldd output
    if let Ok(output) = Command::new("ldd").arg("--version").output() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.to_lowercase().contains("musl") {
            return Libc::Musl;
        }
    }

    // Method 2: check for musl dynamic linker on disk
    let musl_paths = [
        "/lib/ld-musl-x86_64.so.1",
        "/lib/ld-musl-aarch64.so.1",
        "/lib/ld-musl-armhf.so.1",
    ];
    for path in &musl_paths {
        if Path::new(path).exists() {
            return Libc::Musl;
        }
    }

    // Default: glibc
    Libc::Glibc
}

/// Detect whether we're running inside Windows Subsystem for Linux.
fn detect_wsl() -> WslStatus {
    if std::env::consts::OS != "linux" {
        return WslStatus::None;
    }

    // Method 1: WSL_DISTRO_NAME is set by the WSL runtime
    if std::env::var("WSL_DISTRO_NAME").is_ok() {
        if let Ok(version) = std::fs::read_to_string("/proc/version") {
            if version.contains("microsoft-standard-WSL2") || version.contains("WSL2") {
                return WslStatus::Wsl2;
            }
            if version.to_lowercase().contains("microsoft") {
                return WslStatus::Wsl1;
            }
        }
        return WslStatus::Wsl2; // safe default
    }

    // Method 2: fallback — check /proc/version even without env var
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        if version.to_lowercase().contains("microsoft") == true {
            return WslStatus::Wsl2;
        }
    }

    WslStatus::None
}
