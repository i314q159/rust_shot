









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
        && (run_setup(source) || {
            
            std::thread::sleep(Duration::from_secs(3));
            run_setup(source)
        });
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


fn run_setup(setup: &Path) -> bool {
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
        
        
        crate::platform::util::log(
            "apply update: ShellExecuteEx failed (user cancelled or blocked)",
        );
        return false;
    }
    if info.hProcess.is_invalid() {
        return false;
    }
    let process = Handle(info.hProcess);

    
    let _ = unsafe { WaitForSingleObject(process.0, 10 * 60 * 1000) };
    let mut code = 0u32;
    let _ = unsafe { GetExitCodeProcess(process.0, &mut code) };
    crate::platform::util::log(&format!("apply update: setup exit code {code}"));
    code == 0
}


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


struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}



#[allow(dead_code)]
fn is_signalled(code: u32) -> bool {
    code == WAIT_OBJECT_0.0
}
