; NSIS installer for Sxarty.
;
;   makensis -DVERSION=0.2.0 -DSOURCE=..\..\target\release packaging\windows\sxarty.nsi
;
; Registers Sxarty as an *option* for the document types it reads rather than
; taking them over. Nobody installs a speed reader expecting their PDFs to stop
; opening in their PDF viewer, so the associations go in OpenWithProgids and the
; user's existing defaults are left alone.

Unicode true
ManifestDPIAware true

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef SOURCE
  !define SOURCE "..\..\target\release"
!endif

!define APPNAME  "Sxarty"
!define PROGID   "Sxarty.Document"
!define PUBLISHER "Beqa Abuladze"
!define HOMEPAGE "https://github.com/beqaabu/Sxarty"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

Name "${APPNAME} ${VERSION}"
OutFile "..\..\target\dist\Sxarty-${VERSION}-setup.exe"
InstallDir "$PROGRAMFILES64\${APPNAME}"
InstallDirRegKey HKLM "Software\${APPNAME}" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName"     "${APPNAME}"
VIAddVersionKey "FileDescription" "${APPNAME} installer"
VIAddVersionKey "FileVersion"     "${VERSION}"
VIAddVersionKey "ProductVersion"  "${VERSION}"
VIAddVersionKey "CompanyName"     "${PUBLISHER}"
VIAddVersionKey "LegalCopyright"  "MIT licensed"

!define MUI_ICON   "..\..\assets\icon\sxarty.ico"
!define MUI_UNICON "..\..\assets\icon\sxarty.ico"
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_LICENSE "..\..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\sxarty.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Open ${APPNAME}"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "${APPNAME} needs a 64-bit version of Windows."
    Abort
  ${EndIf}
FunctionEnd

Section "Install"
  SetOutPath "$INSTDIR"
  File "${SOURCE}\sxarty.exe"
  File "..\..\LICENSE"
  File "..\..\README.md"

  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortCut "$SMPROGRAMS\${APPNAME}.lnk" "$INSTDIR\sxarty.exe" "" "$INSTDIR\sxarty.exe" 0

  WriteRegStr HKLM "Software\${APPNAME}" "InstallDir" "$INSTDIR"

  ; Add/Remove Programs
  WriteRegStr   HKLM "${UNINSTKEY}" "DisplayName"     "${APPNAME}"
  WriteRegStr   HKLM "${UNINSTKEY}" "DisplayVersion"  "${VERSION}"
  WriteRegStr   HKLM "${UNINSTKEY}" "DisplayIcon"     "$INSTDIR\sxarty.exe"
  WriteRegStr   HKLM "${UNINSTKEY}" "Publisher"       "${PUBLISHER}"
  WriteRegStr   HKLM "${UNINSTKEY}" "URLInfoAbout"    "${HOMEPAGE}"
  WriteRegStr   HKLM "${UNINSTKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr   HKLM "${UNINSTKEY}" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKLM "${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTKEY}" "NoRepair" 1

  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKLM "${UNINSTKEY}" "EstimatedSize" "$0"

  ; The document type Sxarty registers, used only through "Open with".
  WriteRegStr HKLM "Software\Classes\${PROGID}" "" "Document"
  WriteRegStr HKLM "Software\Classes\${PROGID}\DefaultIcon" "" "$INSTDIR\sxarty.exe,0"
  WriteRegStr HKLM "Software\Classes\${PROGID}\shell\open\command" "" '"$INSTDIR\sxarty.exe" "%1"'

  ; Offered for these extensions without displacing whatever already owns them.
  WriteRegStr HKLM "Software\Classes\.txt\OpenWithProgids"  "${PROGID}" ""
  WriteRegStr HKLM "Software\Classes\.md\OpenWithProgids"   "${PROGID}" ""
  WriteRegStr HKLM "Software\Classes\.pdf\OpenWithProgids"  "${PROGID}" ""
  WriteRegStr HKLM "Software\Classes\.epub\OpenWithProgids" "${PROGID}" ""
  WriteRegStr HKLM "Software\Classes\.docx\OpenWithProgids" "${PROGID}" ""

  ; Lets the user pick Sxarty as a default from Windows Settings if they want to.
  WriteRegStr HKLM "Software\${APPNAME}\Capabilities" "ApplicationName"        "${APPNAME}"
  WriteRegStr HKLM "Software\${APPNAME}\Capabilities" "ApplicationDescription" "Reads a document one word at a time"
  WriteRegStr HKLM "Software\${APPNAME}\Capabilities\FileAssociations" ".txt"  "${PROGID}"
  WriteRegStr HKLM "Software\${APPNAME}\Capabilities\FileAssociations" ".md"   "${PROGID}"
  WriteRegStr HKLM "Software\${APPNAME}\Capabilities\FileAssociations" ".pdf"  "${PROGID}"
  WriteRegStr HKLM "Software\${APPNAME}\Capabilities\FileAssociations" ".epub" "${PROGID}"
  WriteRegStr HKLM "Software\${APPNAME}\Capabilities\FileAssociations" ".docx" "${PROGID}"
  WriteRegStr HKLM "Software\RegisteredApplications" "${APPNAME}" "Software\${APPNAME}\Capabilities"

  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\sxarty.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\uninstall.exe"
  RMDir  "$INSTDIR"

  Delete "$SMPROGRAMS\${APPNAME}.lnk"

  DeleteRegKey HKLM "${UNINSTKEY}"
  DeleteRegKey HKLM "Software\Classes\${PROGID}"
  DeleteRegValue HKLM "Software\Classes\.txt\OpenWithProgids"  "${PROGID}"
  DeleteRegValue HKLM "Software\Classes\.md\OpenWithProgids"   "${PROGID}"
  DeleteRegValue HKLM "Software\Classes\.pdf\OpenWithProgids"  "${PROGID}"
  DeleteRegValue HKLM "Software\Classes\.epub\OpenWithProgids" "${PROGID}"
  DeleteRegValue HKLM "Software\Classes\.docx\OpenWithProgids" "${PROGID}"
  DeleteRegValue HKLM "Software\RegisteredApplications" "${APPNAME}"
  DeleteRegKey HKLM "Software\${APPNAME}"

  ; The user's library and settings are deliberately left in place.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'
SectionEnd
