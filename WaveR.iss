#define AppName "WaveR"
; I need to remember to change this version before every release :[
#define AppVersion "1.2.1"

[Setup]
; forces the dark appearance, whatever the Windows theme is
WizardStyle=modern dark
; Fixed GUID that identifies the app: never change it between versions.
; You can keep this one or generate your own (Inno IDE: Tools > Generate GUID).
AppId={{B4388D81-024E-48AA-BF23-7B7C86037854}}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=FRANORDE
DefaultDirName={%USERPROFILE}\.waver
; No admin rights needed, and settings.jsonc can be written next to the exe
PrivilegesRequired=lowest
OutputDir=dist
OutputBaseFilename=WaveR-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
UninstallDisplayIcon={app}\WaveR.exe
; Close a running WaveR during updates, and notify Windows about PATH changes
CloseApplications=yes
; Uninstaller name
UninstallDisplayName={#AppName}
; Installer icon
SetupIconFile=assets\WaveR-install.ico
; Makes Setup notify the shell so Windows refreshes cached icons
ChangesAssociations=yes
; Show the licence
InfoBeforeFile=LICENSE

[Files]
Source: "target\release\WaveR.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\WaveR-uninstall.ico"; DestDir: "{app}\icons"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}\licenses"; DestName: "WaveR-LICENSE.txt"; Flags: ignoreversion
Source: "licenses\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}\licenses"; Flags: ignoreversion

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\WaveR.exe"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\WaveR.exe"; Tasks: desktopicon
; Adds an "Uninstall WaveR" entry next to the app in the Start Menu
Name: "{autoprograms}\Uninstall {#AppName}"; Filename: "{uninstallexe}"; IconFilename: "{app}\icons\WaveR-uninstall.ico"

[UninstallDelete]
; settings.jsonc is created at runtime, so the uninstaller doesn't know about it
Type: files; Name: "{app}\settings.jsonc"

[Run]
; Makes the "Launch app" checkbox
Filename: "{app}\WaveR.exe"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
