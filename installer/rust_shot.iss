
#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif

#define MyAppName "RustShot"
#define MyAppPublisher "i314q159"
#define MyAppExeName "rust_shot.exe"
#define MyAppURL "https://github.com/i314q159/rust_shot"
#define MyAppId "{{B7E1C0F2-6B4A-4E1D-9A3C-2F5D8C1A7E64}"

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
VersionInfoCompany={#MyAppPublisher}
VersionInfoProductName={#MyAppName}
VersionInfoDescription={#MyAppName} Setup
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
AllowNoIcons=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\outputs
OutputBaseFilename=RustShot-Setup-{#MyAppVersion}
SetupIconFile=..\assets\icon\rustshot.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName} {#MyAppVersion}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
ChangesEnvironment=yes
AppMutex=RustShotSingleInstance
ShowLanguageDialog=yes
UsePreviousLanguage=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl,lang\English.isl"
Name: "chinesesimplified"; MessagesFile: "compiler:Default.isl,lang\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:TaskDesktopIcon}"; GroupDescription: "{cm:TasksGroup}"
Name: "startup"; Description: "{cm:TaskStartup}"; GroupDescription: "{cm:TasksGroup}"
Name: "addtopath"; Description: "{cm:TaskAddToPath}"; GroupDescription: "{cm:TasksGroup}"

[Files]
Source: "..\target\release\rust_shot.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE-MIT.md"; DestDir: "{app}"; Flags: ignoreversion

[InstallDelete]
Type: files; Name: "{autoprograms}\{#MyAppName}.lnk"
Type: files; Name: "{autoprograms}\{cm:UninstallShortcut}.lnk"
Type: files; Name: "{autoprograms}\卸载 RustShot.lnk"

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "{cm:AppComment}"
Name: "{group}\{cm:UninstallShortcut}"; Filename: "{uninstallexe}"; Comment: "{cm:UninstallShortcut}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "{cm:AppComment}"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; \
    ValueName: "{#MyAppName}"; ValueData: """{app}\{#MyAppExeName}"" --tray"; \
    Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:RunNow}"; Flags: nowait postinstall skipifsilent

[CustomMessages]
english.TasksGroup=Additional tasks:
chinesesimplified.TasksGroup=附加设置：
english.TaskDesktopIcon=Create a desktop shortcut
chinesesimplified.TaskDesktopIcon=创建桌面快捷方式
english.TaskStartup=Start with Windows (tray only, no panel)
chinesesimplified.TaskStartup=登录 Windows 时自动启动（常驻托盘，不弹面板）
english.TaskAddToPath=Add the install folder to PATH (run rust_shot from any console)
chinesesimplified.TaskAddToPath=把安装目录加入 PATH（命令行里可直接输入 rust_shot）
english.AppComment=Screenshot, annotate, copy
chinesesimplified.AppComment=截图 / 标注 / 一键复制
english.UninstallShortcut=Uninstall RustShot
chinesesimplified.UninstallShortcut=卸载 RustShot
english.RunNow=Launch RustShot
chinesesimplified.RunNow=立即启动 RustShot
english.PathFailed=Could not write PATH, so the install folder was not added to the command line path.
chinesesimplified.PathFailed=无法写入 PATH，安装目录未加入命令行路径。
english.PathFailedHint=You can add this folder to PATH by hand:
chinesesimplified.PathFailedHint=可以手动把这个目录加入 PATH：

[Code]

function EnvRoot: Integer;
begin
  if IsAdminInstallMode then
    Result := HKLM
  else
    Result := HKCU;
end;

function EnvSubKey: String;
begin
  if IsAdminInstallMode then
    Result := 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
  else
    Result := 'Environment';
end;

function NeedsAddPath(Param: string): Boolean;
var
  OrigPath: String;
begin
  if not RegQueryStringValue(EnvRoot, EnvSubKey, 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(Param) + ';', ';' + Uppercase(OrigPath) + ';') = 0;
end;

procedure AddToPath(const Dir: String);
var
  OrigPath: String;
begin
  if not NeedsAddPath(Dir) then
    exit;
  if RegQueryStringValue(EnvRoot, EnvSubKey, 'Path', OrigPath) and (OrigPath <> '') then
  begin
    if Copy(OrigPath, Length(OrigPath), 1) <> ';' then
      OrigPath := OrigPath + ';';
    OrigPath := OrigPath + Dir;
  end
  else
    OrigPath := Dir;
  if not RegWriteExpandStringValue(EnvRoot, EnvSubKey, 'Path', OrigPath) then
    MsgBox(ExpandConstant('{cm:PathFailed}') + #13#10 +
           ExpandConstant('{cm:PathFailedHint}') + ' ' + Dir,
           mbInformation, MB_OK);
end;

procedure RemoveFromPath(const Dir: String);
var
  OrigPath, Work: String;
begin
  if not RegQueryStringValue(EnvRoot, EnvSubKey, 'Path', OrigPath) then
    exit;
  Work := ';' + OrigPath + ';';
  while Pos(';' + Uppercase(Dir) + ';', Uppercase(Work)) > 0 do
    StringChangeEx(Work, ';' + Dir + ';', ';', True);
  if (Length(Work) > 0) and (Copy(Work, 1, 1) = ';') then
    Delete(Work, 1, 1);
  while (Length(Work) > 0) and (Copy(Work, Length(Work), 1) = ';') do
    Delete(Work, Length(Work), 1);
  RegWriteExpandStringValue(EnvRoot, EnvSubKey, 'Path', Work);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
    AddToPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
end;
