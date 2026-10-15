; NSIS hooks for the Disk Cleaner installer (bundle.windows.nsis.installerHooks).

; "Settings > Uninstall Disk Cleaner" runs `uninstall.exe /P /DELETEAPPDATA` after the user typed
; the confirmation phrase in the app. /P skips the uninstaller's own pages, so this ticks its
; "Delete the application data" option on the user's behalf: the data folders are locked while
; the app runs, so only the uninstaller can remove them.
!macro NSIS_HOOK_PREUNINSTALL
  ClearErrors
  ${GetOptions} $CMDLINE "/DELETEAPPDATA" $R0
  ${IfNot} ${Errors}
    StrCpy $DeleteAppDataCheckboxState 1
  ${EndIf}
!macroend
