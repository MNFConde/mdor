//! 版本 tag 语义（§7）：私有命名空间 `refs/mdor/versions/<seq>`，序号单调递增。
//!
//! 版本语义与 gix 机械操作分离：本模块为纯函数（无仓库依赖，可独立单测），
//! 仓库操作见 [`crate::store::snapshot`]。

/// 版本 tag 的引用名前缀（私有命名空间，避免与上游自身 tag 冲突，§7.1）。
pub const VERSIONS_REF_PREFIX: &str = "refs/mdor/versions/";

/// 由序号生成完整版本 tag 引用名（`v3` → `refs/mdor/versions/v3`）。
#[must_use]
pub fn version_tag_ref(seq: u32) -> String {
    format!("{VERSIONS_REF_PREFIX}v{seq}")
}

/// 从版本 tag 引用名解析序号（`refs/mdor/versions/v3` → `Some(3)`；非版本 ref → `None`）。
#[must_use]
pub fn version_seq_of(ref_name: &str) -> Option<u32> {
    let tail = ref_name.strip_prefix(VERSIONS_REF_PREFIX)?;
    tail.strip_prefix('v')?.parse().ok()
}

/// 下一个版本序号 = 现存最大序号 + 1；无现存版本时为 1。
#[must_use]
pub fn next_version_seq(existing_seqs: impl IntoIterator<Item = u32>) -> u32 {
    existing_seqs.into_iter().max().map_or(1, |m| m + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_ref_format() {
        assert_eq!(version_tag_ref(1), "refs/mdor/versions/v1");
        assert_eq!(version_tag_ref(42), "refs/mdor/versions/v42");
    }

    #[test]
    fn seq_parses_own_refs_only() {
        assert_eq!(version_seq_of("refs/mdor/versions/v1"), Some(1));
        assert_eq!(version_seq_of("refs/mdor/versions/v10"), Some(10));
        assert_eq!(version_seq_of("refs/tags/v1"), None);
        assert_eq!(version_seq_of("refs/mdor/versions/other"), None);
        assert_eq!(version_seq_of(""), None);
    }

    #[test]
    fn next_seq_after_max() {
        assert_eq!(next_version_seq([1, 3, 2]), 4);
        assert_eq!(next_version_seq([5]), 6);
        assert_eq!(next_version_seq([]), 1);
    }
}
