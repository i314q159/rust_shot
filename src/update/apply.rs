use crate::config::i18n::strings;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, GetLastError, HANDLE};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
};
use windows::Win32::UI::Shell::{SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::PCWSTR;

use crate::config::i18n::tr1;

const HELPER_NAME: &str = "RustShot-Update.exe";

enum Setup {
    Done,
    Cancelled,
    Failed,
}

pub fn run(parent_pid: u32, source: &Path, app: &Path) -> ! {
    let exe = app.to_path_buf();
    crate::platform::util::log(&format!(
        "apply update: parent={parent_pid} source={} app={} installed={}",
        source.display(),
        exe.display(),
        crate::update::installed_copy_at(&exe)
    ));

    wait_for(parent_pid, Duration::from_secs(20));

    let started = Instant::now();
    while crate::app::worker::mutex_exists() && started.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(120));
    }
    crate::platform::util::log(&format!(
        "apply update: single-instance mutex still held={}",
        crate::app::worker::mutex_exists()
    ));

    std::thread::sleep(Duration::from_millis(900));

    let installed = crate::update::installed_copy_at(&exe);
    let ok = installed
        && match run_setup(source) {
            Setup::Done => true,
            Setup::Failed => {
                std::thread::sleep(Duration::from_secs(3));
                matches!(run_setup(source), Setup::Done)
            }
            Setup::Cancelled => {
                crate::platform::util::log(
                    "apply update: UAC declined, keeping the current install",
                );
                false
            }
        };
    crate::platform::util::log(&format!(
        "apply update finished ok={ok} installed={installed}"
    ));

    if let Err(e) = std::process::Command::new(&exe).arg("--panel").spawn() {
        crate::platform::util::log(&format!("apply update: cannot restart the app: {e}"));
    }
    std::process::exit(if ok { 0 } else { 1 });
}

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

fn run_setup(setup: &Path) -> Setup {
    let verb = crate::platform::util::wide("runas");
    let file = crate::platform::util::wide(&setup.display().to_string());

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
        let err = unsafe { GetLastError() };
        crate::platform::util::log(&format!("apply update: ShellExecuteEx failed ({})", err.0));
        return if err == ERROR_CANCELLED {
            Setup::Cancelled
        } else {
            Setup::Failed
        };
    }
    if info.hProcess.is_invalid() {
        return Setup::Failed;
    }
    let process = Handle(info.hProcess);

    let _ = unsafe { WaitForSingleObject(process.0, 10 * 60 * 1000) };
    let mut code = 0u32;
    let _ = unsafe { GetExitCodeProcess(process.0, &mut code) };
    crate::platform::util::log(&format!("apply update: setup exit code {code}"));
    if code == 0 {
        Setup::Done
    } else {
        Setup::Failed
    }
}

pub fn spawn_helper(source: &Path) -> Result<(), String> {
    let exe =
        std::env::current_exe().map_err(|e| tr1(strings::CANNOT_LOCATE_THE_RUNNING_PROGRAM, e))?;
    let helper = helper_copy(&exe)?;
    std::process::Command::new(helper)
        .args(apply_args(source, &exe))
        .spawn()
        .map(|_| ())
        .map_err(|e| tr1(strings::CANNOT_START_THE_UPDATER, e))
}

fn apply_args(source: &Path, app: &Path) -> Vec<std::ffi::OsString> {
    vec![
        "--apply-update".into(),
        std::process::id().to_string().into(),
        source.into(),
        app.into(),
    ]
}

fn helper_copy(exe: &Path) -> Result<PathBuf, String> {
    let dir = crate::update::update_dir();
    let copied = std::fs::create_dir_all(&dir)
        .and_then(|_| std::fs::copy(exe, dir.join(HELPER_NAME)).map(|_| ()));
    match copied {
        Ok(()) => Ok(dir.join(HELPER_NAME)),
        Err(e) => {
            crate::platform::util::log(&format!(
                "apply update: cannot stage the helper ({e}), running it in place"
            ));
            Ok(exe.to_path_buf())
        }
    }
}

struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::apply_args;
    use std::ffi::OsString;
    use std::path::Path;

    #[test]
    fn helper_gets_the_installed_exe_not_the_temp_copy() {
        let args = apply_args(
            Path::new(r"C:\Temp\RustShot\update\RustShot-Setup-0.1.9.exe"),
            Path::new(r"C:\Program Files\RustShot\rust_shot.exe"),
        );
        assert_eq!(args.len(), 4);
        assert_eq!(args[0], OsString::from("--apply-update"));
        assert_eq!(
            args[2],
            OsString::from(r"C:\Temp\RustShot\update\RustShot-Setup-0.1.9.exe")
        );
        assert_eq!(
            args[3],
            OsString::from(r"C:\Program Files\RustShot\rust_shot.exe")
        );
    }
}
