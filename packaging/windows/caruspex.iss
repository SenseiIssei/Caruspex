; Windows installer, built by tools/package.sh with Inno Setup 6:
;   iscc /DAppVersion=0.4.0 /DSourceDir=..\..\package /DOutputDir=..\..\dist caruspex.iss
; Installs for the current user without admin rights by default; the first
; page offers an installation for all users instead.

#ifndef AppVersion
  #error AppVersion is missing, pass /DAppVersion=x.y.z
#endif
#ifndef SourceDir
  #define SourceDir "..\..\package"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif

[Setup]
AppId={{6B0C2E52-8D2A-4E7B-9F51-3C1A0E4D7B21}
AppName=Caruspex
AppVersion={#AppVersion}
AppVerName=Caruspex {#AppVersion}
AppPublisher=SenseiIssei
AppPublisherURL=https://github.com/SenseiIssei/Caruspex
AppSupportURL=https://github.com/SenseiIssei/Caruspex/issues
AppUpdatesURL=https://github.com/SenseiIssei/Caruspex/releases
DefaultDirName={autopf}\Caruspex
DefaultGroupName=Caruspex
DisableProgramGroupPage=yes
LicenseFile={#SourceDir}\LICENSE
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=Caruspex-{#AppVersion}-windows-x64-setup
SetupIconFile=..\..\resources\icons\caruspex.ico
UninstallDisplayIcon={app}\caruspex.exe
UninstallDisplayName=Caruspex
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
VersionInfoVersion={#AppVersion}
VersionInfoProductName=Caruspex
VersionInfoDescription=Caruspex installer

[Languages]
Name: "de"; MessagesFile: "compiler:Languages\German.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
de.DesktopIcon=Verknüpfung auf dem Desktop
en.DesktopIcon=Shortcut on the desktop
de.CliPath=Kommandozeile caruspex_cli zum Suchpfad hinzufügen
en.CliPath=Add the command line tool caruspex_cli to the search path
de.Launch=Caruspex starten
en.Launch=Start Caruspex

[Tasks]
Name: "desktopicon"; Description: "{cm:DesktopIcon}"; Flags: unchecked
Name: "clipath"; Description: "{cm:CliPath}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Caruspex"; Filename: "{app}\caruspex.exe"
Name: "{autodesktop}\Caruspex"; Filename: "{app}\caruspex.exe"; Tasks: desktopicon

[Registry]
; The search path entry for caruspex_cli, per user or for all users.
Root: HKA; Subkey: "{code:EnvKey}"; ValueType: expandsz; ValueName: "Path"; \
    ValueData: "{olddata};{app}"; Tasks: clipath; Check: NeedsPathEntry(ExpandConstant('{app}'))

[Run]
Filename: "{app}\caruspex.exe"; Description: "{cm:Launch}"; Flags: nowait postinstall skipifsilent

[Code]
function EnvKey(Param: String): String;
begin
  if IsAdminInstallMode then
    Result := 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
  else
    Result := 'Environment';
end;

function NeedsPathEntry(Dir: String): Boolean;
var
  Current: String;
begin
  if not RegQueryStringValue(HKA, EnvKey(''), 'Path', Current) then
    Result := True
  else
    Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Current) + ';') = 0;
end;

procedure RemovePathEntry(Dir: String);
var
  Current, Needle: String;
  P: Integer;
begin
  if not RegQueryStringValue(HKA, EnvKey(''), 'Path', Current) then Exit;
  Needle := ';' + Uppercase(Dir) + ';';
  P := Pos(Needle, ';' + Uppercase(Current) + ';');
  if P = 0 then Exit;
  Current := ';' + Current + ';';
  Delete(Current, P, Length(Dir) + 1);
  Current := Copy(Current, 2, Length(Current) - 2);
  RegWriteExpandStringValue(HKA, EnvKey(''), 'Path', Current);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemovePathEntry(ExpandConstant('{app}'));
end;
