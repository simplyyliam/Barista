; Extra steps for the NSIS installer (tauri.conf.json > bundle.windows.nsis.installerHooks).

; Before files are replaced (install or update): stop the running bar so its exe is not locked. A killed bar
; cannot give the Windows taskbar back, so the installed exe does it from the saved settings first; the new
; bar puts the auto-hide back when it starts (ADR 0031).
!macro NSIS_HOOK_PREINSTALL
  nsExec::Exec 'taskkill /F /IM barista.exe'
  Sleep 500
  IfFileExists "$INSTDIR\barista.exe" 0 +2
  nsExec::Exec '"$INSTDIR\barista.exe" --uninstall-cleanup'
!macroend

; Before files are removed: stop the bar, give the Windows taskbar back, forget "start with Windows".
!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /F /IM barista.exe'
  Sleep 500
  nsExec::Exec '"$INSTDIR\barista.exe" --uninstall-cleanup'
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Barista"
!macroend
