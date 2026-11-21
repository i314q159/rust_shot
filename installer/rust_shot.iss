; RustShot — Windows 安装脚本（Inno Setup 6）
; RustShot — Windows installer script (Inno Setup 6)
;
; 编译：installer\build.ps1（推荐，会先 cargo build --release 并同步版本号）
; 或：  ISCC.exe installer\rust_shot.iss /DMyAppVersion=0.1.0
;
; 始终安装到 C:\Program Files\RustShot（所有用户，需要一次 UAC 确认）；
; 不提供用户目录安装选项，卸载与升级都在“设置 → 应用”里完成。
;
; 界面支持 English / 简体中文：安装包启动时弹语言选择（默认跟随系统 UI 语言）。
; 四页向导与卸载器的文案在两个语言文件里（lang\English.isl、lang\ChineseSimplified.isl，
; 都叠在默认的 Default.isl 之上）；任务描述、快捷方式注释、安装完成后启动项等
; 非 [Messages] 文案在本文件的 [CustomMessages] 里，中英各写一份。

#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif

#define MyAppName "RustShot"
#define MyAppPublisher "i314q159"
#define MyAppExeName "rust_shot.exe"
#define MyAppURL "https://github.com/i314q159/rust_shot"
; 固定 AppId：升级时识别为同一个程序，覆盖安装而不产生第二条卸载项。
; "{{" 是 Inno 的字面 "{" 转义，最终注册表键为 {B7E1C0F2-…}_is1。
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
; 固定为全机器安装：始终装到 {autopf}（= C:\Program Files\RustShot），
; 不提供 /CURRENTUSER 覆盖，避免出现 AppData\Local\Programs 这种用户目录安装。
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
; 已有实例在跑时，让安装/卸载先提示退出（对应 Local\RustShotSingleInstance）。
AppMutex=RustShotSingleInstance
; 每次安装都弹语言选择（默认选中系统语言，可手动改）；
; UsePreviousLanguage=no 表示不记住上次的选择，避免旧值影响下一次安装。
; 静默安装（/VERYSILENT）时不会弹，可用 /LANG=english 或 /LANG=chinesesimplified 指定。
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
Source: "..\README.en.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE-MIT.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "{cm:AppComment}"
Name: "{autoprograms}\{cm:UninstallShortcut}"; Filename: "{uninstallexe}"; Comment: "{cm:UninstallShortcut}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "{cm:AppComment}"; Tasks: desktopicon

[Registry]
; 开机自启：--tray 启动后直接驻留托盘，不弹主面板；卸载时自动删掉该值。
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; \
    ValueName: "{#MyAppName}"; ValueData: """{app}\{#MyAppExeName}"" --tray"; \
    Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:RunNow}"; Flags: nowait postinstall skipifsilent

[CustomMessages]
; 任务 / 快捷方式 / 启动项等非 [Messages] 文案；对话框文案在 lang\*.isl 里。
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
{ PATH 读写：管理员安装改 HKLM，/CURRENTUSER 安装改 HKCU。 }

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

{ 目录已在 PATH 里（大小写不敏感）就不重复添加。 }
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
    { 用户 PATH 末尾常带一个分号，避免拼出重复的 ";;" }
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

{ 卸载时只摘掉自己那一段，保留用户其它 PATH 条目与原有顺序。 }
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
