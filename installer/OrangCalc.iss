; Inno Setup script for OrangCalc
#define MyAppName "OrangCalc"
#define MyAppVersion "4.0.0"
#define MyAppPublisher "Orang Studio"
#define MyAppURL "https://github.com/Orang-Studio/OrangCalc"
#define MyAppExeName "OrangCalc.exe"
#define MyOutputDir "..\OrangCalc\bin\Release\net48"

[Setup]
AppId={{ACF9A789-137B-491B-9E0B-6F18D2F17E88}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
AppMutex=OrangCalc.SingleInstance
DefaultDirName={autopf}\{#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=Output
OutputBaseFilename=OrangCalc-{#MyAppVersion}-setup
SetupIconFile=..\orange.ico
WizardImageFile=assets\WizardLarge.png
WizardSmallImageFile=assets\WizardSmall.png
WizardImageBackColor=#e8f3f9
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#MyOutputDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#MyOutputDir}\OrangCalc.exe.config"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
// Requires .NET Framework 4.8 runtime (Release >= 528040)
function IsDotNet48Installed: Boolean;
var
  Value: Cardinal;
begin
  Result := False;
  if RegQueryDWordValue(HKLM, 'SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full', 'Release', Value) then
    Result := (Value >= 528040);
end;

function InitializeSetup(): Boolean;
var
  ErrorCode: Integer;
begin
  Result := True;
  if not IsDotNet48Installed then
  begin
    MsgBox('OrangCalc requires the .NET Framework 4.8 runtime, which was not detected on this system.' + #13#10#13#10 + 'Setup will now open the official download page. Install .NET Framework 4.8 and then run this installer again.', mbCriticalError, MB_OK);
    ShellExec('open', 'https://dotnet.microsoft.com/en-us/download/dotnet-framework/net48', '', '', SW_SHOWNORMAL, ewNoWait, ErrorCode);
    Result := False;
  end;
end;
