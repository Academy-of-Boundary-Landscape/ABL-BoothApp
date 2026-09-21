// src/utils/ip.rs

use local_ip_address::list_afinet_netifas;
use std::net::IpAddr;

// 黑名单：虚拟机/容器/VPN/隧道/Overlay 网络等不应作为对外 LAN 出口的网卡。
//
// 关于 "virtual"：保留它能捕获 VMware NAT、VirtualBox、Cisco AnyConnect Virtual
// Miniport、GlobalProtect 等 VPN 客户端虚拟适配器。
// Windows 移动热点的网卡（Microsoft Wi-Fi Direct **Virtual** Adapter）也含 "virtual"，
// 但我们在 should_include_iface 里**先**判热点关键字、**再**判黑名单，热点能豁免。
const NIC_BLACKLIST: &[&str] = &[
    // 虚拟机/容器
    "virtual",
    "vmware",
    "vmnet",
    "virtualbox",
    "vbox",
    "docker",
    "wsl",
    "vether",
    // VPN / 隧道
    "vpn",
    "tunnel",
    "tap-windows",
    "tap-",
    // Overlay 网络
    "tailscale",
    "zerotier",
    // Hyper-V 等其它伪接口
    "switch",
    "loopback pseudo",
];

// 真实物理网卡白名单
const NIC_WHITELIST: &[&str] = &["wlan", "wi-fi", "wireless", "eth", "en0", "en1", "ethernet"];

// 移动热点（Hosted Network）网卡名关键词。
// Windows: "Microsoft Wi-Fi Direct Virtual Adapter"
// 多语言系统下名字仍是英文（实测 zh-CN 也是英文）。
const NIC_HOTSPOT_KEYWORDS: &[&str] = &["wi-fi direct", "wifi direct", "hosted network"];

// Windows 移动热点的默认 host 段（192.168.137.0/24）。
// 作为名字识别失败时的 IP 段兜底——比如 Windows 未来某版改了适配器名，
// 至少 host IP 这个特征更难被改。
fn is_windows_hotspot_subnet(ip: &std::net::Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 192 && octets[1] == 168 && octets[2] == 137
}

fn name_matches(name: &str, kws: &[&str]) -> bool {
    let lower = name.to_lowercase();
    kws.iter().any(|k| lower.contains(k))
}

/// 决定一个网卡是否进 LAN 候选集。
/// 顺序：先看是否热点（豁免黑名单），再看黑名单。
fn should_include_iface(name: &str) -> bool {
    if name_matches(name, NIC_HOTSPOT_KEYWORDS) {
        return true;
    }
    !name_matches(name, NIC_BLACKLIST)
}

/// 选一个 IP 给 QR 码用。
///
/// 策略优先级（从高到低）：
/// 0. 活跃的热点适配器（名字含 Wi-Fi Direct / Hosted Network，或 IP 在 192.168.137.0/24）
/// 1. 白名单内的真实物理网卡（WiFi / Ethernet）
/// 2. 任何不在黑名单的非环回非链路本地 IPv4
/// 3. 任何非环回 IPv4（连虚拟机网卡也凑合用）
/// 4. 保底回环
pub fn get_lan_ip() -> String {
    let interfaces = list_afinet_netifas().unwrap_or(vec![]);

    // 策略 0: 热点优先（名字识别 + IP 段兜底）
    for (name, ip) in &interfaces {
        if let IpAddr::V4(ipv4) = ip {
            if ipv4.is_loopback() || ipv4.is_link_local() {
                continue;
            }
            if name_matches(name, NIC_HOTSPOT_KEYWORDS) || is_windows_hotspot_subnet(ipv4) {
                return ipv4.to_string();
            }
        }
    }

    // 策略 1: 白名单内、黑名单外的物理网卡
    for (name, ip) in &interfaces {
        if let IpAddr::V4(ipv4) = ip {
            if ipv4.is_loopback() || ipv4.is_link_local() {
                continue;
            }
            if name_matches(name, NIC_WHITELIST) && !name_matches(name, NIC_BLACKLIST) {
                return ipv4.to_string();
            }
        }
    }

    // 策略 2: 任何不在黑名单的非环回非链路本地 IPv4
    for (name, ip) in &interfaces {
        if let IpAddr::V4(ipv4) = ip {
            if ipv4.is_loopback() || ipv4.is_link_local() {
                continue;
            }
            if should_include_iface(name) {
                return ipv4.to_string();
            }
        }
    }

    // 策略 3: 实在没办法，随便选一个非回环（即便是虚拟机网卡）
    for (_, ip) in &interfaces {
        if let IpAddr::V4(ipv4) = ip {
            if !ipv4.is_loopback() {
                return ipv4.to_string();
            }
        }
    }

    "127.0.0.1".to_string()
}

/// 枚举所有适合作为 LAN 出口的 IPv4 网卡地址。
/// 用于自签证书的 SAN 列表。
///
/// 使用与 `get_lan_ip` 同一份豁免/黑名单逻辑，确保两者对"哪些 IP 算 LAN"判断一致——
/// 这样 cert_meta_still_valid 能正确判断证书是否需要重生成。
///
/// 169.254.x.x 链路本地地址被过滤掉——它们是 DHCP 失败的兜底，无法用于 LAN 通信，
/// 进 SAN 只是徒增噪音。
pub fn get_all_lan_ipv4_addrs() -> Vec<IpAddr> {
    let interfaces = list_afinet_netifas().unwrap_or(vec![]);

    let mut addrs: Vec<IpAddr> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (name, ip) in &interfaces {
        if let IpAddr::V4(ipv4) = ip {
            if ipv4.is_loopback() || ipv4.is_link_local() {
                continue;
            }
            // 兼顾名字判定 + 热点 IP 段兜底（让热点 IP 即使在异常命名下也进 SAN）
            let pass_name = should_include_iface(name);
            let is_hotspot_ip = is_windows_hotspot_subnet(ipv4);
            if !pass_name && !is_hotspot_ip {
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
