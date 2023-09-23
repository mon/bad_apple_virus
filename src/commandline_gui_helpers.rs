use lazy_static::lazy_static;
use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

lazy_static! {
    static ref COULD_ATTACH_CONSOLE: bool = unsafe { AttachConsole(ATTACH_PARENT_PROCESS).into() };
}

/// If you launch your `#![windows_subsystem = "windows"]` .exe inside a
/// terminal, this will attach console output to that terminal.
pub fn init() {
    let _ = *COULD_ATTACH_CONSOLE;
}
