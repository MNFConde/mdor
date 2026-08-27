use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};

/// 元数据读取上限（字节），远超正常元数据量级（§6.7 / D-02 纵深防御）。
pub const MAX_META_BYTES: u64 = 1024 * 1024;

/// 写入持久性分层（D-03 / §6.7）：按文件类型选择，不按平台分支。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// 低频高价值（`library.json`）：fsync 文件内容，断电后 rename 结果在盘。
    Fsync,
    /// 高频低价值（`progress.json`）/ 可重写（`.mdor/versions`）：仅 rename。
    RenameOnly,
}

/// 原子写 JSON：序列化 → 写 `*.tmp` →（可选 fsync）→ 同目录 rename 覆盖。
/// 要么旧文件要么新文件，无半写状态（Android/Linux 上 rename 原子）。
pub fn write_json_atomic<T: Serialize>(
    path: &Path,
    data: &T,
    durability: Durability,
) -> Result<()> {
    let tmp = tmp_path(path)?;
    let bytes = serde_json::to_vec_pretty(data).map_err(|e| Error::json(path, e))?;
    tracing::debug!(path = %path.display(), ?durability, bytes = bytes.len(), "原子写元数据");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }

    let mut file = fs::File::create(&tmp).map_err(|e| Error::io(&tmp, e))?;
    file.write_all(&bytes).map_err(|e| Error::io(&tmp, e))?;
    if durability == Durability::Fsync {
        file.sync_all().map_err(|e| Error::io(&tmp, e))?;
    }
    drop(file);

    fs::rename(&tmp, path).map_err(|e| Error::io(path, e))?;
    Ok(())
}

/// 读取 JSON，读前按字节数上限拦截（§6.7）。
/// 先 `metadata` 预检（快路径），再 `take(max + 1)` 实读校验（防预检后文件膨胀的 TOCTOU）。
pub fn read_json_capped<T: DeserializeOwned>(path: &Path, max: u64) -> Result<T> {
    let file = fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let declared = file.metadata().map_err(|e| Error::io(path, e))?.len();
    if declared > max {
        tracing::warn!(path = %path.display(), declared, max, "元数据文件超限拦截");
        return Err(Error::Capped {
            path: path.to_path_buf(),
            size: declared,
            max,
        });
    }

    let file = file;
    let mut buf = Vec::with_capacity(usize::try_from(declared.min(max)).unwrap_or(0));
    file.take(max + 1)
        .read_to_end(&mut buf)
        .map_err(|e| Error::io(path, e))?;
    if buf.len() as u64 > max {
        tracing::warn!(path = %path.display(), size = buf.len(), max, "元数据文件超限拦截（实读）");
        return Err(Error::Capped {
            path: path.to_path_buf(),
            size: buf.len() as u64,
            max,
        });
    }

    tracing::debug!(path = %path.display(), bytes = buf.len(), "读取元数据");
    serde_json::from_slice(&buf).map_err(|e| Error::json(path, e))
}

/// 同目录 `*.tmp` 路径（同目录保证 rename 原子）。
fn tmp_path(path: &Path) -> Result<PathBuf> {
    let name = path.file_name().ok_or_else(|| {
        Error::io(
            path,
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "路径无文件名"),
        )
    })?;
    let mut name = name.to_os_string();
    name.push(".tmp");
    Ok(path.with_file_name(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mdor-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("创建测试临时目录");
        dir
    }

    #[test]
    fn write_then_read_roundtrip() {
        let dir = temp_dir("write_roundtrip");
        let path = dir.join("meta.json");
        let data = HashMap::from([("a".to_string(), 1u32), ("b".to_string(), 2u32)]);

        write_json_atomic(&path, &data, Durability::Fsync).unwrap();
        let read: HashMap<String, u32> = read_json_capped(&path, MAX_META_BYTES).unwrap();

        assert_eq!(read, data);
        assert!(
            !path.with_extension("json.tmp").exists(),
            "不应残留 .tmp 文件"
        );
    }

    #[test]
    fn overwrite_existing_is_atomic() {
        let dir = temp_dir("overwrite");
        let path = dir.join("meta.json");
        write_json_atomic(&path, &"old", Durability::RenameOnly).unwrap();
        write_json_atomic(&path, &"new", Durability::RenameOnly).unwrap();

        let read: String = read_json_capped(&path, MAX_META_BYTES).unwrap();
        assert_eq!(read, "new");
    }

    #[test]
    fn nested_parent_dir_created() {
        let dir = temp_dir("nested");
        let path = dir.join("a/b/c/meta.json");
        write_json_atomic(&path, &1u32, Durability::Fsync).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn read_exceeding_limit_rejected() {
        let dir = temp_dir("capped");
        let path = dir.join("meta.json");
        let big = vec![0u8; 64];
        fs::write(&path, &big).unwrap();

        let err = read_json_capped::<Vec<u8>>(&path, 32).unwrap_err();
        match err {
            Error::Capped { size, max, .. } => {
                assert_eq!(size, 64);
                assert_eq!(max, 32);
            }
            other => panic!("期望 Capped 错误，实际 {other:?}"),
        }
    }

    #[test]
    fn read_corrupt_json_errors() {
        let dir = temp_dir("corrupt");
        let path = dir.join("meta.json");
        fs::write(&path, b"not-json{").unwrap();

        assert!(matches!(
            read_json_capped::<serde_json::Value>(&path, MAX_META_BYTES),
            Err(Error::Json { .. })
        ));
    }

    #[test]
    fn read_missing_file_is_io_notfound() {
        let dir = temp_dir("missing");
        let path = dir.join("nope.json");

        assert!(matches!(
            read_json_capped::<serde_json::Value>(&path, MAX_META_BYTES),
            Err(Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound
        ));
    }
}
