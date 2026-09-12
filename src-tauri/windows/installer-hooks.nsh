; MoonHunt NSIS installer hooks (Tauri bundler: bundle.windows.nsis.installerHooks)
;
; The app spawns aria2c.exe as a detached child process. Builds up to 0.0.3
; never terminated it on exit, so a live aria2c kept $INSTDIR\aria2c.exe open:
; updates failed to replace files and uninstall left aria2c.exe behind.
; Both installers now close aria2c themselves before touching any files,
; using Tauri's bundled nsis_tauri_utils plugin (same primitive the template
; uses for the main executable).
;
; Return codes of nsis_tauri_utils::KillProcess: 0 = killed, 2 = not running.
; Both count as success; anything else is ignored here because the subsequent
; file operations will surface a proper error if something still locks files.

!macro KillAria2Quietly
  nsis_tauri_utils::KillProcess "aria2c.exe"
  Pop $R8
  Sleep 500
!macroend

!macro NSIS_HOOK_PREINSTALL
  ; Close any detached aria2c before the app-running check / file copy.
  !insertmacro KillAria2Quietly
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Close any detached aria2c so the uninstaller can delete it.
  !insertmacro KillAria2Quietly
!macroend
