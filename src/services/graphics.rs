//! How the window is drawn.
//!
//! With vsync, each frame waits for the compositor before the next one can be drawn. With the
//! NVIDIA driver on Wayland that wait can stop for good, after the window was hidden or the screen
//! went off: the window then stops updating and ignores clicks, while the app itself sits idle.
//! So frames are presented without waiting (`AutoNoVsync`: Mailbox or Immediate) and
//! [`FrameLimiter`] keeps animations at a normal frame rate instead.

use std::time::{Duration, Instant};

use eframe::egui_wgpu::WgpuConfiguration;
use eframe::wgpu::PresentMode;

use crate::constants::layout::MIN_FRAME_INTERVAL;

pub fn wgpu_options() -> WgpuConfiguration {
    let mut options = WgpuConfiguration::default();
    options.surface.present_mode = PresentMode::AutoNoVsync;
    options
}

/// Without vsync nothing slows down an animation, which would redraw as fast as the GPU allows.
/// Waits out the rest of [`MIN_FRAME_INTERVAL`] when frames come back to back.
#[derive(Default)]
pub struct FrameLimiter {
    last: Option<Instant>,
}

impl FrameLimiter {
    pub fn wait(&mut self) {
        if let Some(wait) = self.last.and_then(|last| remaining(last.elapsed(), MIN_FRAME_INTERVAL)) {
            std::thread::sleep(wait);
        }
        self.last = Some(Instant::now());
    }
}

fn remaining(elapsed: Duration, interval: Duration) -> Option<Duration> {
    interval.checked_sub(elapsed).filter(|wait| !wait.is_zero())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_back_to_back_frames_wait() {
        let interval = Duration::from_millis(16);
        assert_eq!(remaining(Duration::from_millis(6), interval), Some(Duration::from_millis(10)));
        assert_eq!(remaining(Duration::from_millis(16), interval), None);
        assert_eq!(remaining(Duration::from_secs(3), interval), None);
    }
}
