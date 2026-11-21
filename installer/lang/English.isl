; RustShot English message overrides.
;
; Default.isl already provides English; this file only rewrites the texts of the
; four-page wizard and the uninstaller to match what the Chinese version says.
; Task descriptions, shortcut comments and the post-install entry come from
; [CustomMessages] in rust_shot.iss (english. prefix).

[LangOptions]
LanguageName=English
LanguageID=$0409
DialogFontName=Segoe UI
DialogFontSize=9
WelcomeFontName=Segoe UI
WelcomeFontSize=12

[Messages]
SetupWindowTitle=Setup - %1
WelcomeLabel1=Welcome to the [name] Setup Wizard
WelcomeLabel2=This will install [name/ver] on your computer.%n%nRustShot goes to C:\Program Files\RustShot and is available to all users; uninstall from Settings > Apps.%n%nClose other applications before continuing.
WizardSelectTasks=Select Additional Tasks
SelectTasksDesc=Which additional tasks should Setup perform?
SelectTasksLabel2=Pick what you need, then click Next. Everything here can be changed later in the uninstaller or in Windows Settings.
WizardInstalling=Installing
InstallingLabel=Setup is installing [name], please wait...
FinishedHeadingLabel=RustShot is installed
FinishedLabel=RustShot is installed. Start it from the Start Menu, or press F1 to capture.
FinishedLabelNoIcons=RustShot is installed.
ClickFinish=Click Finish to exit Setup.
ButtonBack=< &Back
ButtonNext=&Next >
ButtonInstall=&Install
ButtonCancel=Cancel
ButtonFinish=&Finish
ButtonOK=OK
ButtonYes=&Yes
ButtonNo=&No
SetupAlreadyRunning=Setup is already running.
SetupAppRunningError=RustShot is still running.%n%nQuit it first (tray icon > Quit, or Ctrl+Alt+Q), then click OK to continue.
UninstallAppRunningError=RustShot is still running.%n%nQuit it first, then click OK to continue.
UninstallAppTitle=Uninstall
UninstallAppFullTitle=Uninstall RustShot
ConfirmUninstall=Remove %1?%n%nScreenshots are not deleted. Personal settings stay in LocalAppData\RustShot and can be removed by hand.
UninstallStatusLabel=Removing %1 from your computer, please wait...
UninstalledAll=%1 was removed.
UninstalledMost=%1 was removed, but some files could not be deleted (they may still be in use). Clean the install folder later if needed.
WizardUninstalling=Uninstalling
