; Windows installer, built by tools/package.sh with Inno Setup 6:
;   iscc /DAppVersion=0.9.0 /DSourceDir=..\..\package /DOutputDir=..\..\dist caruspex.iss
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
de.WebView2=Microsoft WebView2 wird eingerichtet ...
en.WebView2=Setting up Microsoft WebView2 ...

[Tasks]
Name: "desktopicon"; Description: "{cm:DesktopIcon}"; Flags: unchecked
Name: "clipath"; Description: "{cm:CliPath}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Excludes: "MicrosoftEdgeWebview2Setup.exe"; Flags: ignoreversion recursesubdirs createallsubdirs
; Microsoft's WebView2 bootstrapper, only unpacked and run when the runtime is missing.
Source: "{#SourceDir}\MicrosoftEdgeWebview2Setup.exe"; DestDir: "{tmp}"; Flags: deleteafterinstall; Check: NeedsWebView2

[InstallDelete]
; Up to 0.8 Caruspex was a Qt program. An update over such an installation
; removes the Qt libraries and plug-ins it no longer needs.
Type: files; Name: "{app}\Qt6*.dll"
Type: files; Name: "{app}\msvcp140*.dll"
Type: files; Name: "{app}\vcruntime140*.dll"
Type: files; Name: "{app}\concrt140.dll"
Type: files; Name: "{app}\vccorlib140.dll"
Type: files; Name: "{app}\D3Dcompiler_47.dll"
Type: files; Name: "{app}\opengl32sw.dll"
Type: files; Name: "{app}\*.qm"
Type: filesandordirs; Name: "{app}\platforms"
Type: filesandordirs; Name: "{app}\styles"
Type: filesandordirs; Name: "{app}\imageformats"
Type: filesandordirs; Name: "{app}\iconengines"
Type: filesandordirs; Name: "{app}\tls"
Type: filesandordirs; Name: "{app}\generic"
Type: filesandordirs; Name: "{app}\networkinformation"
Type: filesandordirs; Name: "{app}\printsupport"
Type: filesandordirs; Name: "{app}\translations"

[Icons]
Name: "{autoprograms}\Caruspex"; Filename: "{app}\caruspex.exe"
Name: "{autodesktop}\Caruspex"; Filename: "{app}\caruspex.exe"; Tasks: desktopicon

[Registry]
; The search path entry for caruspex_cli, per user or for all users.
Root: HKA; Subkey: "{code:EnvKey}"; ValueType: expandsz; ValueName: "Path"; \
    ValueData: "{olddata};{app}"; Tasks: clipath; Check: NeedsPathEntry(ExpandConstant('{app}'))

[Run]
Filename: "{tmp}\MicrosoftEdgeWebview2Setup.exe"; Parameters: "/silent /install"; StatusMsg: "{cm:WebView2}"; Check: NeedsWebView2; Flags: waituntilterminated
Filename: "{app}\caruspex.exe"; Description: "{cm:Launch}"; Flags: nowait postinstall skipifsilent
; The update of the app itself (update.rs) runs this installer with
; /VERYSILENT /CARUSPEXRESTART=1 after Caruspex has quit, so the entry above
; is skipped and this one starts the new version once, as the same user.
Filename: "{app}\caruspex.exe"; Flags: nowait runasoriginaluser; Check: RestartAfterUpdate

[Code]
// Only in the silent run of the app's own update. /RESTARTAPPLICATIONS
// alone does not help there: it restarts programs that registered with the
// Restart Manager, and Caruspex has already ended before the files change.
function RestartAfterUpdate: Boolean;
begin
  Result := WizardSilent and (ExpandConstant('{param:CARUSPEXRESTART|0}') = '1');
end;

// The interface runs in Microsoft's WebView2, part of Windows 10 and 11 in
// almost every case. Installed per machine or per user, its version sits here.
function NeedsWebView2: Boolean;
var
  Version: String;
  Key: String;
begin
  Key := 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  Result := True;
  if RegQueryStringValue(HKLM, 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0') then
    Result := False
  else if RegQueryStringValue(HKLM, Key, 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0') then
    Result := False
  else if RegQueryStringValue(HKCU, Key, 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0') then
    Result := False;
end;

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
