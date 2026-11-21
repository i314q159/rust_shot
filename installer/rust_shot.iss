; RustShot — Windows 安装脚本（Inno Setup 6）
;
; 编译：installer\build.ps1（推荐，会先 cargo build --release 并同步版本号）
; 或：  ISCC.exe installer\rust_shot.iss /DMyAppVersion=0.1.0
;
; 始终安装到 C:\Program Files\RustShot（所有用户，需要一次 UAC 确认）；
; 不提供用户目录安装选项，卸载与升级都在“设置 → 应用”里完成。

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
VersionInfoDescription={#MyAppName} 安装程序
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

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加设置："
Name: "startup"; Description: "登录 Windows 时自动启动（常驻托盘，不弹面板）"; GroupDescription: "附加设置："
Name: "addtopath"; Description: "把安装目录加入 PATH（命令行里可直接输入 rust_shot）"; GroupDescription: "附加设置："

[Files]
Source: "..\target\release\rust_shot.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE-MIT.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "截图 / 标注 / 一键复制"
Name: "{autoprograms}\卸载 {#MyAppName}"; Filename: "{uninstallexe}"; Comment: "卸载 {#MyAppName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "截图 / 标注 / 一键复制"; Tasks: desktopicon

[Registry]
; 开机自启：--tray 启动后直接驻留托盘，不弹主面板；卸载时自动删掉该值。
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; \
    ValueName: "{#MyAppName}"; ValueData: """{app}\{#MyAppExeName}"" --tray"; \
    Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "立即启动 {#MyAppName}"; Flags: nowait postinstall skipifsilent

[Messages]
; 只保留“欢迎 → 附加任务 → 安装 → 完成”四页，以下覆盖这四页与卸载器里会出现的英文。
SetupWindowTitle=安装 {#MyAppName} - %1
WelcomeLabel1=欢迎安装 [name]
WelcomeLabel2=即将在电脑上安装 [name/ver]。%n%n默认安装到 C:\Program Files\RustShot，所有用户均可使用；卸载可在“设置 → 应用”中完成。%n%n继续前建议保存并关闭其它程序。
WizardSelectTasks=选择附加设置
SelectTasksDesc=安装时需要一并完成哪些设置？
SelectTasksLabel2=勾选需要的项目后点击“下一步”。这些设置之后都可以用卸载程序或系统设置修改。
WizardInstalling=正在安装
InstallingLabel=正在安装 [name]，请稍候……
FinishedHeadingLabel={#MyAppName} 安装完成
FinishedLabel={#MyAppName} 已安装到电脑上。可以从开始菜单启动，或直接按 F1 截图。
FinishedLabelNoIcons={#MyAppName} 已安装到电脑上。
ClickFinish=点击“完成”退出安装程序。
ButtonBack=< 上一步(&B)
ButtonNext=下一步(&N) >
ButtonInstall=安装(&I)
ButtonCancel=取消
ButtonFinish=完成(&F)
ButtonOK=确定
ButtonYes=是(&Y)
ButtonNo=否(&N)
SetupAlreadyRunning=安装程序已经在运行。
SetupAppRunningError=检测到 {#MyAppName} 正在运行。%n%n请先退出 RustShot（托盘图标右键 → 退出，或按 Ctrl+Alt+Q），再点击“确定”继续。
UninstallAppRunningError=检测到 {#MyAppName} 正在运行。%n%n请先退出 RustShot，再点击“确定”继续。
UninstallAppTitle=卸载程序
UninstallAppFullTitle=卸载 {#MyAppName}
ConfirmUninstall=确定要卸载 %1 吗？%n%n截图文件不会被删除；个人设置保留在 LocalAppData\RustShot 目录，可手动清理。
UninstallStatusLabel=正在从电脑上移除 %1，请稍候……
UninstalledAll=%1 已成功卸载。
UninstalledMost=%1 卸载完成，但有部分文件无法删除（可能仍在使用），可以稍后手动清理安装目录。
WizardUninstalling=卸载状态

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
    MsgBox('无法写入 PATH，安装目录未加入命令行路径。' + #13#10 +
           '可以手动把 ' + Dir + ' 加入 PATH。', mbInformation, MB_OK);
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