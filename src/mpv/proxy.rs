//! Keep libmpv's proxy aligned with Pealayer's persisted remote-media settings.
//!
//! mpv's `http-proxy` option only accepts proxy URLs beginning with `http://`.
//! It also cannot proxy every protocol, so URL inspection remains more capable
//! than playback for some proxy setups.

use libmpv2::{Mpv, MpvInitializer};

const PROXY_ENVIRONMENT_KEYS: [&str; 6] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
];

/// Return the proxy value that can actually be consumed by mpv.
///
/// A custom proxy is authoritative. When it is blank, environment values are
/// considered in HTTP-first order and the first mpv-compatible value wins.
pub fn playback_proxy(use_proxy: bool, custom_proxy: &str) -> Option<String> {
    if !use_proxy {
        return None;
    }

    let custom_proxy = custom_proxy.trim();
    if !custom_proxy.is_empty() {
        return is_mpv_compatible(custom_proxy).then(|| custom_proxy.to_string());
    }

    PROXY_ENVIRONMENT_KEYS.into_iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| is_mpv_compatible(value))
    })
}

pub fn apply_before_initialize(
    initializer: &MpvInitializer,
    use_proxy: bool,
    custom_proxy: &str,
) -> libmpv2::Result<()> {
    let proxy = playback_proxy(use_proxy, custom_proxy).unwrap_or_default();
    initializer.set_option("http-proxy", proxy.as_str())
}

pub fn apply_runtime(mpv: &Mpv, use_proxy: bool, custom_proxy: &str) -> Result<(), String> {
    let proxy = playback_proxy(use_proxy, custom_proxy).unwrap_or_default();
    mpv.set_property("options/http-proxy", proxy.as_str())
        .map_err(|error| format!("Could not apply the playback proxy: {error}"))
}

fn is_mpv_compatible(proxy: &str) -> bool {
    proxy
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_proxy_must_match_mpv_http_constraint() {
        assert_eq!(
            playback_proxy(true, " HTTP://127.0.0.1:8080 ").as_deref(),
            Some("HTTP://127.0.0.1:8080")
        );
        assert_eq!(playback_proxy(true, "https://proxy.example"), None);
        assert_eq!(playback_proxy(false, "http://127.0.0.1:8080"), None);
    }
}
