; The per-user Windows installer (ADR 0019): no administrator prompt, a Start menu entry, and an
; uninstaller that keeps the user's data and library in %LOCALAPPDATA%\SkillBinder. The same
; program installs app updates: the app starts it at quit with /VERYSILENT /SUPPRESSMSGBOXES
; /NORESTART /WAITPID=<pid>, and /RELAUNCH=1 after Restart (crates/engine/src/app_update/install.rs).
; `cargo run -p xtask -- package` compiles it, defining AppVersion, NumericVersion, and SourceDir.

#ifndef AppVersion
  #error Compile with `cargo run -p xtask -- package`, which defines AppVersion, NumericVersion, and SourceDir.
#endif

[Setup]
; Never change it: the uninstall entry, and every update, find the installation by it.
AppId={{E9E8B1D0-B6A5-49E7-9CF5-7B63E7480835}
AppName=SkillBinder
AppVersion={#AppVersion}
AppVerName=SkillBinder {#AppVersion}
AppPublisher=SkillBinder
AppPublisherURL=https://github.com/mastro993/SkillBinder
AppSupportURL=https://github.com/mastro993/SkillBinder/issues
AppUpdatesURL=https://github.com/mastro993/SkillBinder/releases
; Only numbers fit here, so a beta carries its prerelease in AppVersion alone.
VersionInfoVersion={#NumericVersion}
PrivilegesRequired=lowest
DefaultDirName={localappdata}\Programs\SkillBinder
DisableDirPage=yes
DisableProgramGroupPage=yes
UninstallDisplayName=SkillBinder
SetupIconFile={#SourceDir}\skillbinder.ico
UninstallDisplayIcon={app}\skillbinder.ico
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; The silent update opens the app itself, and only after Restart.
RestartApplications=no
; Uninstalling removes the `bin` folder Settings' Install added to the user's PATH (ADR 0024), and
; this tells running programs the environment changed.
ChangesEnvironment=yes
WizardStyle=modern
Compression=lzma2
SolidCompression=yes

[Files]
Source: "{#SourceDir}\skillbinder.exe"; DestDir: "{app}"; Flags: ignoreversion
; The `skillbinder` command, a console program (ADR 0022).
Source: "{#SourceDir}\bin\skillbinder.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "{#SourceDir}\skillbinder.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\THIRD_PARTY_NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion

; No [UninstallDelete]: uninstalling removes only what setup installed (spec assumption A11).

[Icons]
Name: "{autoprograms}\SkillBinder"; Filename: "{app}\skillbinder.exe"; WorkingDir: "{app}"; IconFilename: "{app}\skillbinder.ico"

[Run]
Filename: "{app}\skillbinder.exe"; WorkingDir: "{app}"; Description: "Open SkillBinder"; Flags: nowait postinstall skipifsilent
Filename: "{app}\skillbinder.exe"; WorkingDir: "{app}"; Flags: nowait skipifnotsilent; Check: ShouldRelaunch

[Code]
const
  SYNCHRONIZE = $00100000;
  WAIT_OBJECT_0 = 0;
  { How long an update waits for the app to exit. }
  WAIT_MILLISECONDS = 60000;

function OpenProcess(DesiredAccess: Integer; InheritHandle: Integer; ProcessId: Integer): Integer;
  external 'OpenProcess@kernel32.dll stdcall';
function WaitForSingleObject(Handle: Integer; Milliseconds: Integer): Integer;
  external 'WaitForSingleObject@kernel32.dll stdcall';
function CloseHandle(Handle: Integer): Integer;
  external 'CloseHandle@kernel32.dll stdcall';

{ An update waits for the app, process WAITPID, to exit before replacing its files. While it still
  runs after the wait, setup ends without changing anything, and the next quit tries again. }
function InitializeSetup(): Boolean;
var
  Pid: Integer;
  Process: Integer;
begin
  Result := True;
  Pid := StrToIntDef(ExpandConstant('{param:WAITPID|0}'), 0);
  if Pid = 0 then
    Exit;
  Process := OpenProcess(SYNCHRONIZE, 0, Pid);
  { No process to open: it has already exited. }
  if Process = 0 then
    Exit;
  Result := WaitForSingleObject(Process, WAIT_MILLISECONDS) = WAIT_OBJECT_0;
  CloseHandle(Process);
end;

{ After Restart, the silent update opens the new version. }
function ShouldRelaunch(): Boolean;
begin
  Result := ExpandConstant('{param:RELAUNCH|0}') = '1';
end;

{ The `bin` folder without a trailing backslash, in lower case, as PATH entries compare. }
function PathEntryKey(Entry: String): String;
begin
  Result := Lowercase(RemoveBackslashUnlessRoot(Trim(Entry)));
end;

{ Uninstalling removes the app's bin folder from the user's PATH, where Settings' Install added
  it, and keeps every other entry as it was. }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Paths, Bin, Kept, Entry: String;
  Split: Integer;
  First, Found: Boolean;
begin
  if CurUninstallStep <> usUninstall then
    Exit;
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Paths) then
    Exit;
  Bin := PathEntryKey(ExpandConstant('{app}\bin'));
  Kept := '';
  First := True;
  Found := False;
  repeat
    Split := Pos(';', Paths);
    if Split = 0 then begin
      Entry := Paths;
      Paths := '';
    end else begin
      Entry := Copy(Paths, 1, Split - 1);
      Delete(Paths, 1, Split);
    end;
    if PathEntryKey(Entry) = Bin then
      Found := True
    else begin
      if not First then
        Kept := Kept + ';';
      Kept := Kept + Entry;
      First := False;
    end;
  until Split = 0;
  if Found then
    RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Kept);
end;
