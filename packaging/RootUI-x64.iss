; RootUI installer for Windows 64-bit (Inno Setup 6).
;
;     ISCC.exe packaging\RootUI-x64.iss
;
; Needs dist\windows-x64\RootUI.exe and dist\windows-x64\ui, which BuildFORwindows.bat creates.
; Result: dist\installers\RootUI-1.0V-Setup-x64.exe

#define Arch "x64"

#define MyAppName      "RootUI"
#define MyAppVersion   "1.0V"
#define MyAppPublisher "gitsquare"
#define MyAppHelpURL    "https://github.com/HafizIbrahimSuriya/RootUI"
#define MyAppSupportURL "https://github.com/HafizIbrahimSuriya"
#define MyAppExe       "RootUI.exe"
#define Src            "..\dist\windows-" + Arch

[Setup]
; The same AppId in every installer, so a newer one upgrades an older one.
AppId={{8E5B2C1A-3D47-4F6B-9A10-52C0D1A7B9E4}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
; Control Panel: "Support link" = AppPublisherURL, "Help link" = AppSupportURL
AppPublisherURL={#MyAppSupportURL}
AppSupportURL={#MyAppHelpURL}
VersionInfoVersion=1.0.0.0
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\dist\installers
OutputBaseFilename=RootUI-{#MyAppVersion}-Setup-{#Arch}
SetupIconFile=..\rootui.ico
UninstallDisplayIcon={app}\{#MyAppExe}
UninstallDisplayName={#MyAppName}
WizardImageFile=assets\wizard_large.bmp
WizardSmallImageFile=assets\wizard_small.bmp
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
MinVersion=10.0
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
CloseApplications=yes
; The old "x64" name was replaced by "x64compatible" in Inno Setup 6.3, so pick by version.
#if Ver >= 0x06030000
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#else
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#Src}\{#MyAppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Src}\ui\*"; DestDir: "{app}\ui"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\rootui.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start menu ("program list"), optional desktop icon, and an uninstall entry
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExe}"; IconFilename: "{app}\rootui.ico"; WorkingDir: "{app}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExe}"; IconFilename: "{app}\rootui.ico"; WorkingDir: "{app}"; Tasks: desktopicon
Name: "{autoprograms}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\{#MyAppExe}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent

[Code]
// RootUI draws its window with Microsoft WebView2 (part of Windows 11 and every up-to-date Windows 10).
function WebView2Installed(): Boolean;
var
  v: String;
begin
  Result := RegQueryStringValue(HKLM, 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', v)
         or RegQueryStringValue(HKLM, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', v)
         or RegQueryStringValue(HKCU, 'Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', v);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and (not WebView2Installed()) then
    MsgBox('Microsoft WebView2 Runtime was not detected. If RootUI does not open, install it from:' + #13#10 +
           'https://developer.microsoft.com/microsoft-edge/webview2/', mbInformation, MB_OK);
end;
