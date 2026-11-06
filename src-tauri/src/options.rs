//! Last options used per operation (`options.json` in the config folder).

use convertio_core::OptionValues;
use std::{collections::HashMap, path::Path};

pub fn load_all(path: &Path) -> HashMap<String, OptionValues> {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn get(path: &Path, operation_id: &str) -> OptionValues {
    load_all(path).remove(operation_id).unwrap_or_default()
}

pub fn remember(path: &Path, operation_id: &str, options: OptionValues) -> std::io::Result<()> {
    let mut all = load_all(path);
    all.insert(operation_id.to_owned(), options);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.part");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&all).map_err(std::io::Error::other)?)?;
    std::fs::rename(tmp, path)
}
