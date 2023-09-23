#![windows_subsystem = "windows"]

use std::io::Cursor;
use std::num::NonZeroU8;
use std::time::Instant;

use include_bytes_zstd::include_bytes_zstd;
use kira::clock::ClockSpeed;
use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::streaming::{StreamingSoundData, StreamingSoundSettings},
};
use windows::Win32::Graphics::Gdi::CreateSolidBrush;
use windows::{
    core::*,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

mod commandline_gui_helpers;
mod util;

const WND_CLASS: &str = "BadApple\0";

unsafe extern "system" fn wnd_proc(
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

fn register_window_class() {
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

struct DeferredWindow {
    hwnd: HWND,
    x: i32,
    y: i32,
    pos_stale: bool,
    w: i32,
    h: i32,
    sz_stale: bool,
    visible: bool,
    visible_stale: bool,
}

impl DeferredWindow {
    fn new_from_hwnd(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            hwnd,
            x,
            y,
            w,
            h,
            pos_stale: true,
            sz_stale: false,
            visible: true,
            visible_stale: true,
        }
    }

    fn new() -> Self {
        let w = 200;
        let h = 100;
        let x = 10;
        let y = 10;

        // takes about 1ms per window
        let hwnd = unsafe {
            CreateWindowExA(
                // WS_EX_TOPMOST | WS_EX_NOACTIVATE, // Minimize/Maximize/Close
                // WS_EX_TOPMOST | WS_EX_APPWINDOW, // taskbar, Minimize/Maximize/Close
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW, // no taskbar, Close button only
                PCSTR(WND_CLASS.as_ptr() as _),
                s!("Bad Apple!!"),
                WS_OVERLAPPEDWINDOW,
                // WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME,
                // x,y,w,h
                x,
                y,
                w,
                h,
                None,
                None,
                None,
                None,
            )
        };

        assert!(hwnd.0 != 0);

        Self::new_from_hwnd(hwnd, x, y, w, h)
    }

    fn set_pos(&mut self, x: i32, y: i32) {
        self.pos_stale = self.x != x || self.y != y;
        self.x = x;
        self.y = y;
    }

    fn set_sz(&mut self, w: i32, h: i32) {
        self.sz_stale = self.w != w || self.h != h;
        self.w = w;
        self.h = h;
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible_stale = self.visible != visible;
        self.visible = visible;
    }

    fn stale(&self) -> bool {
        self.pos_stale || self.sz_stale || self.visible_stale
    }

    fn draw(&mut self, hwinposinfo: isize) -> isize {
        // SWP_NOACTIVATE: all windows stay grey
        // no SWP_NOACTIVATE: most recent window touched bounces around. Looks kinda cool.
        let mut flags = SWP_NOZORDER /*| SWP_NOACTIVATE*/;

        if !self.sz_stale {
            flags |= SWP_NOSIZE;
        }

        if !self.pos_stale {
            flags |= SWP_NOMOVE;
        }

        if self.visible_stale {
            flags |= if self.visible {
                SWP_SHOWWINDOW
            } else {
                SWP_HIDEWINDOW
            };
        } else {
            // if we're showing a hidden MessageBox or Progress window, we need
            // to redraw or it is often stale
            flags |= SWP_NOREDRAW;
        }

        self.pos_stale = false;
        self.sz_stale = false;
        self.visible_stale = false;

        unsafe {
            DeferWindowPos(
                hwinposinfo,
                self.hwnd,
                None,
                self.x,
                self.y,
                self.w,
                self.h,
                flags,
            )
        }
    }
}

fn usable_monitor_sz() -> (i32, i32) {
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

struct WindowCollection {
    wins: Vec<DeferredWindow>,
}

impl WindowCollection {
    fn new(wins: Vec<DeferredWindow>) -> Self {
        Self { wins }
    }

    fn changed(&self) -> usize {
        self.wins.iter().filter(|x| x.stale()).count()
    }

    fn draw(&mut self) {
        let changed = self.changed() as i32;
        if changed == 0 {
            return;
        }

        let mut hdwp = unsafe { BeginDeferWindowPos(changed) };
        assert!(hdwp != 0);

        for win in self.wins.iter_mut().filter(|x| x.stale()) {
            hdwp = win.draw(hdwp);
            assert!(hdwp != 0);
        }

        unsafe { EndDeferWindowPos(hdwp) };
    }
}

// Focus on small binary size: width/height are NonZero so Option<>
// becomes free!
#[repr(packed)]
#[derive(Debug, Copy, Clone)]
pub struct WinCoords {
    x: u8,
    y: u8,
    w: NonZeroU8,
    h: NonZeroU8,
}

// get this from `bad apple.py`
const MAX_WINDOWS: usize = 155;
const BASE_WIDTH: u8 = 64;
const BASE_HEIGHT: u8 = 48;

fn main() {
    commandline_gui_helpers::init();

    register_window_class();

    let frames_raw = include_bytes_zstd!("assets/boxes.bin", 22);
    let frames: &[Option<WinCoords>] = unsafe {
        std::slice::from_raw_parts(
            frames_raw.as_ptr() as *const _,
            frames_raw.len() / std::mem::size_of::<WinCoords>(),
        )
    };
    let mut frames_iter = frames.iter();
    // println!("{:?}", frames);

    unsafe {
        // todo WM_PARENTNOTIFY

        println!("Creating windows...");
        let now = Instant::now();
        let wins: Vec<DeferredWindow> = (0..MAX_WINDOWS).map(|_| DeferredWindow::new()).collect();
        println!("Done! in {:?}", now.elapsed());

        let mut collection = WindowCollection::new(wins);

        // Audio playback
        let cursor = Cursor::new(include_bytes!("../assets/bad apple.ogg"));
        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
        let clock = manager
            .add_clock(ClockSpeed::TicksPerSecond(30f64))
            .unwrap();
        let mut next_tick = clock.time().ticks;
        let sound_data = StreamingSoundData::from_cursor(
            cursor,
            StreamingSoundSettings::new().start_time(clock.time()),
        )
        .unwrap();
        manager.play(sound_data).unwrap();
        clock.start().unwrap();

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

                TranslateMessage(&msg);
                DispatchMessageA(&msg);
            }
        }
    }
}
