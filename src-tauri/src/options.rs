//! Last options used per operation (`options.json` in the config folder).

use convix_core::OptionValues;
use std::{collections::HashMap, path::Path};

pub fn load_all(path: &Path) -> HashMap<String, OptionValues> {
    crate::store::load(path)
}

pub fn get(path: &Path, operation_id: &str) -> OptionValues {
    load_all(path).remove(operation_id).unwrap_or_default()
}

pub fn remember(path: &Path, operation_id: &str, options: OptionValues) -> std::io::Result<()> {
    let mut all = load_all(path);
    all.insert(operation_id.to_owned(), options);
    crate::store::save(path, &all)
}
