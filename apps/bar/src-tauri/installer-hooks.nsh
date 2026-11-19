; Extra steps for the NSIS installer (tauri.conf.json > bundle.windows.nsis.installerHooks).

; Before files are replaced (install or update): stop the running bar so its exe is not locked.
!macro NSIS_HOOK_PREINSTALL
  nsExec::Exec 'taskkill /F /IM barista.exe'
!macroend

; Before files are removed: stop the bar, give the Windows taskbar back, forget "start with Windows".
!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /F /IM barista.exe'
  Sleep 500
  nsExec::Exec '"$INSTDIR\barista.exe" --uninstall-cleanup'
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Barista"
!macroend
