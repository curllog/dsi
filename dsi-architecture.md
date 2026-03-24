# dsi — .NET SDK Installer

A fast, cross-platform CLI tool built in Rust for discovering, installing, and managing .NET SDK versions. It does not replace or interfere with the `dotnet` CLI — it complements it by solving the one pain point that remains: getting SDKs onto your machine.

---

## Philosophy

- **Do one thing well**: download and install .NET SDKs.
- **Respect the ecosystem**: let `dotnet` CLI handle version resolution, `global.json`, and rollForward policies.
- **Stay out of the way**: no shims, no PATH manipulation, no custom config files for version switching.
- **User-space only**: install to `~/.dotnet` to avoid conflicts with system package managers.
- **Zero runtime dependencies**: ships as a single static Rust binary.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                      User                               │
│                                                         │
│   $ dsi install 9.0                                    │
│   $ dsi ls-remote --lts                                │
│   $ dotnet build        ← not our concern               │
└────────────┬────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────┐
│                     dsi (Rust binary)                  │
│                                                         │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │   Release    │  │   SDK        │  │   SDK         │  │
│  │   Index      │  │   Downloader │  │   Manager     │  │
│  │   Client     │  │   & Verifier │  │   (fs ops)    │  │
│  └──────┬──────┘  └──────┬───────┘  └──────┬────────┘  │
│         │                │                  │           │
└─────────┼────────────────┼──────────────────┼───────────┘
          │                │                  │
          ▼                ▼                  ▼
┌──────────────┐  ┌────────────────┐  ┌────────────────┐
│  Microsoft   │  │   Download     │  │   ~/.dotnet/   │
│  Releases    │  │   CDN          │  │   sdk/         │
│  API (JSON)  │  │   (tarballs)   │  │   shared/      │
└──────────────┘  └────────────────┘  └────────────────┘
                                             │
                                             ▼
                                      ┌────────────────┐
                                      │  dotnet CLI    │
                                      │  (muxer +      │
                                      │   hostfxr)     │
                                      │                │
                                      │  Handles:      │
                                      │  - global.json │
                                      │  - rollForward │
                                      │  - SDK resolve │
                                      └────────────────┘
```

---

## Scope

### What dsi does

| Responsibility | Description |
|---|---|
| **Discover** | Query Microsoft's release metadata API to list all available SDK versions with their support status, release dates, and EOL dates. |
| **Install** | Download SDK tarballs/zips from Microsoft's CDN, verify SHA512 checksums, and extract into `~/.dotnet/`. |
| **List** | Show locally installed SDK versions with enriched metadata (LTS/STS/EOL status, whether updates are available). |
| **Uninstall** | Remove specific SDK versions from `~/.dotnet/sdk/` and clean up orphaned shared runtimes. |
| **Prune** | Remove outdated patch versions, keeping only the latest patch per feature band. |

### What dsi does NOT do

| Not our concern | Why |
|---|---|
| Version switching / selection | `dotnet` CLI + `hostfxr` + `global.json` already handle this natively. |
| PATH manipulation | User adds `~/.dotnet` to PATH once during setup. Done. |
| Shims or proxies | No interception of `dotnet` commands. |
| global.json management | User creates these manually or via `dotnet new globaljson`. |
| Runtime-only installs | Scope is SDK management. SDKs bundle their matching runtimes. |
| .NET Framework (4.x) | Windows-only system component. Out of scope. |
| System-level installation | We never write to `/usr/share/dotnet`, `/usr/lib/dotnet`, or `C:\Program Files\dotnet`. |

---

## Installation Target

dsi always installs SDKs to the **user-level dotnet root**:

| OS | Install path | Notes |
|---|---|---|
| Linux | `~/.dotnet/` | Default for `dotnet-install.sh`, avoids conflict with apt/dnf/pacman |
| macOS | `~/.dotnet/` | Avoids conflict with Homebrew's `/usr/local/share/dotnet/` |
| Windows | `%LOCALAPPDATA%\Microsoft\dotnet\` | Standard user-level location, avoids conflict with VS installer at `%ProgramFiles%` |

### Why user-level, not system-level

System package managers (apt, dnf, pacman, brew) track file ownership in their system-level dotnet directories. Extracting files there without going through the package manager causes:

- **Phantom files**: package manager doesn't know about files dsi placed, so `apt remove` leaves orphans or `pacman -Qo` reports unknown ownership.
- **Overwrite conflicts**: a system package update may overwrite the shared `dotnet` muxer binary or `host/` directory, breaking SDK versions dsi installed.
- **Permission issues**: system paths require `sudo`, which is a security risk for a download tool.

User-level installation (`~/.dotnet/`) avoids all of these. The `dotnet` muxer at `~/.dotnet/dotnet` discovers all SDKs within its own root. System-installed SDKs at `/usr/share/dotnet/` remain untouched and independently managed.

### PATH setup

The user must ensure `~/.dotnet` is on their `PATH`. dsi provides a `setup` command to assist:

```bash
# dsi adds this to the appropriate shell profile
export DOTNET_ROOT="$HOME/.dotnet"
export PATH="$HOME/.dotnet:$HOME/.dotnet/tools:$PATH"
```

If the user also has a system-level dotnet, `~/.dotnet` should come **first** in PATH so user-managed SDKs take precedence. dsi should detect this and warn during setup.

---

## Data Sources

### Microsoft Release Metadata API

All version discovery comes from Microsoft's public JSON API:

**Release index** (all channels):
```
https://builds.dotnet.microsoft.com/dotnet/release-metadata/releases-index.json
```

Returns:
```json
{
  "releases-index": [
    {
      "channel-version": "10.0",
      "latest-release": "10.0.0",
      "latest-sdk": "10.0.100",
      "release-type": "lts",
      "support-phase": "active",
      "eol-date": "2028-11-10",
      "releases.json": "https://builds.dotnet.microsoft.com/dotnet/release-metadata/10.0/releases.json"
    }
  ]
}
```

**Per-channel releases** (all patches for a major.minor):
```
https://builds.dotnet.microsoft.com/dotnet/release-metadata/9.0/releases.json
```

Returns detailed info per release including:
- All SDK versions in that release
- Download URLs per platform (linux-x64, linux-arm64, osx-x64, osx-arm64, win-x64, etc.)
- SHA512 checksums
- Runtime versions bundled with each SDK

### SDK Download URLs

SDK tarballs follow a predictable pattern:
```
https://builds.dotnet.microsoft.com/dotnet/Sdk/{version}/dotnet-sdk-{version}-{os}-{arch}.tar.gz
```

dsi resolves the exact URL from the releases JSON — never constructs URLs manually.

---

## File System Layout

After installing several SDKs via dsi, `~/.dotnet/` looks like:

```
~/.dotnet/
├── dotnet                              # Muxer executable
├── LICENSE.txt
├── ThirdPartyNotices.txt
├── host/
│   └── fxr/
│       ├── 8.0.8/                      # hostfxr for .NET 8
│       └── 9.0.0/                      # hostfxr for .NET 9
├── sdk/
│   ├── 8.0.300/                        # SDK 8.0.300 (installed by dsi)
│   ├── 8.0.404/                        # SDK 8.0.404 (installed by dsi)
│   └── 9.0.100/                        # SDK 9.0.100 (installed by dsi)
├── sdk-manifests/
├── shared/
│   ├── Microsoft.NETCore.App/
│   │   ├── 8.0.8/
│   │   └── 9.0.0/
│   └── Microsoft.AspNetCore.App/
│       ├── 8.0.8/
│       └── 9.0.0/
├── packs/
├── templates/
└── tools/                              # Global tools (dotnet tool install -g)
```

dsi does not maintain its own metadata directory or database. The filesystem IS the source of truth — just like how `dotnet --list-sdks` works.

---

## CLI Interface

### Commands

#### `dsi setup`

First-time setup. Detects shell, adds PATH entries to profile.

```bash
$ dsi setup

Detected shell: zsh
Added to ~/.zshrc:
  export DOTNET_ROOT="$HOME/.dotnet"
  export PATH="$HOME/.dotnet:$HOME/.dotnet/tools:$PATH"

⚠  System dotnet found at /usr/share/dotnet/dotnet
   Your PATH is configured so ~/.dotnet takes precedence.

Restart your shell or run: source ~/.zshrc
```

#### `dsi ls-remote`

List all available SDK versions from the releases API.

```bash
$ dsi ls-remote

Channel   Latest SDK   Release Type   Support Phase   EOL Date
──────────────────────────────────────────────────────────────────
10.0      10.0.100     LTS            active          2028-11-10
9.0       9.0.203      STS            active          2026-11-08
8.0       8.0.404      LTS            active          2026-11-10
7.0       7.0.410      STS            eol             2024-05-14
6.0       6.0.428      LTS            eol             2024-11-12

# Show all patches for a specific channel
$ dsi ls-remote 9.0

SDK Version   Release Date   Runtime    Security
──────────────────────────────────────────────────
9.0.203       2025-03-11     9.0.3      yes
9.0.202       2025-02-11     9.0.2      yes
9.0.200       2025-01-14     9.0.1      yes
9.0.104       2025-03-11     9.0.3      yes
9.0.103       2025-02-11     9.0.2      yes
9.0.100       2024-11-12     9.0.0      no
...

# Filter by support type
$ dsi ls-remote --lts
$ dsi ls-remote --sts
$ dsi ls-remote --include-eol
$ dsi ls-remote --include-preview
```

#### `dsi install`

Download and install an SDK version.

```bash
# Install latest SDK for a channel
$ dsi install 9.0
Resolving latest SDK for channel 9.0...
Downloading dotnet-sdk-9.0.203-linux-x64.tar.gz (218 MB)
[████████████████████████████████████████] 100%
Verifying SHA512 checksum... ✓
Extracting to ~/.dotnet/... ✓
Installed SDK 9.0.203

# Install a specific SDK version
$ dsi install 8.0.300
Downloading dotnet-sdk-8.0.300-linux-x64.tar.gz (215 MB)
[████████████████████████████████████████] 100%
Verifying SHA512 checksum... ✓
Extracting to ~/.dotnet/... ✓
Installed SDK 8.0.300

# Install latest LTS
$ dsi install --lts

# Install latest of any supported channel
$ dsi install --latest

# Skip confirmation
$ dsi install 9.0 -y

# Cross-platform download (e.g., for deployment to ARM server)
$ dsi install 9.0 --rid linux-arm64 --download-only
Downloaded: ./dotnet-sdk-9.0.203-linux-arm64.tar.gz

# Already installed
$ dsi install 9.0.203
SDK 9.0.203 is already installed.
```

#### `dsi ls`

List locally installed SDK versions with enriched metadata.

```bash
$ dsi ls

Installed SDKs (in ~/.dotnet/sdk/):

SDK Version   Channel   Type   Status        Update Available
──────────────────────────────────────────────────────────────
9.0.203       9.0       STS    ✓ current     —
8.0.404       8.0       LTS    ✓ current     —
8.0.300       8.0       LTS    ✓ outdated    → 8.0.404

Active SDK: 9.0.203 (no global.json found, using latest)
```

The "Active SDK" line is determined by running `dotnet --version` — dsi doesn't resolve this itself.

#### `dsi uninstall`

Remove a specific SDK version.

```bash
$ dsi uninstall 8.0.300
Remove SDK 8.0.300 from ~/.dotnet/sdk/8.0.300? [y/N] y
Removed SDK 8.0.300 ✓

# Note: shared runtimes are NOT removed automatically
# because other SDKs may depend on them.
```

#### `dsi prune`

Remove outdated patch versions, keeping only the latest patch per feature band.

```bash
$ dsi prune --dry-run

Would remove:
  8.0.300  (newer in same feature band: 8.0.303)
  8.0.301  (newer in same feature band: 8.0.303)
  9.0.100  (newer in same feature band: 9.0.104)

Would keep:
  8.0.303  (latest in 8.0.3xx)
  8.0.404  (latest in 8.0.4xx)
  9.0.104  (latest in 9.0.1xx)
  9.0.203  (latest in 9.0.2xx)

$ dsi prune
Removed 3 outdated SDK versions. Freed 1.2 GB.
```

#### `dsi info`

Show environment info for debugging.

```bash
$ dsi info

dsi version:        0.1.0
Platform:           linux-x64
Install location:   /home/mo/.dotnet
Dotnet muxer:       /home/mo/.dotnet/dotnet (v9.0.3)
System dotnet:      /usr/share/dotnet/dotnet (v8.0.8)
PATH precedence:    ~/.dotnet takes priority ✓
Shell:              zsh
Profile:            ~/.zshrc (dsi PATH configured ✓)
```

---

## Use Cases

### Use Case 1: Fresh machine setup

A developer sets up a new Linux workstation and needs .NET for development.

```bash
# 1. Install dsi (single binary, curl or package manager)
curl -fsSL https://dsi.dev/install.sh | sh

# 2. Set up PATH
dsi setup

# 3. See what's available
dsi ls-remote --lts

# 4. Install what they need
dsi install --lts
dsi install 9.0

# 5. Verify
dotnet --list-sdks
```

**Before dsi**: visit website → find download → download tarball → extract → edit .bashrc → source.
**After dsi**: three commands.

### Use Case 2: Working on a project that requires an older SDK

A developer clones a repo with a `global.json` pinning SDK 8.0.300.

```bash
$ cd legacy-project
$ dotnet build
# Error: SDK 8.0.300 not found

$ dsi install 8.0.300
# Downloads and extracts

$ dotnet build
# Works — dotnet CLI reads global.json and finds 8.0.300
```

### Use Case 3: Checking for SDK updates

```bash
$ dsi ls

SDK Version   Channel   Type   Status        Update Available
──────────────────────────────────────────────────────────────
9.0.100       9.0       STS    ✓ outdated    → 9.0.203
8.0.300       8.0       LTS    ✓ outdated    → 8.0.404

$ dsi install 9.0
Installed SDK 9.0.203

$ dsi install 8.0
Installed SDK 8.0.404

$ dsi prune
Removed 9.0.100 and 8.0.300. Freed 890 MB.
```

### Use Case 4: CI/CD pipeline

```yaml
# GitHub Actions example
steps:
  - name: Install dsi
    run: curl -fsSL https://dsi.dev/install.sh | sh

  - name: Install required SDK
    run: dsi install 9.0 -y

  - name: Build
    run: dotnet build
```

Lighter alternative to `actions/setup-dotnet` for simple cases.

### Use Case 5: Exploring a preview release

```bash
$ dsi ls-remote --include-preview

Channel   Latest SDK        Release Type   Support Phase
──────────────────────────────────────────────────────────
11.0      11.0.100-preview.3  —            preview
10.0      10.0.100            LTS          active
...

$ dsi install 11.0
Downloading dotnet-sdk-11.0.100-preview.3-linux-x64.tar.gz...
Installed SDK 11.0.100-preview.3

# Try it out without affecting other projects
$ mkdir preview-test && cd preview-test
$ dotnet new globaljson --sdk-version 11.0.100-preview.3
$ dotnet new console
$ dotnet run
```

### Use Case 6: Coexistence with system package manager

The user installed .NET 8 via pacman on Arch/CachyOS, and needs .NET 9 for a side project.

```bash
# System dotnet (managed by pacman, at /usr/share/dotnet)
$ pacman -Q dotnet-sdk
dotnet-sdk-8.0 8.0.300-1

# Install .NET 9 via dsi (goes to ~/.dotnet, no conflict)
$ dsi install 9.0

$ dotnet --list-sdks
8.0.300 [/usr/share/dotnet/sdk]    # pacman's
9.0.203 [/home/mo/.dotnet/sdk]     # dsi's

# Both coexist. dotnet CLI sees both.
# global.json or "latest wins" determines which is active.
```

### Use Case 7: Working inside WSL

The user develops in WSL2 on Windows. They have .NET installed on both the Windows side (via Visual Studio) and need .NET inside WSL for CLI builds.

```bash
# Check environment
$ dsi info
Platform:       linux-x64 (glibc)
WSL:            WSL2 (distro: Ubuntu)
Windows dotnet: /mnt/c/Program Files/dotnet/dotnet (v8.0.8)
                ⚠  This is separate from your WSL Linux .NET

# dsi correctly downloads the Linux build, not Windows
$ dsi install 9.0
Downloading dotnet-sdk-9.0.203-linux-x64.tar.gz...
Installed SDK 9.0.203

# The Windows .NET at /mnt/c/... is untouched
# WSL Linux .NET lives at ~/.dotnet/
$ dotnet --version
9.0.203
```

dsi never downloads the Windows SDK when running inside WSL — it detects Linux and acts accordingly. The Windows-side .NET installation is completely independent.

---

## Internal Modules

### 1. Release Index Client

**Responsibility**: fetch and parse Microsoft's release metadata.

- Fetches `releases-index.json` once and caches it locally (with TTL, e.g., 1 hour).
- Fetches per-channel `releases.json` on demand.
- Resolves "9.0" → latest SDK version in that channel.
- Resolves "--lts" → latest LTS channel → latest SDK.
- Detects current OS and architecture for download URL selection.

**Cache location**: `~/.dotnet/.dsi-cache/` (inside dotnet root, or `$XDG_CACHE_HOME/dsi/`).

### 2. SDK Downloader & Verifier

**Responsibility**: download SDK archives and verify integrity.

- Downloads `.tar.gz` (Linux/macOS) or `.zip` (Windows) from Microsoft's CDN.
- Shows progress bar with download speed and ETA.
- Verifies SHA512 checksum against the value from the releases JSON.
- Aborts and cleans up on checksum mismatch.
- Supports resumable downloads (HTTP Range headers) for large files.

### 3. SDK Manager (Filesystem Operations)

**Responsibility**: extract, list, and remove SDK installations.

- Extracts archives into `~/.dotnet/` (merges with existing content — this is how .NET side-by-side works).
- Lists installed SDKs by scanning `~/.dotnet/sdk/` directories.
- Removes SDK versions by deleting `~/.dotnet/sdk/<version>/` directory.
- Prune logic: parse version strings, group by feature band (`x.y.zxx`), keep latest patch per band.
- Does NOT touch shared runtimes during uninstall (other SDKs may depend on them).

### 4. Platform Detection

**Responsibility**: detect OS, architecture, and runtime environment for download URL resolution.

This is a critical module. The wrong detection means downloading an SDK that won't run.

#### Detection Sources

| Property | How to detect in Rust | Notes |
|---|---|---|
| OS | `std::env::consts::OS` → `linux`, `macos`, `windows` | Compile-time constant |
| Architecture | `std::env::consts::ARCH` → `x86_64`, `aarch64`, `arm`, `x86` | Compile-time constant |
| C library (Linux) | Runtime: check ELF interpreter or `ldd --version` | musl vs glibc |
| WSL | Runtime: check env vars and `/proc/version` | Affects install path guidance |

#### OS + Architecture → Microsoft RID Mapping

| `std::env::consts` | Libc | Microsoft RID | Archive format |
|---|---|---|---|
| `linux` + `x86_64` | glibc | `linux-x64` | `.tar.gz` |
| `linux` + `x86_64` | musl | `linux-musl-x64` | `.tar.gz` |
| `linux` + `aarch64` | glibc | `linux-arm64` | `.tar.gz` |
| `linux` + `aarch64` | musl | `linux-musl-arm64` | `.tar.gz` |
| `linux` + `arm` | glibc | `linux-arm` | `.tar.gz` |
| `macos` + `x86_64` | — | `osx-x64` | `.tar.gz` |
| `macos` + `aarch64` | — | `osx-arm64` | `.tar.gz` |
| `windows` + `x86_64` | — | `win-x64` | `.zip` |
| `windows` + `x86` | — | `win-x86` | `.zip` |
| `windows` + `aarch64` | — | `win-arm64` | `.zip` |

#### musl vs glibc Detection (Linux only)

Most Linux distros use glibc. Alpine, Void Linux (musl variant), and some embedded/container distros use musl. .NET ships separate musl builds because the libc is not interchangeable.

Detection strategy (in order of reliability):

1. **Check the ELF interpreter of the current binary**: read `/proc/self/exe` ELF headers. If the interpreter path contains `musl` (e.g., `/lib/ld-musl-x86_64.so.1`), it's musl.
2. **Check `ldd --version` output**: glibc prints version info to stdout; musl prints to stderr and includes "musl".
3. **Check for `/lib/ld-musl-*.so.*`**: if this file exists, the system has musl.

```rust
fn detect_libc() -> Libc {
    // Method 1: check if ldd mentions musl
    if let Ok(output) = Command::new("ldd").arg("--version").output() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("musl") {
            return Libc::Musl;
        }
    }
    // Method 2: check for musl dynamic linker
    if Path::new("/lib/ld-musl-x86_64.so.1").exists()
        || Path::new("/lib/ld-musl-aarch64.so.1").exists()
    {
        return Libc::Musl;
    }
    Libc::Glibc
}
```

#### WSL Detection (Windows Subsystem for Linux)

WSL is a special case. Rust's `std::env::consts::OS` returns `"linux"` inside WSL because it IS a Linux environment. The Linux SDK build is correct for running inside WSL — so **dsi should download the Linux SDK, not the Windows SDK**. This is the right default behavior.

However, dsi should still detect WSL for two reasons:

1. **Install path guidance**: in WSL, `~/.dotnet` is on the Linux filesystem. If the user also has .NET on the Windows side (`/mnt/c/Program Files/dotnet/`), dsi should detect and warn about potential confusion.
2. **Info command**: `dsi info` should report WSL status so the user understands their environment.

Detection strategy:

```rust
#[derive(Debug)]
enum WslVersion {
    None,
    Wsl1,
    Wsl2,
}

fn detect_wsl() -> WslVersion {
    // Method 1: WSL_DISTRO_NAME is set by WSL
    if std::env::var("WSL_DISTRO_NAME").is_ok() {
        // Method 2: distinguish WSL1 vs WSL2
        if let Ok(version) = std::fs::read_to_string("/proc/version") {
            if version.contains("microsoft-standard-WSL2")
                || version.contains("WSL2")
            {
                return WslVersion::Wsl2;
            }
            if version.to_lowercase().contains("microsoft") {
                return WslVersion::Wsl1;
            }
        }
        // WSL_DISTRO_NAME set but can't determine version
        return WslVersion::Wsl2; // safe default, WSL2 is far more common
    }

    // Method 3: fallback — check /proc/version for Microsoft string
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        if version.to_lowercase().contains("microsoft") {
            return WslVersion::Wsl2;
        }
    }

    WslVersion::None
}
```

**WSL-specific behaviors:**

| Scenario | dsi behavior |
|---|---|
| Running inside WSL | Download `linux-x64` (or `linux-arm64`). This is correct. |
| Windows dotnet found at `/mnt/c/Program Files/dotnet/` | `dsi info` warns: "Windows .NET detected at /mnt/c/... — this is separate from your WSL Linux .NET." |
| User explicitly wants Windows SDK | `dsi install 9.0 --rid win-x64` — manual override for cross-download |
| WSL1 specific | Warn that WSL1 has known limitations with .NET; recommend WSL2 |

#### `--rid` Override Flag

For cross-download scenarios (downloading an SDK for a different platform):

```bash
# On a linux-x64 machine, download for linux-arm64 (for deployment to ARM server)
$ dsi install 9.0 --rid linux-arm64

# Inside WSL, download the Windows SDK (for some reason)
$ dsi install 9.0 --rid win-x64

# On macOS arm64, download x64 for Rosetta testing
$ dsi install 9.0 --rid osx-x64
```

When `--rid` is used, dsi downloads to a separate directory to avoid polluting the native SDK root:

```
~/.dotnet/                          # native SDKs (auto-detected platform)
~/.dotnet-cross/linux-arm64/        # cross-downloaded SDKs
```

Or alternatively, just download the archive to the current directory without extracting:

```bash
$ dsi install 9.0 --rid linux-arm64 --download-only
Downloaded: ./dotnet-sdk-9.0.203-linux-arm64.tar.gz
```

This is simpler and avoids managing a second dotnet root.

#### Updated `dsi info` Output with Platform Details

```bash
$ dsi info

dsi version:        0.1.0
Platform:           linux-x64 (glibc)
Architecture:       x86_64
OS:                 Linux (CachyOS)
WSL:                no
Install location:   /home/mo/.dotnet
Dotnet muxer:       /home/mo/.dotnet/dotnet (v9.0.3)
System dotnet:      /usr/share/dotnet/dotnet (v8.0.8)
PATH precedence:    ~/.dotnet takes priority ✓
Shell:              zsh
Profile:            ~/.zshrc (dsi PATH configured ✓)
```

Inside WSL:

```bash
$ dsi info

dsi version:        0.1.0
Platform:           linux-x64 (glibc)
Architecture:       x86_64
OS:                 Linux (Ubuntu 22.04)
WSL:                WSL2 (distro: Ubuntu)
Install location:   /home/mo/.dotnet
Dotnet muxer:       /home/mo/.dotnet/dotnet (v9.0.3)
Windows dotnet:     /mnt/c/Program Files/dotnet/dotnet (v8.0.8)
                    ⚠  This is separate from your WSL Linux .NET
PATH precedence:    ~/.dotnet takes priority ✓
Shell:              bash
Profile:            ~/.bashrc (dsi PATH configured ✓)
```

### 5. Shell Integration

**Responsibility**: first-time PATH setup via `dsi setup`.

- Detects current shell: bash, zsh, fish, PowerShell, nushell.
- Appends `DOTNET_ROOT` and `PATH` exports to the appropriate profile file.
- Detects if a system-level dotnet exists and warns about PATH ordering.
- Idempotent — does not duplicate entries on repeated runs.

---

## Rust Crate Dependencies

| Crate | Purpose |
|---|---|
| `clap` | CLI argument parsing and help generation |
| `reqwest` | HTTP client for API queries and SDK downloads |
| `tokio` | Async runtime for network operations |
| `serde` + `serde_json` | Parse releases API JSON and global.json |
| `flate2` + `tar` | Extract `.tar.gz` archives (Linux/macOS) |
| `zip` | Extract `.zip` archives (Windows) |
| `sha2` | SHA512 checksum verification |
| `indicatif` | Progress bars and spinners |
| `dirs` | Cross-platform home directory detection |
| `os_info` | OS distribution name detection (CachyOS, Ubuntu, Alpine, etc.) |
| `semver` | Version parsing and comparison (adapted for .NET's scheme) |
| `colored` | Terminal color output |

---

## Edge Cases & Considerations

### Muxer version conflicts
When extracting a newer SDK, the `dotnet` muxer binary and `host/fxr/` files get overwritten with the newer version. This is by design — .NET's muxer is backward compatible. A newer muxer can run older SDKs. However, extracting an **older** SDK over a **newer** muxer is safe too — the extraction will not downgrade the muxer because the older tarball's muxer has a lower version, and dsi should skip overwriting the muxer if the existing one is newer.

### Partial extraction recovery
If extraction is interrupted (power loss, Ctrl+C), the SDK directory may be incomplete. dsi should detect this on next run (e.g., check for a sentinel file or validate expected directory structure) and offer to re-install.

### Disk space
Each SDK is roughly 200-400 MB extracted. dsi should show the download size before confirming install, and `dsi prune` should report freed space.

### Offline usage
`dsi ls` works fully offline (filesystem scan). `dsi ls-remote` and `dsi install` require network access. Cached release index data allows `ls-remote` to work with stale data if the network is unavailable (with a warning).

### Permission errors
If `~/.dotnet/` was previously created by a root process or system installer, the user may not have write permission. dsi should detect this early and provide a clear error message rather than failing mid-extraction.