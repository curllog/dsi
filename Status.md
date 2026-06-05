# dsi — implementation status

Companion to `dsi-architecture.md`. The architecture doc describes the design;
this file tracks what's actually built so far.

## Scope (locked in)

- Linux, macOS, WSL only. Native Windows is NOT supported (gated with
  `compile_error!` in `src/main.rs`).
- Install location: `~/.dotnet/` (user-level only — never `/usr/share/dotnet`).
- Minimum installable .NET version: 6.0. Older versions require OpenSSL 1.x
  which modern distros don't ship.
- TLS: `rustls-tls` (no system OpenSSL dependency).
- Error type: `use anyhow::Result as AnyResult;` is the project convention.
- No unit tests yet. Explicit decision to add later.

## Commands implemented

| Command | Status | Notes |
|---|---|---|
| `dsi info` | ✅ done | Shows platform/OS/WSL/libc detection. |
| `dsi ls-remote` | ✅ done | Fetches Microsoft's release index. Supports `--lts`, `--include-eol`. |
| `dsi ls` | ✅ done | Lists SDKs from `~/.dotnet/sdk/`. Shows active SDK via `~/.dotnet/dotnet --version`. |
| `dsi install <ver>` | ✅ done | Full pipeline: resolve → fetch → download → SHA-512 verify → tar.gz extract. Supports channel ("9.0"), exact version ("9.0.313"), `--lts`, `--latest`. Tested end-to-end. |
| `dsi update` | ✅ done | Groups installed SDKs by channel, finds the channel's latest SDK, installs it if newer than the highest installed patch. Confirms (`[Y/n]`) unless `--yes`/`-y`. Reuses `install::fetch_and_install`. |
| `dsi uninstall <ver>` | ✅ done | Removes `~/.dotnet/sdk/<version>/`. Confirms before deleting unless `--yes`/`-y`. Leaves shared runtimes in place. |
| `dsi prune` | ✅ done | Groups SDKs by feature band (e.g. 9.0.1xx), keeps the latest patch per band, removes the rest. Reports freed space. Supports `--dry-run` and `--yes`/`-y`. |
| `dsi selfupdate` | 🟡 stub | Deliberate stub: bails with "not implemented yet". Release/asset convention is still undecided and must be settled with the release CI + `install.sh`. See order note below. |
| `dsi selfuninstall` | ✅ done | Removes the running binary (`std::env::current_exe()`), leaves `~/.dotnet/` SDKs intact. Confirms (`[y/N]`) unless `--yes`/`-y`. |

## Out-of-tree work pending

- **`install.sh`** — the one-shot installer that downloads the dsi binary and
  configures PATH. Should detect bash, zsh, fish, PowerShell, nushell, and add
  marker-fenced PATH exports to each profile. PATH configuration was deliberately
  kept out of `dsi install` and centralized here.

## File layout

```
dsi/
├── Cargo.toml
├── dsi-architecture.md             # design document
├── STATUS.md                       # this file
└── src/
    ├── main.rs                     # CLI entry, subcommand dispatch, error printing
    ├── platform.rs                 # OS/arch/libc/WSL detection, RID construction
    ├── paths.rs                    # DsiPaths: dotnet_root, dotnet_bin, sdk_dir
    ├── prompt.rs                   # confirm(): shared y/n prompt for destructive cmds
    ├── version.rs                  # version_key, channel_of, feature_band helpers
    ├── api/
    │   ├── mod.rs
    │   ├── models.rs               # serde structs: ReleasesIndex, Channel,
    │   │                           # ChannelReleases, Release, Sdk, SdkFile
    │   │                           # plus impl Channel { is_lts, is_supported,
    │   │                           # major_version, is_installable }
    │   │                           # plus const MIN_SUPPORTED_MAJOR_VERSION = 6
    │   └── client.rs               # ApiClient: fetch_releases_index,
    │                               # fetch_channel_releases, download
    │                               # plus fn find_sdk_file_for_rid
    └── commands/
        ├── mod.rs
        ├── info.rs                 # ✅ done
        ├── ls_remote.rs            # ✅ done
        ├── ls.rs                   # ✅ done
        ├── install.rs              # ✅ done
        ├── update.rs               # ✅ done
        ├── uninstall.rs            # ✅ done
        ├── prune.rs                # ✅ done
        ├── selfupdate.rs           # 🟡 deliberate stub (release convention undecided)
        └── selfuninstall.rs        # ✅ done
```

## Technical decisions and gotchas worth remembering

These are things that took time to figure out during the build. Don't re-discover
them.

1. **`std::io::copy(&mut file, &mut hasher)` does NOT work with `sha2::Sha512`.**
   `Sha512` doesn't implement `std::io::Write` — it implements `digest::Digest`.
   `verify_sha512` uses a manual `read` + `hasher.update` loop with an 8 KB
   buffer. See `src/commands/install.rs`.

2. **Microsoft's `SdkFile.name` field omits the version number** (returns
   "dotnet-sdk-linux-x64.tar.gz" with no version). Don't trust it for the local
   filename. The install command builds its own filename:
   `format!("dsi-download-{}-{}.tar.gz", sdk.version, rid)`. Archive is deleted
   after successful extraction.

3. **The `Channel.releases_json_url` field uses
   `#[serde(rename = "releases.json")]`** because the JSON key literally contains
   a dot. Struct-level `rename_all = "kebab-case"` doesn't handle dots — must be
   field-level.

4. **Sdk.files uses `#[serde(default)]`** so missing `files` arrays parse as
   empty vec instead of failing. Some older releases lack this field.

5. **`anyhow::Result` is imported as `AnyResult`** project-wide to avoid
   shadowing the std prelude's `Result`. Use `AnyResult<T>` in function
   signatures, never `Result<T, E>`.

6. **Native Windows is gated at compile time** in `src/main.rs` with a
   `compile_error!` block. Don't add Windows-specific code or restore Windows
   conditional branches that may have existed in earlier commits.

7. **PATH modification belongs in `install.sh`, not in `dsi install`.**
   The install command writes files to disk and stops. PATH config is a one-time
   setup performed by the bootstrap installer.

## Dependencies (Cargo.toml)

```
clap            — CLI parsing (derive feature)
tokio           — async runtime (macros, rt-multi-thread features)
reqwest         — HTTP client (json, rustls-tls features, default-features = false)
serde           — serialization (derive feature)
serde_json      — JSON support
anyhow          — error handling
dirs            — cross-platform paths
indicatif       — progress bars
futures-util    — Stream traits (for streaming downloads)
sha2            — SHA-512
hex             — hex encoding for hash comparison
flate2          — gzip
tar             — tar archives
```

## Suggested implementation order for remaining commands

1. **`uninstall`** — simplest. Read `~/.dotnet/sdk/<version>/`, confirm with
   user, `std::fs::remove_dir_all`. Mirror of `install`'s arg patterns.
2. **`update`** — reuses install logic. For each installed SDK, look up the
   latest patch in its channel; if newer, install it.
3. **`prune`** — pure filesystem + version-string logic. Group by feature band
   (parse `x.y.zNN` where NN identifies the band), keep the latest patch per
   band, delete the rest.
4. **`selfupdate`** — DEFERRED (stub bails for now). Release host is known
   (GitHub `curllog/dsi`), but the asset convention — raw `dsi-{rid}` binary vs
   `dsi-{rid}.tar.gz`, tag format, atomic-replace path — is intentionally left
   undecided until the release CI and `install.sh` are designed, so all three
   stay consistent. `SelfUpdateArgs` already carries a `--yes`/`-y` flag for the
   eventual implementation.
5. **`selfuninstall`** — DONE. Deletes the running binary via
   `std::env::current_exe()`, leaves SDKs alone.

Independent of these: write `install.sh`. Can be done at any point.
