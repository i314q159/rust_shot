//! Hidden helper mode: finishes a silent update after the app has exited.
//!
//! The app downloads and verifies the new build, spawns
//! `rust_shot.exe --apply-update <pid> <file>` and quits. The helper waits for
//! that pid to disappear, runs the setup silently and starts the app again.
//! Auto-update is installed-copy only; portable builds are updated by hand.
//!
//! It never shows a window of its own; the only prompt is the UAC consent
//! Windows puts in front of the installer.

use crate::config::i18n::strings;
use std::path::Path;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
};
use windows::Win32::UI::Shell::{SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::PCWSTR;

use crate::config::i18n::tr1;

/// Entry point of `--apply-update`. Never returns.
///
/// `app` is the exe to replace / restart (the helper itself runs from `%TEMP%`
/// so the installed `rust_shot.exe` is not "in use" while the setup runs).
pub fn run(parent_pid: u32, source: &Path, app: &Path) -> ! {
    let exe = app.to_path_buf();
    crate::platform::util::log(&format!(
        "apply update: parent={parent_pid} source={} app={} installed={}",
        source.display(),
        exe.display(),
        crate::update::installed_copy_at(&exe)
    ));

    wait_for(parent_pid, Duration::from_secs(20));

    // The installer's AppMutex check aborts silently while any copy still holds
    // the single-instance mutex, so wait for the name to disappear first.
    let started = Instant::now();
    while crate::app::worker::mutex_exists() && started.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(120));
    }
    crate::platform::util::log(&format!(
        "apply update: single-instance mutex still held={}",
        crate::app::worker::mutex_exists()
    ));

    // Windows can still list the old process in the Restart Manager for a
    // moment after it leaves the message loop; the installer would then abort.
    std::thread::sleep(Duration::from_millis(900));

    // Auto-update is for the installed copy only (the portable exe is updated
    // by downloading a fresh portable zip), so anything else just restarts.
    let installed = crate::update::installed_copy_at(&exe);
    let ok = installed
        && (run_setup(source) || {
            // One retry, in case the first attempt raced the teardown.
            std::thread::sleep(Duration::from_secs(3));
            run_setup(source)
        });
    crate::platform::util::log(&format!(
        "apply update finished ok={ok} installed={installed}"
    ));

    // Whether it worked or not, bring the app back: on failure the old build is
    // still in place, so the user keeps a working tool instead of nothing.
    if let Err(e) = std::process::Command::new(&exe).arg("--panel").spawn() {
        crate::platform::util::log(&format!("apply update: cannot restart the app: {e}"));
    }
    std::process::exit(if ok { 0 } else { 1 });
}

/// Block until `pid` exits (or the timeout passes, so a stuck app cannot hang
/// the updater forever).
fn wait_for(pid: u32, timeout: Duration) {
    if pid == 0 {
        return;
    }
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }) else {
        return;
    };
    let handle = Handle(handle);
    let millis = timeout.as_millis().min(u32::MAX as u128) as u32;
    let _ = unsafe { WaitForSingleObject(handle.0, millis) };
}

/// `RustShot-Setup-<ver>.exe /VERYSILENT …` elevated, then wait for it.
fn run_setup(setup: &Path) -> bool {
    let verb = crate::platform::util::wide("runas");
    let file = crate::platform::util::wide(&setup.display().to_string());
    // The installer keeps its own log so a failed silent run can be diagnosed
    // from rustshot.log / install.log.
    let log_path = crate::update::update_dir().join("install.log");
    let params = crate::platform::util::wide(&format!(
        "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOCLOSEAPPLICATIONS /LOG=\"{}\"",
        log_path.display()
    ));
    let dir = setup
        .parent()
        .map(|p| crate::platform::util::wide(&p.display().to_string()));

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        lpDirectory: dir
            .as_ref()
            .map(|d| PCWSTR(d.as_ptr()))
            .unwrap_or_else(PCWSTR::null),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    if unsafe { windows::Win32::UI::Shell::ShellExecuteExW(&mut info) }.is_err() {
        // A cancelled UAC prompt lands here (ERROR_CANCELLED): nothing was
        // installed, the current build is untouched.
        crate::platform::util::log(
            "apply update: ShellExecuteEx failed (user cancelled or blocked)",
        );
        return false;
    }
    if info.hProcess.is_invalid() {
        return false;
    }
    let process = Handle(info.hProcess);

    // Silent install of a 5 MB package takes a few seconds; don't hang forever.
    let _ = unsafe { WaitForSingleObject(process.0, 10 * 60 * 1000) };
    let mut code = 0u32;
    let _ = unsafe { GetExitCodeProcess(process.0, &mut code) };
    crate::platform::util::log(&format!("apply update: setup exit code {code}"));
    code == 0
}

/// Kick off the helper for `source` and return immediately.
pub fn spawn_helper(source: &Path) -> Result<(), String> {
    let exe =
        std::env::current_exe().map_err(|e| tr1(strings::CANNOT_LOCATE_THE_RUNNING_PROGRAM, e))?;
    std::process::Command::new(exe)
        .arg("--apply-update")
        .arg(std::process::id().to_string())
        .arg(source)
        .spawn()
        .map(|_| ())
        .map_err(|e| tr1(strings::CANNOT_START_THE_UPDATER, e))
}

/// Small RAII wrapper so every early return closes the process handle.
struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

/// Keep the unused-import warning away in case the `WAIT_OBJECT_0` check is
/// simplified later.
#[allow(dead_code)]
fn is_signalled(code: u32) -> bool {
    code == WAIT_OBJECT_0.0
}
