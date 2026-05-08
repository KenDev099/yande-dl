use anyhow::Result;
use std::path::Path;
use tokio::fs;

/// Write `value` as pretty JSON to `path` atomically: write a sibling temp
/// file, then rename. If anything fails before the rename, the original
/// file (if any) is untouched.
pub async fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;

    // Use a sibling tmp filename. `with_extension("json.tmp")` preserves the
    // base name and just changes the extension.
    let tmp = path.with_extension("json.tmp");

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).await?;
        }
    }

    fs::write(&tmp, json).await?;
    fs::rename(&tmp, path).await?;
    Ok(())
}

/// Read JSON; if the file is missing, return `T::default()`.
pub async fn read_json_or_default<T>(path: &Path) -> Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    match fs::read_to_string(path).await {
        Ok(s) => Ok(serde_json::from_str(&s)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
    struct Sample {
        name: String,
        count: u32,
    }

    #[tokio::test]
    async fn round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");

        let v = Sample {
            name: "foo".into(),
            count: 7,
        };
        atomic_write_json(&path, &v).await.unwrap();

        let back: Sample = read_json_or_default(&path).await.unwrap();
        assert_eq!(back, v);
    }

    #[tokio::test]
    async fn missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does-not-exist.json");
        let v: Sample = read_json_or_default(&path).await.unwrap();
        assert_eq!(v, Sample::default());
    }

    #[tokio::test]
    async fn replaces_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");

        atomic_write_json(
            &path,
            &Sample {
                name: "v1".into(),
                count: 1,
            },
        )
        .await
        .unwrap();
        atomic_write_json(
            &path,
            &Sample {
                name: "v2".into(),
                count: 2,
            },
        )
        .await
        .unwrap();

        let back: Sample = read_json_or_default(&path).await.unwrap();
        assert_eq!(back.name, "v2");
        assert_eq!(back.count, 2);
    }

    #[tokio::test]
    async fn does_not_leave_tmp_artifact() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");

        atomic_write_json(
            &path,
            &Sample {
                name: "foo".into(),
                count: 1,
            },
        )
        .await
        .unwrap();

        let tmp = path.with_extension("json.tmp");
        assert!(!tmp.exists(), "tmp file should be gone after rename");
    }
}
