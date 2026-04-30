// src/utils/ip.rs

use local_ip_address::list_afinet_netifas;
use std::net::IpAddr;

pub fn get_lan_ip() -> String {
    // 获取所有网络接口 (名称, IP)
    let network_interfaces = list_afinet_netifas().unwrap_or(vec![]);

    // 定义黑名单关键词：这些通常是虚拟机的网卡，我们尽量不要选它们
    let blacklist = [
        "virtual", "vmware", "vbox", "docker", "wsl", "vether", "vpn", "switch",
    ];

    // 定义白名单高优先级关键词：这些通常是真实的物理网卡 (Wi-Fi 或 有线)
    // 根据你的 ipconfig，你的网卡名包含 "WLAN"，这会直接命中
    let whitelist = ["wlan", "wi-fi", "wireless", "eth", "en0", "en1", "ethernet"];

    // =========================================================================
    // 策略 1: 优先寻找白名单内的真实网卡 (比如 "WLAN", "Ethernet")
    // =========================================================================
    for (name, ip) in &network_interfaces {
        if let IpAddr::V4(ipv4) = ip {
            if ipv4.is_loopback() {
                continue;
            }

            let name_lower = name.to_lowercase();

            // 如果网卡名字包含 "wlan" 等关键词，且不包含 "virtual" 等黑名单词
            // 直接锁定，这 99% 是你要的 IP
            let is_whitelisted = whitelist.iter().any(|&k| name_lower.contains(k));
            let is_blacklisted = blacklist.iter().any(|&k| name_lower.contains(k));

            if is_whitelisted && !is_blacklisted {
                return ipv4.to_string();
            }
        }
    }

    // =========================================================================
    // 策略 2: 如果找不到白名单网卡，找任何不在黑名单里的网卡
    // =========================================================================
    for (name, ip) in &network_interfaces {
        if let IpAddr::V4(ipv4) = ip {
            if ipv4.is_loopback() {
                continue;
            }

            let name_lower = name.to_lowercase();
            // 只要名字不包含 vmware/virtualbox 等，就凑合用
            let is_blacklisted = blacklist.iter().any(|&k| name_lower.contains(k));

            if !is_blacklisted {
                return ipv4.to_string();
            }
        }
    }

    // =========================================================================
    // 策略 3: 实在没办法（比如只有虚拟机网卡开启），那只能选一个非回环的 IPv4
    // =========================================================================
    for (_, ip) in &network_interfaces {
        if let IpAddr::V4(ipv4) = ip {
            if !ipv4.is_loopback() {
                return ipv4.to_string();
            }
        }
    }

    // 保底
    "127.0.0.1".to_string()
}

/// 枚举所有非环回、非黑名单（虚拟机/Docker等）的 IPv4 网卡地址。
/// 用于自签证书的 SAN 列表。
pub fn get_all_lan_ipv4_addrs() -> Vec<IpAddr> {
    let interfaces = list_afinet_netifas().unwrap_or(vec![]);
    let blacklist = [
        "virtual", "vmware", "vbox", "docker", "wsl", "vether", "vpn", "switch",
    ];

    let mut addrs: Vec<IpAddr> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (name, ip) in &interfaces {
        if let IpAddr::V4(ipv4) = ip {
            if ipv4.is_loopback() {
                continue;
            }
            let name_lower = name.to_lowercase();
            if blacklist.iter().any(|&k| name_lower.contains(k)) {
                continue;
            }
            let s = ipv4.to_string();
            if seen.insert(s) {
                addrs.push(IpAddr::V4(*ipv4));
            }
        }
    }
    addrs
}
