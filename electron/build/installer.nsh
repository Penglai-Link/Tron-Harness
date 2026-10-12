!define TRON_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define TRON_STARTUP_KEY "Software\Tron\Desktop"
!define TRON_STARTUP_NAME "com.tron.desktop"

!macro customInstall
  # Initialize once, including when upgrading from a version without startup
  # support. Subsequent upgrades preserve a user's disabled startup setting.
  ReadRegDWORD $0 HKCU "${TRON_STARTUP_KEY}" "StartupInitialized"
  ${If} $0 != 1
    WriteRegStr HKCU "${TRON_RUN_KEY}" "${TRON_STARTUP_NAME}" '$\"$appExe$\" --tron-startup'
    WriteRegDWORD HKCU "${TRON_STARTUP_KEY}" "StartupInitialized" 1
  ${Else}
    # Refresh the executable path if the user changed the installation directory.
    ReadRegStr $0 HKCU "${TRON_RUN_KEY}" "${TRON_STARTUP_NAME}"
    ${If} $0 != ""
      WriteRegStr HKCU "${TRON_RUN_KEY}" "${TRON_STARTUP_NAME}" '$\"$appExe$\" --tron-startup'
    ${EndIf}
  ${EndIf}
!macroend

!macro customUnInstall
  # electron-builder invokes the old uninstaller with --updated during upgrades.
  ${IfNot} ${isUpdated}
    DeleteRegValue HKCU "${TRON_RUN_KEY}" "${TRON_STARTUP_NAME}"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "${TRON_STARTUP_NAME}"
    DeleteRegKey HKCU "${TRON_STARTUP_KEY}"
  ${EndIf}
!macroend
