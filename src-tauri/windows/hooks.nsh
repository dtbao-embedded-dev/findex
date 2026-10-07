; Findex's installer hooks (bundle.windows.nsis.installerHooks in tauri.conf.json).
;
; Findex writes its "Tag with Findex" Explorer menu under HKCU at start
; (src/model/shell_menu.rs). Uninstalling removes it, so Explorer never offers a
; helper that is gone. An update that runs this is harmless: the next start of
; Findex writes the menu again.
!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Classes\Directory\shell\FindexTag"
  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\FindexTag"
!macroend
