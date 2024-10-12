#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use bad_apple::*;

fn main() {
    #[cfg(target_os = "windows")]
    {
        use bad_apple::windows::*;
        commandline_gui_helpers::init();

        register_window_class();
    }

    assert_eq!(size_of::<WinCoords>(), 4);
    let frames_raw = include_bytes!("../assets/boxes.bin");
    let frames: &[Option<WinCoords>] = unsafe {
        std::slice::from_raw_parts(
            frames_raw.as_ptr() as *const _,
            frames_raw.len() / std::mem::size_of::<WinCoords>(),
        )
    };

    #[cfg(target_os = "macos")]
    {
        let frames = frames.to_vec();
        cacao::appkit::App::new("com.bad.apple", macos::BadApple::generate_windows(frames)).run();
    }

    #[cfg(target_os = "windows")]
    windows::main(frames);
}
