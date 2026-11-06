//! Small JSON files in the config folder (settings, remembered options).
//! Written atomically; a file that can't be read is kept aside (`*.bad.json`) and logged
//! instead of being silently overwritten with defaults.

use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

pub fn load<T: DeserializeOwned + Default>(path: &Path) -> T {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return T::default(), // first launch: nothing saved yet
    };
    match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            let backup = path.with_extension("bad.json");
            let _ = std::fs::rename(path, &backup);
            crate::log::error(&format!(
                "{} unreadable ({e}), defaults used; copy kept as {}",
                path.display(),
                backup.display()
            ));
            T::default()
        }
    }
}

pub fn save<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn corrupt_file_is_kept_aside() {
        let dir = std::env::temp_dir().join(format!("convix-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        std::fs::write(&path, b"{ not json").unwrap();
        let v: HashMap<String, u32> = load(&path);
        assert!(v.is_empty());
        assert!(!path.exists());
        assert_eq!(std::fs::read(dir.join("settings.bad.json")).unwrap(), b"{ not json");

        save(&path, &HashMap::from([("a".to_string(), 1u32)])).unwrap();
        let v: HashMap<String, u32> = load(&path);
        assert_eq!(v["a"], 1);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2, "no temp file left");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
