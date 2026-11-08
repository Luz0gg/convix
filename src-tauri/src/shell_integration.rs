//! Windows Explorer context menu ("clic droit › Convix › Convertir en PNG").
//!
//! Classic cascading menu under
//! `HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell\Convix`, built from the
//! operation registry. HKCU only: no admin rights. On Windows 11 it shows under
//! "Afficher plus d'options" (see docs/adr/0003-menu-contextuel.md).
//! Each entry runs `Convix.exe --op <operation-id> "<file>"`; the single-instance
//! plugin funnels multi-file selections into the running window.

use convix_core::{Format, Registry};

/// Extensions that get the menu (every spelling Explorer may see).
/// Keep `src-tauri/windows/hooks.nsh` (uninstaller) in sync.
pub const EXTENSIONS: &[&str] = &[
    // images
    "png", "jpg", "jpeg", "jfif", "webp", "gif", "bmp", "tif", "tiff", "ico", "tga", "svg",
    // audio (menu shown once FFmpeg is installed)
    "mp3", "wav", "flac", "m4a", "aac", "ogg", "opus", "aif", "aiff",
    // video: extract the audio
    "mp4", "m4v", "mov", "mkv", "webm", "avi",
    // documents
    "pdf",
];

/// Tells whether an external module (ffmpeg…) is installed.
pub type HasModule<'a> = &'a dyn Fn(&str) -> bool;

const ROOT: &str = r"Software\Classes\SystemFileAssociations";
const VERB: &str = "Convix";
/// Menus written by earlier builds under the old code name: removed on install/uninstall.
const LEGACY_VERBS: &[&str] = &["ConvertIO"];

/// (operation id, label) shown for a format, most useful first.
pub fn menu_entries(
    registry: &Registry,
    format: Format,
    lang: &str,
    has_module: HasModule,
) -> Vec<(String, String)> {
    let fr = lang != "en";
    // Only actions that can run right now (an audio entry without FFmpeg would just fail).
    let ops: Vec<_> = registry
        .operations_for(format)
        .into_iter()
        .filter(|o| o.requires.iter().all(|m| has_module(m)))
        .collect();
    let mut entries = Vec::new();
    let convert_label = |target: Format| {
        if fr {
            format!("Convertir en {}", target.label())
        } else {
            format!("Convert to {}", target.label())
        }
    };

    for target in [Format::Png, Format::Jpeg, Format::Webp, Format::Ico] {
        let id = format!("image.convert.{}", target.extension());
        if ops.iter().any(|o| o.id == id) {
            entries.push((id, convert_label(target)));
        }
    }
    for target in [Format::Mp3, Format::Wav, Format::Flac] {
        let id = format!("audio.convert.{}", target.extension());
        if ops.iter().any(|o| o.id == id) {
            entries.push((id, convert_label(target)));
        }
    }
    // Videos: re-encode first (most asked for by video editors), then extract the sound.
    for (id, fr_label, en_label) in [
        ("video.convert.mp4", "Convertir en MP4", "Convert to MP4"),
        ("video.compress", "Compresser la vidéo", "Compress video"),
    ] {
        if ops.iter().any(|o| o.id == id) {
            entries.push((id.to_owned(), if fr { fr_label } else { en_label }.to_owned()));
        }
    }
    for target in [Format::Mp3, Format::Wav] {
        let id = format!("audio.extract.{}", target.extension());
        if ops.iter().any(|o| o.id == id) {
            let label = if fr {
                format!("Extraire l'audio ({})", target.label())
            } else {
                format!("Extract audio ({})", target.label())
            };
            entries.push((id, label));
        }
    }
    for (id, fr_label, en_label) in [
        (
            "image.compress.smaller",
            "Compresser (plus léger)",
            "Compress (smaller)",
        ),
        (
            "image.compress.lossless",
            "Compresser sans perte",
            "Compress losslessly",
        ),
        ("pdf.fromImages", "Convertir en PDF", "Convert to PDF"),
        ("pdf.compress", "Compresser le PDF", "Compress PDF"),
        ("pdf.toImages.png", "Pages en PNG", "Pages to PNG"),
        ("pdf.toImages.jpg", "Pages en JPEG", "Pages to JPEG"),
        ("pdf.split", "Séparer les pages", "Split pages"),
        ("video.convert.gif", "Convertir en GIF", "Convert to GIF"),
    ] {
        if ops.iter().any(|o| o.id == id) {
            entries.push((
                id.to_owned(),
                if fr { fr_label } else { en_label }.to_owned(),
            ));
        }
    }
    entries
}

#[cfg(windows)]
mod imp {
    use super::*;
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    fn io(e: std::io::Error) -> String {
        e.to_string()
    }

    pub fn install_at(
        root: &str,
        registry: &Registry,
        lang: &str,
        has_module: HasModule,
    ) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(io)?;
        let exe = exe.to_string_lossy();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let open_label = if lang == "en" {
            "Open in Convix"
        } else {
            "Ouvrir dans Convix"
        };

        for ext in EXTENSIONS {
            let Some(format) = Format::from_extension(ext) else {
                continue;
            };
            let entries = menu_entries(registry, format, lang, has_module);
            let base_path = format!(r"{root}\.{ext}\shell\{VERB}");
            let _ = hkcu.delete_subkey_all(&base_path); // rebuild from scratch: exe path or labels may have changed
            for old in LEGACY_VERBS {
                let _ = hkcu.delete_subkey_all(format!(r"{root}\.{ext}\shell\{old}"));
            }
            if entries.is_empty() {
                continue;
            }

            let (base, _) = hkcu.create_subkey(&base_path).map_err(io)?;
            base.set_value("MUIVerb", &"Convix").map_err(io)?;
            base.set_value("Icon", &format!("{exe},0")).map_err(io)?;
            base.set_value("SubCommands", &"").map_err(io)?; // = cascading menu from the `shell` subkey

            // Explorer sorts sub-verbs by key name: number them to keep our order.
            let all = entries
                .iter()
                .map(|(id, l)| (format!("--op {id} "), l.as_str()))
                .chain([(String::new(), open_label)]);
            for (i, (args, label)) in all.enumerate() {
                let (verb, _) = base.create_subkey(format!(r"shell\{i:02}")).map_err(io)?;
                verb.set_value("MUIVerb", &label).map_err(io)?;
                verb.set_value("MultiSelectModel", &"Player").map_err(io)?; // allow >15 selected files
                if args.is_empty() {
                    verb.set_value("CommandFlags", &0x20u32).map_err(io)?; // separator before "Open in…"
                }
                let (cmd, _) = verb.create_subkey("command").map_err(io)?;
                cmd.set_value("", &format!("\"{exe}\" {args}\"%1\""))
                    .map_err(io)?;
            }
        }
        Ok(())
    }

    pub fn uninstall_at(root: &str) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for ext in EXTENSIONS {
            for verb in std::iter::once(&VERB).chain(LEGACY_VERBS) {
                let _ = hkcu.delete_subkey_all(format!(r"{root}\.{ext}\shell\{verb}"));
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
pub fn install(registry: &Registry, lang: &str, has_module: HasModule) -> Result<(), String> {
    imp::install_at(ROOT, registry, lang, has_module)
}

#[cfg(windows)]
pub fn uninstall() -> Result<(), String> {
    imp::uninstall_at(ROOT)
}

#[cfg(not(windows))]
pub fn install(_: &Registry, _: &str, _: HasModule) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn uninstall() -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod sync_tests {
    /// The uninstaller (NSIS, can't read our Rust list) must clean every extension we write.
    #[test]
    fn uninstaller_lists_every_extension() {
        let nsh = include_str!("../windows/hooks.nsh");
        for ext in super::EXTENSIONS {
            for verb in std::iter::once(&super::VERB).chain(super::LEGACY_VERBS) {
                let line = format!(r"SystemFileAssociations\.{ext}\shell\{verb}");
                assert!(nsh.contains(&line), "hooks.nsh is missing {line}");
            }
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    /// Writes under a throwaway test key (never the real Explorer one), reads back, cleans up.
    #[test]
    fn writes_cascading_menu() {
        let root = format!(r"Software\Convix-test-{}", std::process::id());
        let mut registry = Registry::new();
        registry.register(convix_image::ImageConverter);
        registry.register(convix_audio::AudioConverter);
        registry.register(convix_audio::VideoConverter);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // Without FFmpeg: no audio menu at all.
        imp::install_at(&root, &registry, "fr", &|_| false).unwrap();
        assert!(
            hkcu.open_subkey(format!(r"{root}\.mp3\shell\Convix"))
                .is_err()
        );

        imp::install_at(&root, &registry, "fr", &|m| m == "ffmpeg").unwrap();
        let base = hkcu
            .open_subkey(format!(r"{root}\.webp\shell\Convix"))
            .unwrap();
        let first: String = base
            .open_subkey(r"shell\00")
            .unwrap()
            .get_value("MUIVerb")
            .unwrap();
        assert_eq!(first, "Convertir en PNG");
        let cmd: String = base
            .open_subkey(r"shell\00\command")
            .unwrap()
            .get_value("")
            .unwrap();
        assert!(cmd.contains("--op image.convert.png \"%1\""), "{cmd}");
        let video = hkcu
            .open_subkey(format!(r"{root}\.mov\shell\Convix"))
            .unwrap();
        let first: String = video
            .open_subkey(r"shell\00")
            .unwrap()
            .get_value("MUIVerb")
            .unwrap();
        assert_eq!(first, "Convertir en MP4");

        imp::uninstall_at(&root).unwrap();
        assert!(
            hkcu.open_subkey(format!(r"{root}\.webp\shell\Convix"))
                .is_err()
        );
        assert!(
            hkcu.open_subkey(format!(r"{root}\.mov\shell\Convix"))
                .is_err()
        );
        let _ = hkcu.delete_subkey_all(&root);
    }
}
