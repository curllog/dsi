// src/version.rs
//
// Helpers for comparing and grouping .NET SDK version strings.

/// A comparable key for an SDK version.
///
/// Splits the numeric "major.minor.patch" portion into integers; any
/// pre-release suffix ("-preview.3") is dropped and the version flagged as
/// non-stable so that, for equal numeric parts, a stable release sorts above
/// its pre-releases (`true > false`).
pub fn version_key(version: &str) -> (Vec<u32>, bool) {
    let (numeric, is_stable) = match version.split_once('-') {
        Some((head, _)) => (head, false),
        None => (version, true),
    };
    let parts = numeric
        .split('.')
        .map(|p| p.parse::<u32>().unwrap_or(0))
        .collect();
    (parts, is_stable)
}

/// The "major.minor" channel of a full version string.
///   "9.0.312" → "9.0"
pub fn channel_of(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        version.to_string()
    }
}

/// The SDK feature band of a full version string.
///
/// .NET SDK patch numbers encode the feature band in their hundreds digit:
///   "8.0.300" / "8.0.303" → "8.0.3xx"
///   "8.0.404"             → "8.0.4xx"
///   "9.0.100"             → "9.0.1xx"
///
/// Versions that don't have a parseable third component fall back to the bare
/// version string so they're never grouped (and therefore never pruned)
/// against something they don't belong with.
pub fn feature_band(version: &str) -> String {
    let numeric = version.split('-').next().unwrap_or(version);
    let parts: Vec<&str> = numeric.split('.').collect();
    if parts.len() >= 3
        && let Ok(patch) = parts[2].parse::<u32>()
    {
        return format!("{}.{}.{}xx", parts[0], parts[1], patch / 100);
    }
    version.to_string()
}
