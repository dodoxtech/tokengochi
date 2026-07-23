; NSIS installer hooks for Tokengochi (Windows).
;
; Wired into the bundle via `tauri.conf.json` -> bundle.windows.nsis.installerHooks.
; Goal: when the user *uninstalls* the app, also delete its persisted data so a
; later reinstall starts from a brand-new, empty database. Crucially this must
; NOT fire during an in-place update, or the user would lose their pet and token
; history every time they upgrade. Tauri's updater runs the uninstaller with the
; `/UPDATE` flag, so we skip deletion whenever that flag is present.
;
; The paths mirror `src-tauri/src/storage_paths.rs`: on Windows `dirs::data_dir()`
; resolves to %APPDATA% (Roaming), where the app writes:
;   - %APPDATA%\com.tokengochi.app   (SQLite game database)
;   - %APPDATA%\tokengochi           (watcher bookkeeping files)

!include FileFunc.nsh
!include LogicLib.nsh

!macro NSIS_HOOK_POSTUNINSTALL
  ${GetOptions} $CMDLINE "/UPDATE" $R0
  ${If} ${Errors}
    ; Not an update -> a real uninstall. Remove all persisted app data.
    RMDir /r "$APPDATA\com.tokengochi.app"
    RMDir /r "$APPDATA\tokengochi"
  ${EndIf}
!macroend
