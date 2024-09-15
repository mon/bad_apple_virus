use std::{io::Cursor, num::NonZeroU8};

use kira::{
    clock::{ClockHandle, ClockSpeed},
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::streaming::{StreamingSoundData, StreamingSoundSettings},
};

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

// get this from `bad apple.py`
const MAX_WINDOWS: usize = 155;
const BASE_WIDTH: u8 = 64;
const BASE_HEIGHT: u8 = 48;

// Focus on small binary size: width/height are NonZero so Option<>
// becomes free!
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WinCoords {
    pub x: u8,
    pub y: u8,
    pub w: NonZeroU8,
    pub h: NonZeroU8,
}

pub fn initialize_audio() -> (u64, ClockHandle, AudioManager<DefaultBackend>) {
    // Audio playback
    let cursor = Cursor::new(include_bytes!("../assets/bad apple.ogg"));
    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
    let clock = manager
        .add_clock(ClockSpeed::TicksPerSecond(30f64))
        .unwrap();
    let next_tick = clock.time().ticks;
    let sound_data = StreamingSoundData::from_cursor(
        cursor,
        StreamingSoundSettings::new().start_time(clock.time()),
    )
    .unwrap();
    manager.play(sound_data).unwrap();
    clock.start().unwrap();

    (next_tick, clock, manager)
}

pub struct DeferredWindow {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "macos")]
    hnd: cacao::appkit::window::Window,
    x: i32,
    y: i32,
    pos_stale: bool,
    w: i32,
    h: i32,
    sz_stale: bool,
    visible: bool,
    visible_stale: bool,
}

impl Default for DeferredWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl DeferredWindow {
    #[cfg(target_os = "windows")]
    pub fn new_from_hwnd(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> Self {
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

    #[cfg(target_os = "macos")]
    pub fn new() -> Self {
        use cacao::appkit::window::*;
        use cacao::geometry::*;

        let hnd = Window::new(cacao::appkit::window::WindowConfig {
            style: WindowStyle::Titled.into(),
            initial_dimensions: Rect::new(10.0, 10.0, BASE_WIDTH as f64, BASE_HEIGHT as f64),
            defer: true,
            toolbar_style: cacao::appkit::window::WindowToolbarStyle::Expanded,
        });
        hnd.set_ignores_mouse_events(true);
        hnd.set_accepts_mouse_moved_events(false);
        hnd.set_title("Bad Apple!!");
        hnd.set_background_color(cacao::color::Color::rgb(255, 255, 255));
        hnd.set_is_visible(true);
        hnd.set_has_shadow(false);

        Self {
            hnd,
            x: 10,
            y: 10,
            pos_stale: true,
            w: BASE_WIDTH as i32,
            h: BASE_HEIGHT as i32,
            sz_stale: false,
            visible: true,
            visible_stale: true,
        }
    }

    #[cfg(target_os = "windows")]
    pub fn new() -> Self {
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

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.pos_stale = self.x != x || self.y != y;
        self.x = x;
        self.y = y;
    }

    pub fn set_sz(&mut self, w: i32, h: i32) {
        self.sz_stale = self.w != w || self.h != h;
        self.w = w;
        self.h = h;
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible_stale = self.visible != visible;
        self.visible = visible;
    }

    pub fn stale(&self) -> bool {
        self.pos_stale || self.sz_stale || self.visible_stale
    }

    #[cfg(target_os = "macos")]
    pub fn draw(&mut self) {
        if self.visible_stale {
            self.hnd.set_is_visible(self.visible);
            if !self.visible {
                return;
            }
        }

        if !(self.sz_stale || self.pos_stale) {
            return;
        }

        self.visible_stale = false;
        self.pos_stale = false;
        self.sz_stale = false;

        self.hnd.set_scale(cacao::geometry::Rect::new(
            self.y as f64,
            self.x as f64,
            self.w as f64,
            self.h as f64,
        ));
    }

    #[cfg(target_os = "windows")]
    pub fn draw(&mut self, hwinposinfo: isize) -> isize {
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

#[derive(Default)]
pub struct WindowCollection {
    wins: Vec<DeferredWindow>,
}

impl WindowCollection {
    pub fn new(wins: Vec<DeferredWindow>) -> Self {
        Self { wins }
    }

    pub fn changed(&self) -> usize {
        self.wins.iter().filter(|x| x.stale()).count()
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.wins.len()
    }

    #[cfg(target_os = "macos")]
    pub fn draw(&mut self) {
        let changed = self.changed();
        if changed == 0 {
            return;
        }

        let _hnd = macos::BatchHandle::alloc();
        for win in self.wins.iter_mut().filter(|x| x.stale()) {
            win.draw();
        }
    }

    #[cfg(target_os = "windows")]
    pub fn draw(&mut self) {
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
