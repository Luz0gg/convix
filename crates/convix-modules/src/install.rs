//! Download → SHA-256 check → extraction. Nothing is unpacked before the hash matches.

use crate::{ModuleAsset, ModuleError, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufReader, Read, Write},
    path::Path,
    time::Duration,
};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InstallPhase {
    Downloading,
    Verifying,
    Extracting,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub phase: InstallPhase,
    pub done: u64,
    pub total: u64,
}

fn agent() -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls) // Windows' SChannel and certificate store
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .timeout_connect(Some(Duration::from_secs(20)))
        // Per read: a stalled connection fails instead of hanging forever.
        .timeout_recv_body(Some(Duration::from_secs(60)))
        .user_agent(concat!("Convix/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

pub(crate) fn download(
    url: &str,
    dest: &Path,
    expected_size: u64,
    progress: &dyn Fn(InstallProgress),
    cancelled: &dyn Fn() -> bool,
) -> Result<()> {
    let net = |e: ureq::Error| ModuleError::Network(e.to_string());
    let response = agent().get(url).call().map_err(net)?;
    // A little slack over the manifest size, never an unbounded read.
    let mut body = response.into_body().into_with_config().limit(expected_size + 1024 * 1024).reader();

    let mut file = File::create(dest)?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    loop {
        if cancelled() {
            return Err(ModuleError::Cancelled);
        }
        let n = body.read(&mut buf).map_err(|e| ModuleError::Network(e.to_string()))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        done += n as u64;
        progress(InstallProgress { phase: InstallPhase::Downloading, done, total: expected_size });
    }
    file.sync_all()?;
    Ok(())
}

pub(crate) fn verify(archive: &Path, asset: &ModuleAsset, progress: &dyn Fn(InstallProgress)) -> Result<()> {
    progress(InstallProgress { phase: InstallPhase::Verifying, done: 0, total: asset.size });
    let mut reader = BufReader::new(File::open(archive)?);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut size = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    let hash: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if size != asset.size || !hash.eq_ignore_ascii_case(&asset.sha256) {
        return Err(ModuleError::Checksum);
    }
    Ok(())
}

/// Unpack the files the manifest asks for into `dest` (flat), returns the entry's file name.
pub(crate) fn extract(
    archive: &Path,
    asset: &ModuleAsset,
    dest: &Path,
    progress: &dyn Fn(InstallProgress),
    cancelled: &dyn Fn() -> bool,
) -> Result<String> {
    if asset.archive != "7z" {
        return Err(ModuleError::Archive(format!("type d'archive non géré : {}", asset.archive)));
    }
    std::fs::create_dir_all(dest)?;
    let wanted: Vec<&str> = std::iter::once(asset.entry.as_str()).chain(asset.files.iter().map(String::as_str)).collect();
    let file_name = |p: &str| p.rsplit(['/', '\\']).next().unwrap_or(p).to_owned();
    let normalize = |p: &str| p.replace('\\', "/");

    let mut reader = sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::empty())
        .map_err(|e| ModuleError::Archive(e.to_string()))?;
    let total = wanted.len() as u64;
    let mut found = 0u64;
    let mut io_error: Option<std::io::Error> = None;

    let result = reader.for_each_entries(|entry, data| {
        if cancelled() {
            return Ok(false);
        }
        let name = normalize(entry.name());
        if entry.is_directory() || !wanted.iter().any(|w| normalize(w) == name) {
            return Ok(true); // skipped (still decoded: 7z blocks are solid)
        }
        let out = dest.join(file_name(&name));
        let copy = File::create(&out).and_then(|mut f| std::io::copy(data, &mut f).map(|_| ()));
        if let Err(e) = copy {
            io_error = Some(e);
            return Ok(false);
        }
        found += 1;
        progress(InstallProgress { phase: InstallPhase::Extracting, done: found, total });
        Ok(found < total)
    });
    if let Some(e) = io_error {
        return Err(e.into());
    }
    if cancelled() {
        return Err(ModuleError::Cancelled);
    }
    result.map_err(|e| ModuleError::Archive(e.to_string()))?;

    let entry = file_name(&normalize(&asset.entry));
    if !dest.join(&entry).exists() {
        return Err(ModuleError::Archive(format!("{entry} introuvable dans l'archive")));
    }
    Ok(entry)
}
