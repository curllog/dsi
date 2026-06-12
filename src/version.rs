// src/version.rs
//
// Helpers for comparing and grouping .NET SDK version strings.

/// One dot-separated identifier of a pre-release suffix.
///
/// Per semver, numeric identifiers compare numerically and sort below
/// alphanumeric ones (`Num` before `Str` in declaration order).
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum PreSegment {
    Num(u64),
    Str(String),
}

/// A comparable key for an SDK version.
///
/// Splits the numeric "major.minor.patch" portion into integers and flags the
/// version as stable or not, so that, for equal numeric parts, a stable
/// release sorts above its pre-releases (`true > false`). Among pre-releases
/// the suffix identifiers break the tie, so "-preview.5" > "-preview.4".
pub fn version_key(version: &str) -> (Vec<u32>, bool, Vec<PreSegment>) {
    let (numeric, is_stable, pre) = match version.split_once('-') {
        Some((head, tail)) => (head, false, tail),
        None => (version, true, ""),
    };
    let parts = numeric
        .split('.')
        .map(|p| p.parse::<u32>().unwrap_or(0))
        .collect();
    let pre_parts = pre
        .split('.')
        .filter(|s| !s.is_empty())
        .map(|s| match s.parse::<u64>() {
            Ok(n) => PreSegment::Num(n),
            Err(_) => PreSegment::Str(s.to_string()),
        })
        .collect();
    (parts, is_stable, pre_parts)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_sorts_above_its_prereleases() {
        assert!(version_key("11.0.100") > version_key("11.0.100-rc.1.26450.107"));
    }

    #[test]
    fn later_preview_sorts_above_earlier_preview() {
        assert!(
            version_key("11.0.100-preview.5.26302.115")
                > version_key("11.0.100-preview.4.26230.115")
        );
        assert!(
            version_key("11.0.100-rc.1.26450.107") > version_key("11.0.100-preview.5.26302.115")
        );
    }

    #[test]
    fn numeric_patch_comparison_still_wins() {
        assert!(version_key("10.0.301") > version_key("10.0.300"));
        assert!(version_key("10.0.300") > version_key("9.0.315"));
    }
}
