use std::collections::BTreeSet;
use std::net::IpAddr;

use network_interface::{Addr, NetworkInterface, NetworkInterfaceConfig};
use serde::Serialize;

/// One real host address that the Web server can bind, plus presentation
/// metadata shared by egui and the browser Preferences renderers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetworkBindTarget {
    pub value: String,
    pub label: &'static str,
    pub detail: String,
    pub icon: &'static str,
    pub interface_name: String,
    pub family: &'static str,
    pub internal: bool,
}

fn interface_presentation(name: &str, internal: bool) -> (&'static str, &'static str) {
    if internal {
        return ("Loopback interface", "loopback");
    }
    let normalized = name.to_ascii_lowercase();
    if normalized.contains("wi-fi")
        || normalized.contains("wifi")
        || normalized.contains("wlan")
        || normalized.starts_with("wl")
    {
        ("Wi-Fi interface", "wifi")
    } else if normalized.contains("ethernet")
        || normalized.starts_with("eth")
        || normalized.starts_with("en")
    {
        ("Ethernet interface", "ethernet")
    } else if normalized.contains("vpn")
        || normalized.contains("wireguard")
        || normalized.contains("tailscale")
        || normalized.contains("tunnel")
        || normalized.starts_with("wg")
        || normalized.starts_with("tun")
    {
        ("VPN or tunnel interface", "vpn")
    } else if normalized.contains("virtual")
        || normalized.contains("hyper-v")
        || normalized.contains("vmware")
        || normalized.contains("vbox")
        || normalized.contains("docker")
        || normalized.contains("vethernet")
    {
        ("Virtual interface", "virtual")
    } else {
        ("Network interface", "network")
    }
}

fn address_detail(interface: &NetworkInterface, address: Addr) -> String {
    let ip = address.ip();
    let mut detail = format!("{} — {ip}", interface.name);
    if let Some(mac) = interface
        .mac_addr
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        detail.push_str(&format!(" · {mac}"));
    }
    detail
}

/// Discover bindable addresses through native adapter enumeration on Windows,
/// Linux, FreeBSD, and macOS. The two wildcard choices are explicit because
/// they mean “future and current interfaces”, not one currently enumerated NIC.
pub fn discover_bind_targets() -> Vec<NetworkBindTarget> {
    let mut targets = vec![
        NetworkBindTarget {
            value: "0.0.0.0".to_string(),
            label: "All IPv4 interfaces",
            detail: "Every current and future IPv4 address".to_string(),
            icon: "globe",
            interface_name: "*".to_string(),
            family: "ipv4",
            internal: false,
        },
        NetworkBindTarget {
            value: "::".to_string(),
            label: "All IPv6 interfaces",
            detail: "Every current and future IPv6 address".to_string(),
            icon: "globe",
            interface_name: "*".to_string(),
            family: "ipv6",
            internal: false,
        },
    ];
    let Ok(interfaces) = NetworkInterface::show() else {
        targets.push(NetworkBindTarget {
            value: "127.0.0.1".to_string(),
            label: "Loopback interface",
            detail: "Loopback — 127.0.0.1".to_string(),
            icon: "loopback",
            interface_name: "loopback".to_string(),
            family: "ipv4",
            internal: true,
        });
        return targets;
    };
    let mut seen = BTreeSet::new();
    for interface in interfaces {
        let (label, icon) = interface_presentation(&interface.name, interface.internal);
        for address in interface.addr.iter().copied() {
            let ip = address.ip();
            if ip.is_unspecified() || ip.is_multicast() || !seen.insert(ip) {
                continue;
            }
            targets.push(NetworkBindTarget {
                value: ip.to_string(),
                label,
                detail: address_detail(&interface, address),
                icon,
                interface_name: interface.name.clone(),
                family: if ip.is_ipv4() { "ipv4" } else { "ipv6" },
                internal: interface.internal,
            });
        }
    }
    targets.sort_by(|left, right| {
        let rank = |target: &NetworkBindTarget| match target.value.as_str() {
            "0.0.0.0" => 0,
            "::" => 1,
            _ if target.internal => 3,
            _ => 2,
        };
        rank(left)
            .cmp(&rank(right))
            .then_with(|| left.interface_name.cmp(&right.interface_name))
            .then_with(|| left.value.cmp(&right.value))
    });
    targets
}

pub fn parse_bind_addresses(values: &[String]) -> Result<Vec<IpAddr>, String> {
    if values.is_empty() {
        return Err("web_listen_addresses must contain at least one address".to_string());
    }
    let mut parsed = Vec::with_capacity(values.len());
    for value in values {
        let ip = value
            .trim()
            .parse::<IpAddr>()
            .map_err(|_| format!("web_listen_addresses contains an invalid IP address: {value}"))?;
        if !parsed.contains(&ip) {
            parsed.push(ip);
        }
    }
    let wildcard_v4 = parsed.contains(&IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
    let wildcard_v6 = parsed.contains(&IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED));
    if wildcard_v4 || wildcard_v6 {
        parsed.retain(|ip| {
            (ip.is_ipv4() && (!wildcard_v4 || ip.is_unspecified()))
                || (ip.is_ipv6() && (!wildcard_v6 || ip.is_unspecified()))
        });
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_targets_are_always_available_and_iconized() {
        let targets = discover_bind_targets();
        let all_v4 = targets
            .iter()
            .find(|target| target.value == "0.0.0.0")
            .expect("IPv4 wildcard");
        assert_eq!(all_v4.icon, "globe");
        assert!(!all_v4.detail.is_empty());
    }

    #[test]
    fn bind_address_parser_rejects_names_and_deduplicates_addresses() {
        assert!(parse_bind_addresses(&[]).is_err());
        assert!(parse_bind_addresses(&["localhost".to_string()]).is_err());
        assert_eq!(
            parse_bind_addresses(&[
                "127.0.0.1".to_string(),
                "127.0.0.1".to_string(),
                "::1".to_string(),
            ])
            .unwrap(),
            vec![
                "127.0.0.1".parse::<IpAddr>().unwrap(),
                "::1".parse::<IpAddr>().unwrap(),
            ]
        );
    }
}
