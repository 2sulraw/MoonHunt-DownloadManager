; MoonHunt 0.0.7 Windows installer (NSIS, perMachine)
; Mirrors the old Tauri 0.0.6 NSIS install: same product dir, same id,
; and it runs the previous uninstaller before laying down the new files.

Unicode true
!include "MUI2.nsh"

!define APP_NAME "MoonHunt"
!define APP_VERSION "0.0.7"
!define APP_EXE "moonhunt.exe"
!define SIDECAR "aria2c-x86_64-pc-windows-msvc.exe"
!define ROOT "D:\MoonHunt-Rust"
!define SIDECAR_SRC "D:\MoonHunt\src-tauri\binaries\aria2c-x86_64-pc-windows-msvc.exe"

Name "${APP_NAME} ${APP_VERSION}"
OutFile "${ROOT}\dist\moonhunt_${APP_VERSION}_x64-setup.exe"
InstallDir "$PROGRAMFILES64\moonhunt"
InstallDirRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\MoonHunt" "InstallLocation"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

VIProductVersion "0.0.7.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "CompanyName" "werlos"
VIAddVersionKey "FileDescription" "${APP_NAME} installer"

Icon "${ROOT}\app\assets\moonhunt.ico"
UninstallIcon "${ROOT}\app\assets\moonhunt.ico"

!define MUI_ICON "${ROOT}\app\assets\moonhunt.ico"
!define MUI_UNICON "${ROOT}\app\assets\moonhunt.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetRegView 64
  ; Close a running copy so the files can be replaced.
  nsExec::ExecToLog 'taskkill /F /IM ${APP_EXE}'
  Pop $0

  ; Upgrade path: run the old (Tauri) uninstaller silently first.
  IfFileExists "$INSTDIR\uninstall.exe" 0 +2
    ExecWait '"$INSTDIR\uninstall.exe" /S _?=$INSTDIR'

  SetOutPath "$INSTDIR"
  File "${ROOT}\target\release\${APP_EXE}"
  File "/oname=${SIDECAR}" "${SIDECAR_SRC}"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"

  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\MoonHunt" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\MoonHunt" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\MoonHunt" "Publisher" "werlos"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\MoonHunt" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\MoonHunt" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\MoonHunt" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\MoonHunt" "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\MoonHunt" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetRegView 64
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\${SIDECAR}"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\MoonHunt"
SectionEnd
