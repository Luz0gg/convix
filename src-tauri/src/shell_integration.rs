//! Windows Explorer context menu ("clic droit › ConvertIO › Convertir en PNG").
//!
//! Classic cascading menu under
//! `HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell\ConvertIO`, built from the
//! operation registry. HKCU only: no admin rights. On Windows 11 it shows under
//! "Afficher plus d'options" (see docs/adr/0003-menu-contextuel.md).
//! Each entry runs `ConvertIO.exe --op <operation-id> "<file>"`; the single-instance
//! plugin funnels multi-file selections into the running window.

use convertio_core::{Format, Registry};

/// Extensions that get the menu (every spelling Explorer may see).
pub const EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "jfif", "webp", "gif", "bmp", "tif", "tiff", "ico", "tga", "svg",
];

const ROOT: &str = r"Software\Classes\SystemFileAssociations";
const VERB: &str = "ConvertIO";

/// (operation id, label) shown for a format, most useful first.
pub fn menu_entries(registry: &Registry, format: Format, lang: &str) -> Vec<(String, String)> {
    let fr = lang != "en";
    let ops = registry.operations_for(format);
    let mut entries = Vec::new();

    for target in [Format::Png, Format::Jpeg, Format::Webp, Format::Ico] {
        let id = format!("image.convert.{}", target.extension());
        if ops.iter().any(|o| o.id == id) {
            let label = if fr { format!("Convertir en {}", target.label()) } else { format!("Convert to {}", target.label()) };
            entries.push((id, label));
        }
    }
    for (id, fr_label, en_label) in [
        ("image.compress.smaller", "Compresser (plus léger)", "Compress (smaller)"),
        ("image.compress.lossless", "Compresser sans perte", "Compress losslessly"),
    ] {
        if ops.iter().any(|o| o.id == id) {
            entries.push((id.to_owned(), if fr { fr_label } else { en_label }.to_owned()));
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

    pub fn install_at(root: &str, registry: &Registry, lang: &str) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(io)?;
        let exe = exe.to_string_lossy();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let open_label = if lang == "en" { "Open in ConvertIO" } else { "Ouvrir dans ConvertIO" };

        for ext in EXTENSIONS {
            let Some(format) = Format::from_extension(ext) else { continue };
            let entries = menu_entries(registry, format, lang);
            let base_path = format!(r"{root}\.{ext}\shell\{VERB}");
            let _ = hkcu.delete_subkey_all(&base_path); // rebuild from scratch: exe path or labels may have changed
            if entries.is_empty() {
                continue;
            }

            let (base, _) = hkcu.create_subkey(&base_path).map_err(io)?;
            base.set_value("MUIVerb", &"ConvertIO").map_err(io)?;
            base.set_value("Icon", &format!("{exe},0")).map_err(io)?;
            base.set_value("SubCommands", &"").map_err(io)?; // = cascading menu from the `shell` subkey

            // Explorer sorts sub-verbs by key name: number them to keep our order.
            let all = entries.iter().map(|(id, l)| (format!("--op {id} "), l.as_str())).chain([(String::new(), open_label)]);
            for (i, (args, label)) in all.enumerate() {
                let (verb, _) = base.create_subkey(format!(r"shell\{i:02}")).map_err(io)?;
                verb.set_value("MUIVerb", &label).map_err(io)?;
                verb.set_value("MultiSelectModel", &"Player").map_err(io)?; // allow >15 selected files
                if args.is_empty() {
                    verb.set_value("CommandFlags", &0x20u32).map_err(io)?; // separator before "Open in…"
                }
                let (cmd, _) = verb.create_subkey("command").map_err(io)?;
                cmd.set_value("", &format!("\"{exe}\" {args}\"%1\"")).map_err(io)?;
            }
        }
        Ok(())
    }

    pub fn uninstall_at(root: &str) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for ext in EXTENSIONS {
            let _ = hkcu.delete_subkey_all(format!(r"{root}\.{ext}\shell\{VERB}"));
        }
        Ok(())
    }
}

#[cfg(windows)]
pub fn install(registry: &Registry, lang: &str) -> Result<(), String> {
    imp::install_at(ROOT, registry, lang)
}

#[cfg(windows)]
pub fn uninstall() -> Result<(), String> {
    imp::uninstall_at(ROOT)
}

#[cfg(not(windows))]
pub fn install(_: &Registry, _: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn uninstall() -> Result<(), String> {
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    /// Writes under a throwaway test key (never the real Explorer one), reads back, cleans up.
    #[test]
    fn writes_cascading_menu() {
        let root = format!(r"Software\ConvertIO-test-{}", std::process::id());
        let mut registry = Registry::new();
        registry.register(convertio_image::ImageConverter);

        imp::install_at(&root, &registry, "fr").unwrap();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let base = hkcu.open_subkey(format!(r"{root}\.webp\shell\ConvertIO")).unwrap();
        let first: String = base.open_subkey(r"shell\00").unwrap().get_value("MUIVerb").unwrap();
        assert_eq!(first, "Convertir en PNG");
        let cmd: String = base.open_subkey(r"shell\00\command").unwrap().get_value("").unwrap();
        assert!(cmd.contains("--op image.convert.png \"%1\""), "{cmd}");

        imp::uninstall_at(&root).unwrap();
        assert!(hkcu.open_subkey(format!(r"{root}\.webp\shell\ConvertIO")).is_err());
        let _ = hkcu.delete_subkey_all(&root);
    }
}
