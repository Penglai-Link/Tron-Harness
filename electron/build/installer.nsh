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

# Keep install-local user data during upgrades and by default on uninstall.
# The default electron-builder uninstaller removes the entire installation.
!macro customRemoveFiles
  SetOutPath "$TEMP"
  FindFirst $0 $1 "$INSTDIR\*"
  tron_remove_next:
    StrCmp $1 "" tron_remove_done
    StrCmp $1 "." tron_remove_continue
    StrCmp $1 ".." tron_remove_continue
    StrCmp $1 "data" tron_remove_continue
    IfFileExists "$INSTDIR\$1\*.*" 0 tron_remove_file
    RMDir /r "$INSTDIR\$1"
    Goto tron_remove_continue
  tron_remove_file:
    Delete "$INSTDIR\$1"
  tron_remove_continue:
    FindNext $0 $1
    Goto tron_remove_next
  tron_remove_done:
    FindClose $0
    RMDir "$INSTDIR"
!macroend

!macro customUnInstall
  # electron-builder invokes the old uninstaller with --updated during upgrades.
  ${IfNot} ${isUpdated}
    DeleteRegValue HKCU "${TRON_RUN_KEY}" "${TRON_STARTUP_NAME}"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "${TRON_STARTUP_NAME}"
    DeleteRegKey HKCU "${TRON_STARTUP_KEY}"
  ${EndIf}
!macroend

# electron-builder adds an optional checkbox to the assisted uninstaller and
# runs this section after customRemoveFiles has preserved the data directory.
!macro customUnInstallSection
  Section /o "un.Delete personal data (data) / 删除个人数据"
    # The installed uninstaller also runs silently during an upgrade.
    ${IfNot} ${isUpdated}
      RMDir /r "$INSTDIR\data"
      RMDir "$INSTDIR"
    ${EndIf}
  SectionEnd
!macroend
