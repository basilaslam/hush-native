Unicode True
!include "MUI2.nsh"
Name "Hush"
OutFile "Hush-0.5.0-Windows-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Hush"
RequestExecutionLevel user
Icon "..\..\native\assets\icon.ico"
UninstallIcon "..\..\native\assets\icon.ico"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Section "Hush native desktop client"
  SetOutPath "$INSTDIR"
  File "..\..\target\release\hush-native.exe"
  File "..\..\native\assets\OFL-Manrope.txt"
  ; Remove only the app-owned payload after writing the new client.
  ; Preferences and Cloudflare credentials are outside the install directory.
  RMDir /r "$INSTDIR\deployment"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Hush"
  CreateShortcut "$SMPROGRAMS\Hush\Hush.lnk" "$INSTDIR\hush-native.exe"
  CreateShortcut "$SMPROGRAMS\Hush\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Hush" "DisplayName" "Hush native voice"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Hush" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Hush" "DisplayVersion" "0.5.0"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Hush" "DisplayIcon" "$INSTDIR\hush-native.exe"
SectionEnd
Section "Uninstall"
  Delete "$INSTDIR\hush-native.exe"
  Delete "$INSTDIR\OFL-Manrope.txt"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir /r "$INSTDIR\deployment"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Hush\Hush.lnk"
  Delete "$SMPROGRAMS\Hush\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Hush"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Hush"
SectionEnd
