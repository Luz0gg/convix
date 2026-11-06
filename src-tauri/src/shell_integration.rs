//! Windows Explorer context menu ("clic droit › ConvertIO").
//!
//! Plan (Phase 1), see docs/ARCHITECTURE.md § "Clic droit":
//! - Classic cascading menu written under `HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell\ConvertIO`
//!   for each supported extension, with sub-commands built from the registry
//!   (the 4-6 most useful operations per format + "Ouvrir dans ConvertIO").
//! - HKCU only: no admin rights, removed by the uninstaller and by the settings toggle.
//! - On Windows 11 it appears under "Afficher plus d'options" (Shift+F10). A top-level
//!   Windows 11 entry needs an `IExplorerCommand` COM handler + sparse package: later phase.

pub fn install() -> Result<(), String> {
    Err("Explorer context menu: not implemented yet (Phase 1)".into())
}

pub fn uninstall() -> Result<(), String> {
    Err("Explorer context menu: not implemented yet (Phase 1)".into())
}
