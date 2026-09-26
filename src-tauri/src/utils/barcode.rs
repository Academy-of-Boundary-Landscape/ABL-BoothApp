//! 商业条码（JAN/EAN-13、ISBN 等）的规范化。
//!
//! 只做形态清洗，**不校验校验位**：校验位算法有好几种（EAN-13 / UPC-A / Code128…），
//! 手贴标签的内容也不受约束，误拦比放过更烦人。

/// 规范化扫描/手输得到的条码文本。
///
/// - 去掉首尾空格（含各类 Unicode 空白）。
/// - 去掉后为空 → `None`（存 NULL，而不是空串）。
/// - 若只由 ASCII 数字、空格、`-` 组成 → 去掉所有空格与 `-`（手敲的 `978-4-…`）。
/// - 否则原样返回（已 trim）——`Code128` 等可能含字母。
pub fn normalize_barcode(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed
        .chars()
        .all(|c| c.is_ascii_digit() || c == ' ' || c == '-')
    {
        let cleaned: String = trimmed.chars().filter(|c| *c != ' ' && *c != '-').collect();
        // 纯分隔符（如 "-" 或 " - "）：清洗后没有内容，和空串一样存 NULL。
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_barcode;

    #[test]
    fn strips_separators_from_digit_only_codes() {
        assert_eq!(
            normalize_barcode(" 978-4-06-123456-7 "),
            Some("9784061234567".to_string())
        );
    }

    #[test]
    fn keeps_non_digit_codes_verbatim_after_trim() {
        assert_eq!(normalize_barcode("ab-12 C"), Some("ab-12 C".to_string()));
    }

    #[test]
    fn blank_becomes_none() {
        assert_eq!(normalize_barcode("  "), None);
    }

    #[test]
    fn separator_only_becomes_none() {
        assert_eq!(normalize_barcode(" - - "), None);
    }
}
