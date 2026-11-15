; Agate was called Blocco until 0.2.0. When Blocco updates itself to Agate (or Agate is
; installed over it), Agate goes next to the old folder, gets its shortcuts and the old app
; is removed. The data folder (%APPDATA%\Blocco -> %APPDATA%\Agate) is moved by the app itself
; on its first start, so nothing here touches profiles, worlds or accounts.

!define AGATE_OLD_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Blocco"
Var AgateOldDir

; "C:\path" -> C:\path
!macro AgateReadOldDir
  ReadRegStr $AgateOldDir HKCU "${AGATE_OLD_KEY}" "InstallLocation"
  StrCpy $0 $AgateOldDir 1
  ${If} $0 == '"'
    StrCpy $AgateOldDir $AgateOldDir "" 1
    StrCpy $AgateOldDir $AgateOldDir -1
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  ; The in-app updater runs this installer with /UPDATE. Agate has no install location of its
  ; own yet, so INSTDIR is the default: keep the drive and folder the user chose for Blocco.
  ${If} $UpdateMode = 1
    ReadRegStr $0 SHCTX "${MANUPRODUCTKEY}" ""
    ${If} $0 == ""
      !insertmacro AgateReadOldDir
      ${If} $AgateOldDir != ""
        ${GetParent} $AgateOldDir $1
        StrCpy $INSTDIR "$1\Agate"
        SetOutPath $INSTDIR
      ${EndIf}
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro AgateReadOldDir
  ${If} $AgateOldDir != ""
  ${AndIf} ${FileExists} "$AgateOldDir\uninstall.exe"
    ; Update mode creates no shortcuts: give Agate the ones Blocco had.
    ${IfNot} ${FileExists} "$SMPROGRAMS\${PRODUCTNAME}.lnk"
      CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
      !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    ${EndIf}
    ${If} ${FileExists} "$DESKTOP\Blocco.lnk"
    ${AndIfNot} ${FileExists} "$DESKTOP\${PRODUCTNAME}.lnk"
      CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
      !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
    ${EndIf}
    ; Remove Blocco: its own uninstaller, silent, waited for (_?= keeps it in place), which also
    ; removes its shortcuts and registry entries; then the uninstaller itself and the empty folder.
    ExecWait '"$AgateOldDir\uninstall.exe" /S _?=$AgateOldDir'
    Delete "$AgateOldDir\uninstall.exe"
    RMDir "$AgateOldDir"
    ; Blocco's WebView cache (Agate has its own).
    RMDir /r "$LOCALAPPDATA\app.blocco.launcher"
  ${EndIf}
!macroend
