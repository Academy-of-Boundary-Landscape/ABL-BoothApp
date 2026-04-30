// LAN HTTPS 自签证书：生成、缓存、加载。
//
// 缓存文件位于 <app_data_dir>:
//   cert.pem        证书
//   key.pem         私钥 (PKCS#8)
//   cert.meta.json  {generated_at, valid_until, sans} —— 用来快速判断是否需要重生成，
//                   不必解析证书本身

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use rcgen::{
    CertificateParams, DistinguishedName, DnType, Ia5String, KeyPair, SanType,
    PKCS_ECDSA_P256_SHA256,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::Path;
use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::fs;

const CERT_FILENAME: &str = "cert.pem";
const KEY_FILENAME: &str = "key.pem";
const META_FILENAME: &str = "cert.meta.json";
const VALIDITY_YEARS: i64 = 10;
// 离过期还剩多少天就重生成（防止运行中跨过期时间）
const RENEW_THRESHOLD_DAYS: i64 = 30;

#[derive(Debug, thiserror::Error)]
pub enum CertError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("rcgen: {0}")]
    Rcgen(#[from] rcgen::Error),
    #[error("serde_json: {0}")]
    SerdeJson(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize)]
struct CertMeta {
    generated_at: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    sans: Vec<String>,
}

/// 加载缓存证书，必要时重新生成。
/// 返回 (cert_pem, key_pem) 字节流。
pub async fn load_or_generate_cert(
    app_data_dir: &Path,
    lan_ips: &[IpAddr],
) -> Result<(Vec<u8>, Vec<u8>), CertError> {
    let cert_path = app_data_dir.join(CERT_FILENAME);
    let key_path = app_data_dir.join(KEY_FILENAME);
    let meta_path = app_data_dir.join(META_FILENAME);

    // 看缓存是否能继续用
    if cert_path.exists() && key_path.exists() && meta_path.exists() {
        if let Ok(meta_bytes) = fs::read(&meta_path).await {
            if let Ok(meta) = serde_json::from_slice::<CertMeta>(&meta_bytes) {
                if cert_meta_still_valid(&meta, lan_ips) {
                    let cert_pem = fs::read(&cert_path).await?;
                    let key_pem = fs::read(&key_path).await?;
                    eprintln!(
                        "[cert] reusing cached cert (valid until {})",
                        meta.valid_until
                    );
                    return Ok((cert_pem, key_pem));
                } else {
                    eprintln!(
                        "[cert] cached cert no longer covers current LAN IPs or expired — regenerating"
                    );
                }
            }
        }
    } else {
        eprintln!("[cert] no cached cert — generating fresh");
    }

    let (cert_pem, key_pem, meta) = generate_self_signed(lan_ips)?;
    fs::create_dir_all(app_data_dir).await?;
    fs::write(&cert_path, &cert_pem).await?;
    fs::write(&key_path, &key_pem).await?;
    fs::write(&meta_path, serde_json::to_vec_pretty(&meta)?).await?;
    eprintln!(
        "[cert] generated new cert, valid until {}, SANs: {:?}",
        meta.valid_until, meta.sans
    );
    Ok((cert_pem, key_pem))
}

fn cert_meta_still_valid(meta: &CertMeta, current_ips: &[IpAddr]) -> bool {
    let now = Utc::now();
    let renew_deadline = meta.valid_until - ChronoDuration::days(RENEW_THRESHOLD_DAYS);
    if now >= renew_deadline {
        return false;
    }
    // 当前所有 LAN IP 都必须在 SAN 里；否则换网络后旧证书覆盖不到，需要重生成。
    //
    // 故意不对称：SAN 里残留了已断开网卡的旧 IP **不**触发重生成。理由：
    // 浏览器仅校验"它连的那个 IP 是否在 SAN 列表中"——多余的 SAN 不会让任何客户端报错；
    // 而每次重生成都会让所有已接受过证书的设备再看一次安全警告，体验糟糕。
    // 所以"宁多勿少"是正确的策略，请勿误改成对称比较。
    let san_set: std::collections::HashSet<&str> =
        meta.sans.iter().map(|s| s.as_str()).collect();
    for ip in current_ips {
        let ip_str = ip.to_string();
        if !san_set.contains(ip_str.as_str()) {
            return false;
        }
    }
    true
}

fn generate_self_signed(
    lan_ips: &[IpAddr],
) -> Result<(Vec<u8>, Vec<u8>, CertMeta), CertError> {
    // 收集 SAN：传入的 LAN IP + 标准 loopback + DNS 名
    let mut sans: Vec<SanType> = Vec::new();
    let mut sans_strs: Vec<String> = Vec::new();

    let loopback_v4: IpAddr = "127.0.0.1".parse().unwrap();
    let loopback_v6: IpAddr = "::1".parse().unwrap();
    sans.push(SanType::IpAddress(loopback_v4));
    sans_strs.push(loopback_v4.to_string());
    sans.push(SanType::IpAddress(loopback_v6));
    sans_strs.push(loopback_v6.to_string());

    for ip in lan_ips {
        // 去重 + 跳过环回（已加过）
        let s = ip.to_string();
        if sans_strs.contains(&s) {
            continue;
        }
        sans.push(SanType::IpAddress(*ip));
        sans_strs.push(s);
    }

    let localhost_dns = Ia5String::try_from("localhost".to_string())
        .expect("'localhost' is valid IA5");
    sans.push(SanType::DnsName(localhost_dns));
    sans_strs.push("localhost".to_string());

    let mut params = CertificateParams::default();
    params.subject_alt_names = sans;

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "booth-kernel-self-signed");
    params.distinguished_name = dn;

    let now_time = OffsetDateTime::now_utc();
    let valid_until_time = now_time + TimeDuration::days(365 * VALIDITY_YEARS);
    params.not_before = now_time;
    params.not_after = valid_until_time;

    let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
    let cert = params.self_signed(&key_pair)?;
    let cert_pem = cert.pem().into_bytes();
    let key_pem = key_pair.serialize_pem().into_bytes();

    // CertMeta 用 chrono 存储（人类可读 ISO8601 序列化）
    let now_chrono = Utc::now();
    let valid_until_chrono = now_chrono + ChronoDuration::days(365 * VALIDITY_YEARS);
    let meta = CertMeta {
        generated_at: now_chrono,
        valid_until: valid_until_chrono,
        sans: sans_strs,
    };

    Ok((cert_pem, key_pem, meta))
}
