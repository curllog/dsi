# dsi

A fast, simple command-line tool for installing and managing **.NET SDK** versions on Linux, macOS, and WSL.

Everything installs into your home folder (`~/.dotnet/`), so you never need `sudo`.

## Table of Contents

- [Why dsi?](#why-dsi)
- [How it works (and what it does *not* touch)](#how-it-works-and-what-it-does-not-touch)
- [Installation](#installation)
- [Commands](#commands)
  - [See what's available](#see-whats-available)
  - [Install an SDK](#install-an-sdk)
  - [Keep things up to date](#keep-things-up-to-date)
  - [Clean up](#clean-up)
  - [Manage dsi itself](#manage-dsi-itself)
- [IMPORTANT](#important)

## Why dsi?

Installing the .NET SDK on Unix-like systems is surprisingly fiddly. There isn't one clean way to do it — you end up choosing between:

- **Package managers** (`apt`, `dnf`, `yum`, …) — these often lag behind the latest releases, and mixing Microsoft's repository with your distro's own .NET packages is a well-known source of breakage. The two install to *different* roots (`/usr/share/dotnet` vs `/usr/lib/dotnet`), and once they collide you can end up with a `dotnet` that won't run at all.
- **`dotnet-install.sh`** — Microsoft's official script is really aimed at CI: it's a large bash script meant for throwaway, non-persistent installs, and it leaves PATH and version management up to you.
- **Manual tarballs** — download, extract, set `DOTNET_ROOT`, fix your PATH, repeat for every version. Easy to get wrong.

`dsi` exists to do **one small job well**: download and install the SDK you ask for into your user folder. That's it. Doing a small thing reliably matters here precisely *because* the .NET install story on Unix is so messy — a tool you can trust to just put the right SDK in the right place removes most of the pain.

## How it works (and what it does *not* touch)

`dsi` does not replace or interfere with .NET's own version resolution. It **only downloads SDKs and unpacks them** side by side under `~/.dotnet/sdk/`.

Once the files are on disk, the regular `dotnet` host takes over exactly as it normally would:

- without a `global.json`, `dotnet` uses the latest installed SDK;
- with a `global.json`, `dotnet` picks the version it specifies.

`dsi` writes no config, sets no magic environment variables, and rewrites no resolution logic — so anything you already know about `global.json` and `dotnet --list-sdks` keeps working unchanged.

## Installation


```sh
curl -fsSL https://raw.githubusercontent.com/curllog/dsi/main/install.sh | sh
```

This installs `dsi` to `~/.local/bin/dsi` and adds `dsi` + `~/.dotnet` to your PATH
for every shell it detects (bash, zsh, fish, PowerShell, nushell). **Restart your
shell** (or `source` your profile) afterwards so the `dsi` command is available.


Supported platforms: Linux (glibc & musl) and macOS, on x64 and arm64. WSL is treated as Linux.

## Commands

Here are the basic things you can do with `dsi`:

### See what's available

```sh
dsi info          # show your platform, OS, and environment details
dsi ls-remote     # list .NET SDK versions you can install
dsi ls            # list the SDKs you already have installed
```

Tip: `dsi ls-remote --lts` shows only Long-Term Support releases.

### Install an SDK

```sh
dsi install 9.0         # install the latest patch for the 9.0 channel
dsi install 9.0.313     # install one exact version
dsi install --lts       # install the latest LTS release
dsi install --latest    # install the newest release available
```

### Keep things up to date

```sh
dsi update        # update your installed SDKs to the latest patch in each channel
```

### Clean up

```sh
dsi uninstall 9.0.313   # remove one specific SDK version
dsi prune               # remove older patches, keep the newest in each band
dsi prune --dry-run     # preview what prune would remove, without deleting
```

### Manage dsi itself

```sh
dsi selfupdate      # update dsi to the latest release
dsi selfuninstall   # remove the dsi binary (your installed SDKs stay)
```

## IMPORTANT

- The minimum installable version is **.NET 6.0**.
