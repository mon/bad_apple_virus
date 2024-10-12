pub mod commandline_gui_helpers;
pub mod util;

use crate::*;
use ::windows::Win32::Graphics::Gdi::CreateSolidBrush;
use ::windows::{
    core::*,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
use std::time::Instant;

pub unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CLOSE {
        println!("WM_CLOSE (wnd_proc)");
    }
    DefWindowProcA(hwnd, msg, wparam, lparam)
}

pub fn register_window_class() {
    // we draw the white parts of the video, so make the background white -
    // because we use SWP_NOREDRAW, this is all we can really use to change
    // colours
    let brush = unsafe { CreateSolidBrush(COLORREF(0xFFFFFF)) };
    let icon = unsafe { LoadIconW(util::get_instance_handle(), PCWSTR(1 as _)).unwrap() };

    let wc = WNDCLASSA {
        lpfnWndProc: Some(wnd_proc),
        hInstance: util::get_instance_handle(),
        lpszClassName: PCSTR(WND_CLASS.as_ptr() as _),
        hbrBackground: brush,
        hIcon: icon,
        ..Default::default()
    };

    unsafe {
        RegisterClassA(&wc);
    };
}

pub fn usable_monitor_sz() -> (i32, i32) {
    let mut sz: RECT = Default::default();
    assert!(unsafe {
        SystemParametersInfoA(
            SPI_GETWORKAREA,
            0,
            Some(&mut sz as *mut _ as _),
            Default::default(),
        )
        .into()
    });

    (
        (sz.right - sz.left).try_into().unwrap(),
        (sz.bottom - sz.top).try_into().unwrap(),
    )
}

pub fn main(frames: &[Option<WinCoords>]) {
    let mut frames_iter = frames.iter();
    unsafe {
        // todo WM_PARENTNOTIFY

        println!("Creating windows...");
        let now = Instant::now();
        let wins: Vec<DeferredWindow> = (0..MAX_WINDOWS).map(|_| DeferredWindow::new()).collect();
        println!("Done! in {:?}", now.elapsed());

        let mut collection = WindowCollection::new(wins);

        let (mut next_tick, clock, _manager) = initialize_audio();

        // println!("Showing windows...");
        // let now = Instant::now();
        // Normal windows (appear in taskbar and alt-tab):
        //   ~15ms per window for 100
        //   ~44ms per window for 500
        // WS_EX_TOOLWINDOW:
        //   ~7ms  per window for 100
        //   ~15ms per window for 500
        // wins.iter().for_each(|win| {ShowWindow(*win, SW_SHOW);});
        // println!("Done! in {:?}", now.elapsed());

        let (usable_x, usable_y) = usable_monitor_sz();
        let ratio_x = usable_x as f32 / BASE_WIDTH as f32;
        let ratio_y = usable_y as f32 / BASE_HEIGHT as f32;

        SetTimer(None, 1, 16, None); // nyquist 30fps

        'outer: loop {
            let mut msg: MSG = std::mem::zeroed();

            //if there was a windows message
            // note: mouse moves only trigger when over the window
            while PeekMessageA(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    println!("WM_QUIT");
                    break;
                } else if msg.message == WM_TIMER {
                    let current_tick = clock.time().ticks;
                    // nothing to do yet, eg next_tick = 1, current_tick = 0
                    if next_tick > current_tick {
                        continue;
                    }
                    // skip any frames that we missed, eg next_tick = 2, current_tick = 3
                    while current_tick > next_tick {
                        loop {
                            let Some(val) = frames_iter.next() else {
                                break 'outer;
                            };
                            let Some(_coords) = val else {
                                break;
                            };
                        }
                        next_tick += 1;
                    }

                    // process the current tick
                    let mut windows = collection.wins.iter_mut();
                    loop {
                        let Some(val) = frames_iter.next() else {
                            break 'outer;
                        };
                        let Some(coords) = val else {
                            break;
                        };

                        let win = windows.next().unwrap();
                        // windows have padding, cbf working out exactly what
                        const FUDGE_X: i32 = 15;
                        const FUDGE_Y: i32 = 8;
                        win.set_pos(
                            (coords.x as f32 * ratio_x) as i32,
                            (coords.y as f32 * ratio_y) as i32,
                        );
                        win.set_sz(
                            (coords.w.get() as f32 * ratio_x) as i32 + FUDGE_X,
                            (coords.h.get() as f32 * ratio_y) as i32 + FUDGE_Y,
                        );
                        win.set_visible(true);
                    }

                    // hide the rest
                    for win in windows {
                        win.set_visible(false);
                    }

                    collection.draw();
                    // WindowCollection::draw_many(&mut [&mut collection, &mut normal_collection]);

                    next_tick += 1;
                }

                #[cfg(target_os = "windows")]
                {
                    TranslateMessage(&msg);
                    DispatchMessageA(&msg);
                }
            }
        }
    }
}
