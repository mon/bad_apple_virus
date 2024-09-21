use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

use cacao::{
    appkit::{App, AppDelegate, ApplicationActivationOptions},
    geometry::Rect,
    notification_center::Dispatcher,
};
use objc::{class, msg_send, runtime::Object, sel, sel_impl};

use crate::{
    initialize_audio, DeferredWindow, WinCoords, WindowCollection, BASE_HEIGHT, BASE_WIDTH,
    MAX_WINDOWS,
};

pub fn screen_size() -> Rect {
    unsafe {
        let screen: *mut Object = msg_send![class!(NSScreen), mainScreen];
        msg_send![screen, visibleFrame]
    }
}

pub struct BatchHandle;

impl BatchHandle {
    /// Also sets animation time to 0
    pub fn alloc() -> Self {
        unsafe {
            let _: () = msg_send![class!(NSAnimationContext), beginGrouping];
            // let ctx: *mut Object = msg_send![class!(NSAnimationContext), currentContext];
            // let _: () = msg_send![ctx, setDuration: 1.0];
        }
        Self
    }
}

impl Drop for BatchHandle {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![class!(NSAnimationContext), endGrouping];
        }
    }
}

#[derive(Default)]
pub struct BadApple {
    windows: RefCell<WindowCollection>,
    frames: Vec<Option<WinCoords>>,
}

impl BadApple {
    pub fn generate_windows(frames: Vec<Option<WinCoords>>) -> Self {
        let _batch = BatchHandle::alloc();
        let wins = (0..MAX_WINDOWS).map(|_| DeferredWindow::new()).collect();

        Self {
            windows: RefCell::new(WindowCollection::new(wins)),
            frames,
        }
    }

    fn tick(&self, msg: Message) {
        let size = screen_size();
        let ratio_x = size.width as f64 / BASE_WIDTH as f64;
        let ratio_y = size.height as f64 / BASE_HEIGHT as f64;
        match msg {
            Message::Draw => self.windows.borrow_mut().draw(),
            Message::Hide(i) => {
                self.windows
                    .borrow_mut()
                    .wins
                    .iter_mut()
                    .skip(i)
                    .for_each(|w| w.set_visible(false));
            }
            Message::TickWindow { frame, win } => {
                let frame = self.frames[frame].unwrap();
                let mut wins = self.windows.borrow_mut();
                wins.wins[win].set_visible(true);
                // Mac screens start from bottom left, not top left
                let corrected_y = size.height - ratio_y * (frame.y as f64 + frame.h.get() as f64);
                wins.wins[win].set_pos((frame.x as f64 * ratio_x) as i32, corrected_y as i32);
                wins.wins[win].set_sz(
                    (frame.w.get() as f64 * ratio_x) as i32,
                    (frame.h.get() as f64 * ratio_y) as i32,
                );
            }
        }
    }
}

pub enum Message {
    TickWindow { frame: usize, win: usize },
    Hide(usize),
    Draw,
}

impl Dispatcher for BadApple {
    type Message = Message;

    fn on_ui_message(&self, msg: Self::Message) {
        self.tick(msg);
    }
}

impl AppDelegate for BadApple {
    fn did_finish_launching(&self) {
        let num_windows = self.windows.borrow().len();
        let num_frames = self.frames.len();
        let frame_mask = self.frames.iter().map(|f| f.is_some()).collect::<Vec<_>>();

        std::thread::spawn(move || {
            let (mut next_tick, clock, _manager) = initialize_audio();
            let mut frame_iter = 0..num_frames;
            let frame_duration = Duration::from_millis(33);

            'outer: loop {
                App::activate(ApplicationActivationOptions::ActivateIgnoringOtherApps);
                let start_time = Instant::now();
                let current_tick = clock.time().ticks;
                if next_tick > current_tick {
                    // std::thread::sleep(Duration::from_secs_f64(1.0 / 30.0));
                    continue;
                }

                while current_tick > next_tick {
                    loop {
                        let Some(idx) = frame_iter.next() else {
                            break 'outer;
                        };
                        if !frame_mask[idx] {
                            break;
                        }
                    }
                    next_tick += 1;
                }

                let mut wins = 0..num_windows;
                loop {
                    let Some(val) = frame_iter.next() else {
                        break 'outer;
                    };
                    if !frame_mask[val] {
                        break;
                    }

                    let win = wins.next().unwrap();

                    App::<BadApple, Message>::dispatch_main(Message::TickWindow {
                        frame: val,
                        win,
                    });
                }

                if let Some(win) = wins.next() {
                    App::<BadApple, Message>::dispatch_main(Message::Hide(win));
                }

                App::<BadApple, Message>::dispatch_main(Message::Draw);

                next_tick += 1;
                let elapsed = start_time.elapsed();
                if elapsed < frame_duration {
                    std::thread::sleep(frame_duration - elapsed);
                }
            }

            App::terminate();
        });
    }
}
