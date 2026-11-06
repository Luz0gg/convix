; NSIS installer hooks (see tauri.conf.json > bundle.windows.nsis.installerHooks).
; Keep the extension list in sync with EXTENSIONS in src-tauri/src/shell_integration.rs.

!macro NSIS_HOOK_POSTUNINSTALL
  ; Remove the Explorer context menu written by the app (HKCU, per user).
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.png\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.jpg\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.jpeg\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.jfif\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.webp\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.gif\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.bmp\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tif\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tiff\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.ico\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.tga\shell\ConvertIO"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.svg\shell\ConvertIO"
!macroend
