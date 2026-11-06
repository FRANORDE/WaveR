#define AppName "WaveR"
; I need to remember to change this version before every release :[
#define AppVersion "1.0.0"

[Setup]
; Fixed GUID that identifies the app: never change it between versions.
; You can keep this one or generate your own (Inno IDE: Tools > Generate GUID).
AppId={{A3F1C2D4-5B6E-4F70-9A81-2C3D4E5F6A7B}
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
WizardStyle=modern
UninstallDisplayIcon={app}\WaveR.exe
; Close a running WaveR during updates, and notify Windows about PATH changes
CloseApplications=yes
ChangesEnvironment=yes
; Uninstaller name
UninstallDisplayName={#AppName}
; Installer icon
SetupIconFile=assets\WaveR-install.ico
; Makes Setup notify the shell so Windows refreshes cached icons
ChangesAssociations=yes
; Show the licence
InfoBeforeFile=LICENSE
