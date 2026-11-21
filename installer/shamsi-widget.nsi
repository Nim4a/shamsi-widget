; Shamsi Widget installer (NSIS 3, Unicode, per-user, no admin required)
; Build:  makensis /DSRCDIR=<dir with shamsi_widget.exe + widget.ico> shamsi-widget.nsi
Unicode true

!define APPNAME   "Shamsi Widget"
!define APPNAME_FA "ویجت تقویم شمسی"
!define VERSION   "1.0.0"
!define PUBLISHER "Nim4a"
!define URL       "https://github.com/Nim4a/shamsi-widget"
!define EXE       "shamsi_widget.exe"
!define ICONF     "widget.ico"
!define REGKEY    "Software\Microsoft\Windows\CurrentVersion\Uninstall\ShamsiWidget"

!ifndef SRCDIR
  !define SRCDIR "${__FILEDIR__}\.."
!endif

Name "${APPNAME} ${VERSION}"
BrandingText "${APPNAME} ${VERSION} — ${PUBLISHER}"
OutFile "${__FILEDIR__}\ShamsiWidget-Setup-${VERSION}.exe"
InstallDir "$LOCALAPPDATA\Programs\${APPNAME}"
InstallDirRegKey HKCU "Software\ShamsiWidget" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 32

VIProductVersion "1.0.0.0"
VIAddVersionKey "ProductName"     "${APPNAME}"
VIAddVersionKey "ProductVersion"  "${VERSION}"
VIAddVersionKey "FileDescription" "${APPNAME} Setup"
VIAddVersionKey "FileVersion"     "${VERSION}.0"
VIAddVersionKey "CompanyName"     "${PUBLISHER}"
VIAddVersionKey "LegalCopyright"  "MIT — ${PUBLISHER}"

!include "MUI2.nsh"
!include "FileFunc.nsh"

!define MUI_ICON   "${SRCDIR}\${ICONF}"
!define MUI_UNICON "${SRCDIR}\${ICONF}"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "اجرای ویجت شمسی / Run Shamsi Widget"
!define MUI_FINISHPAGE_NOREBOOTSUPPORT

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "Farsi"
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_RESERVEFILE_LANGDLL

; ---------- sections ----------
LangString DESC_APP    ${LANG_FARSI}   "فایل‌های اصلی ویجت، میان‌بر منوی استارت و برنامهٔ حذف نصب."
LangString DESC_APP    ${LANG_ENGLISH} "Core widget files, Start Menu shortcut and the uninstaller."
LangString DESC_DESK   ${LANG_FARSI}   "ساخت میان‌بر روی دسکتاپ."
LangString DESC_DESK   ${LANG_ENGLISH} "Create a desktop shortcut."
LangString DESC_START  ${LANG_FARSI}   "ویجت هر بار با روشن شدن ویندوز اجرا شود."
LangString DESC_START  ${LANG_ENGLISH} "Start the widget automatically when Windows starts."

Section "Shamsi Widget  (فایل‌های برنامه)" SEC_APP
  SectionIn RO
  ; stop a running copy so the files are not locked
  nsExec::ExecToLog 'taskkill /IM ${EXE} /F'
  Pop $0

  SetOutPath "$INSTDIR"
  SetOverwrite try
  File "${SRCDIR}\${EXE}"
  File "${SRCDIR}\${ICONF}"
  File "${SRCDIR}\README.md"
  SetOverwrite on
  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKCU "Software\ShamsiWidget" "InstallDir" "$INSTDIR"

  CreateDirectory "$SMPROGRAMS\${APPNAME}"
  CreateShortCut "$SMPROGRAMS\${APPNAME}\Shamsi Widget.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${ICONF}" 0
  CreateShortCut "$SMPROGRAMS\${APPNAME}\Uninstall Shamsi Widget.lnk" "$INSTDIR\uninstall.exe"

  WriteRegStr   HKCU "${REGKEY}" "DisplayName"     "${APPNAME} — ${APPNAME_FA}"
  WriteRegStr   HKCU "${REGKEY}" "DisplayVersion"  "${VERSION}"
  WriteRegStr   HKCU "${REGKEY}" "DisplayIcon"     "$INSTDIR\${ICONF}"
  WriteRegStr   HKCU "${REGKEY}" "Publisher"       "${PUBLISHER}"
  WriteRegStr   HKCU "${REGKEY}" "URLInfoAbout"    "${URL}"
  WriteRegStr   HKCU "${REGKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr   HKCU "${REGKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr   HKCU "${REGKEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${REGKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${REGKEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${REGKEY}" "EstimatedSize" "$0"
SectionEnd

Section /o "Shamsi Widget  (دسکتاپ)" SEC_DESK
  CreateShortCut "$DESKTOP\Shamsi Widget.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${ICONF}" 0
SectionEnd

Section /o "Shamsi Widget  (اجرا با ویندوز)" SEC_START
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ShamsiWidget" '"$INSTDIR\${EXE}"'
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_APP}   $(DESC_APP)
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_DESK}  $(DESC_DESK)
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_START} $(DESC_START)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; ---------- uninstall ----------
Section "Uninstall"
  nsExec::ExecToLog 'taskkill /IM ${EXE} /F'
  Pop $0
  Sleep 600

  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\${ICONF}"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${APPNAME}\Shamsi Widget.lnk"
  Delete "$SMPROGRAMS\${APPNAME}\Uninstall Shamsi Widget.lnk"
  RMDir  "$SMPROGRAMS\${APPNAME}"
  Delete "$DESKTOP\Shamsi Widget.lnk"

  ; only remove the autostart entry if it points at this installation
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ShamsiWidget"
  StrCmp $0 '"$INSTDIR\${EXE}"' 0 check_plain
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ShamsiWidget"
    Goto autostart_done
  check_plain:
  StrCmp $0 "$INSTDIR\${EXE}" 0 autostart_done
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ShamsiWidget"
  autostart_done:

  DeleteRegKey HKCU "${REGKEY}"
  DeleteRegKey HKCU "Software\ShamsiWidget"   ; kept only while installed
  ; NOTE: "%APPDATA%\ShamsiWidget" (the user's settings) is deliberately left in place,
  ; so that a reinstall or a parallel build keeps its position/colours/opacity.
SectionEnd
