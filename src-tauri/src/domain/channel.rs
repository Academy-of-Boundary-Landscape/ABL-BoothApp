//! 收款渠道名的规范化。
//!
//! **渠道名会成为账户名的一部分**（`实收-<渠道>`，见 `domain/ledger.rs` 的 `Account`），
//! 一旦写进 `money_movements` 就没有任何办法清理——账户不是一张表，是一堆字符串。
//! 所以规范化必须发生在**写库之前**，而不是显示的时候。
//!
//! ②-1 与 ②-2 的交接段都点名了这件事（「channel 目前是无约束自由文本」）。
//! 三件套：规范化、长度上限、给前端一个已用渠道列表让摊主从已有的里挑。
//! 第三条在 `api/settlement.rs` 的 `GET /channels`，才是真正防「微信」和
//! 「微信支付」分裂成两个账户的那一条。
//!
//! **不加白名单**——母 spec 第 5 节明写「渠道可自定义（有社团用银行转账、有的用闲鱼）」。

use crate::error::{ApiError, ApiResult};

/// 渠道名最长 20 个**字符**（不是字节）。20 个汉字 = 60 字节，按字节限长
/// 会让中文渠道名只能写 6 个字。
pub const MAX_CHANNEL_CHARS: usize = 20;

/// 预置渠道。母 spec 第 5 节：「至少预置现金/微信/支付宝三种」。
pub const PRESET_CHANNELS: [&str; 3] = ["现金", "微信", "支付宝"];

/// 规范化：首尾去空白，内部连续空白（含制表与换行）折叠成一个半角空格。
///
/// 折叠而不是禁止，是因为「银行 转账」这种中间带空格的写法是合理的，
/// 而「银行&nbsp;&nbsp;转账」和「银行 转账」必须是同一个账户。
pub fn normalize(raw: &str) -> ApiResult<String> {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return Err(ApiError::BadRequest("收款渠道不能为空".into()));
    }
    let n = collapsed.chars().count();
    if n > MAX_CHANNEL_CHARS {
        return Err(ApiError::BadRequest(format!(
            "收款渠道名最长 {MAX_CHANNEL_CHARS} 个字，当前 {n} 个"
        )));
    }
    Ok(collapsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_collapses_inner_whitespace() {
        assert_eq!(normalize("  微信  ").unwrap(), "微信");
        assert_eq!(normalize("银行 \t 转账").unwrap(), "银行 转账");
        assert_eq!(normalize("换\n行").unwrap(), "换 行");
    }

    #[test]
    fn rejects_blank() {
        // 空渠道名会造出账户 "实收-"，写进 money_movements 之后没有任何办法清理
        assert!(normalize("").is_err());
        assert!(normalize("   ").is_err());
        assert!(normalize("\t\n").is_err());
    }

    #[test]
    fn length_limit_counts_chars_not_bytes() {
        // 20 个汉字是 60 字节。按字节限长会让中文渠道名只能写 6 个字。
        let twenty = "一二三四五六七八九十一二三四五六七八九十";
        assert_eq!(twenty.chars().count(), 20);
        assert!(normalize(twenty).is_ok());
        assert!(normalize(&format!("{twenty}超")).is_err());
    }

    #[test]
    fn keeps_hyphens_intact() {
        // Account::from_str 用 strip_prefix 取剩余全部，所以带横线的渠道名能原样解回来。
        // 这条钉住的是「不要为了好解析而禁掉横线」。
        assert_eq!(normalize("自定义-渠道").unwrap(), "自定义-渠道");
    }
}
