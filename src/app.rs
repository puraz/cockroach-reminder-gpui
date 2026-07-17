//! Shared application state and sprite loading.

use crate::cockroach::{AnimConfig, Cockroach};
use crate::config::Settings;
use crate::constants::{FRAME_BYTES, ORIENTATION_COUNT};
use crate::overlay_view::RenderedFrame;
use crate::platform;
use crate::timer::{Phase, Timer};

use gpui::{ImageSource, RenderImage};
use image::imageops::FilterType;
use rand::Rng;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

/// A decoded sprite frame stored as BGRA pixel data (ready for GPU upload via RenderImage).
#[derive(Clone)]
pub struct DecodedFrame {
    pub pixels: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub display_width: f32,
    pub display_height: f32,
    pub body_anchor_x: f32,
    pub body_anchor_y: f32,
}

const PIXEL_SPRITE_FRAME_WIDTH: u32 = 384;
const DISPLAY_SPRITE_FRAME_WIDTH: f32 = 640.0;

/// Per-display overlay state.
pub struct OverlayState {
    pub x: f64,
    pub y: f64,
    pub width: f32,
    pub height: f32,
    pub cockroaches: Vec<Cockroach>,
    pub anim_start: Instant,
}

/// Central shared application state.
pub struct AppState {
    pub settings: Settings,
    pub timer: Timer,
    pub frames: Option<Rc<Vec<RenderedFrame>>>,
    pub overlays: Vec<OverlayState>,
    pub settings_view_opened: bool,
    pub rng: rand::rngs::ThreadRng,
    runtime_waker: smol::channel::Sender<()>,
}

impl AppState {
    pub fn new(runtime_waker: smol::channel::Sender<()>) -> Self {
        let settings = Settings::load();
        let mut timer = Timer::new(settings.interval_minutes, settings.duration_seconds);
        if settings.auto_start {
            timer.start();
        }
        Self {
            settings,
            timer,
            frames: None,
            overlays: Vec::new(),
            settings_view_opened: false,
            rng: rand::thread_rng(),
            runtime_waker,
        }
    }

    pub fn wake_runtime(&self) {
        let _ = self.runtime_waker.try_send(());
    }

    pub fn anim_config(&self) -> AnimConfig {
        AnimConfig {
            size_percent: self.settings.cockroach_size_percent,
            normal_fps: self.settings.normal_speed_fps,
            fast_min_fps: self.settings.fast_speed_min_fps,
            fast_max_fps: self.settings.fast_speed_max_fps,
            fast_probability: self.settings.fast_speed_probability,
            movement_percent: self.settings.movement_percent,
        }
    }

    pub fn refresh_tray_labels(&self) -> (String, String, bool, String) {
        let f = self.timer.formatted();
        let (status, tooltip) = match self.timer.phase {
            Phase::Running => (
                format!("⏱ 下次休息还有 {f}"),
                format!("🪳 下次休息还有 {f}"),
            ),
            Phase::Break => (
                format!("🪳 休息中！还剩 {f}"),
                format!("🪳 休息时间！还剩 {f}"),
            ),
            Phase::Paused => (
                format!("⏸ 已暂停 — 剩余 {f}"),
                format!("🪳 已暂停 — 剩余 {f}"),
            ),
            Phase::Idle => (
                "⏹ 计时器已停止".to_string(),
                "🪳 蟑螂提醒 (已停止)".to_string(),
            ),
        };
        let is_running = self.timer.phase == Phase::Running;
        let is_paused = self.timer.phase == Phase::Paused;
        let toggle_label = if is_running {
            "⏸  暂停计时"
        } else {
            "▶  恢复计时"
        };
        let toggle_enabled = is_running || is_paused;
        (status, toggle_label.to_string(), toggle_enabled, tooltip)
    }

    pub fn prepare_overlays(&mut self) {
        if self.frames.is_none() {
            return;
        }
        if self.settings.show_notifications {
            notify("🪳 休息时间到！", "该放松一下眼睛了！");
        }
        let mut screens = platform::screen_frames();
        if screens.is_empty() {
            screens.push(platform::ScreenFrame {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            });
        }
        self.overlays.clear();
        let cfg = self.anim_config();
        let count = self.settings.cockroach_count;
        for sf in screens {
            let w = sf.width as f32;
            let h = sf.height as f32;
            let roaches = Self::seed_cockroaches(count, cfg, &mut self.rng, w, h);
            self.overlays.push(OverlayState {
                x: sf.x,
                y: sf.y,
                width: w,
                height: h,
                cockroaches: roaches,
                anim_start: Instant::now(),
            });
        }
    }

    pub fn clear_overlays(&mut self) {
        self.overlays.clear();
    }

    pub fn update_animation(&mut self, now: Instant) {
        for ov in &mut self.overlays {
            let ms = now.duration_since(ov.anim_start).as_secs_f32() * 1000.0;
            for roach in &mut ov.cockroaches {
                roach.update(&mut self.rng, ms, ov.width, ov.height);
            }
        }
    }

    fn seed_cockroaches(
        count: u32,
        cfg: AnimConfig,
        rng: &mut impl Rng,
        w: f32,
        h: f32,
    ) -> Vec<Cockroach> {
        (0..count).map(|_| Cockroach::new(rng, cfg, w, h)).collect()
    }
}

/// Load & decode sprite frames from embedded PNGs.
pub fn load_sprite_frame_data() -> Vec<DecodedFrame> {
    let (min_x, min_y, max_x, max_y) =
        FRAME_BYTES
            .iter()
            .fold((u32::MAX, u32::MAX, 0u32, 0u32), |acc, bytes| {
                let image = image::load_from_memory(bytes)
                    .expect("PNG frame")
                    .into_rgba8();
                let (a, b, c, d) = alpha_bounds(&image);
                (acc.0.min(a), acc.1.min(b), acc.2.max(c), acc.3.max(d))
            });

    let crop_w = max_x - min_x + 1;
    let crop_h = max_y - min_y + 1;

    FRAME_BYTES
        .iter()
        .map(|bytes| {
            let image = image::load_from_memory(bytes)
                .expect("PNG frame")
                .into_rgba8();
            let (body_x, body_y) = alpha_centroid(&image);
            let cropped =
                image::imageops::crop_imm(&image, min_x, min_y, crop_w, crop_h).to_image();
            let final_img = if crop_w > PIXEL_SPRITE_FRAME_WIDTH {
                let s = PIXEL_SPRITE_FRAME_WIDTH as f32 / crop_w as f32;
                let rh = (crop_h as f32 * s).round().max(1.0) as u32;
                image::imageops::resize(
                    &cropped,
                    PIXEL_SPRITE_FRAME_WIDTH,
                    rh,
                    FilterType::Lanczos3,
                )
            } else {
                cropped
            };
            let display_scale = (DISPLAY_SPRITE_FRAME_WIDTH / crop_w as f32).min(1.0);

            let (fw, fh) = final_img.dimensions();
            let mut bgra = final_img.into_raw();
            for c in bgra.as_chunks_mut::<4>().0 {
                c.swap(0, 2);
            }

            DecodedFrame {
                pixels: Arc::new(bgra),
                width: fw,
                height: fh,
                display_width: crop_w as f32 * display_scale,
                display_height: crop_h as f32 * display_scale,
                body_anchor_x: (body_x - min_x as f32) * display_scale,
                body_anchor_y: (body_y - min_y as f32) * display_scale,
            }
        })
        .collect()
}

/// Convert a DecodedFrame to a GPUI ImageSource (RenderImage).
pub fn frame_to_image_source(frame: DecodedFrame) -> ImageSource {
    use image::Frame;
    let pixels = Arc::try_unwrap(frame.pixels).unwrap_or_else(|pixels| (*pixels).clone());
    let buf =
        image::ImageBuffer::from_raw(frame.width, frame.height, pixels).expect("valid img buf");
    let img_frame = Frame::new(buf);
    let ri = Arc::new(RenderImage::new(smallvec::smallvec![img_frame]));
    ImageSource::Render(ri)
}

/// Rotate a decoded sprite clockwise while preserving its logical body anchor.
pub fn rotate_frame(frame: &DecodedFrame, angle_degrees: f32) -> DecodedFrame {
    let image = image::RgbaImage::from_raw(frame.width, frame.height, (*frame.pixels).clone())
        .expect("valid decoded sprite");
    let radians = angle_degrees.to_radians();
    let (raw_sin, raw_cos) = radians.sin_cos();
    let sin = if raw_sin.abs() < 0.000_001 {
        0.0
    } else {
        raw_sin
    };
    let cos = if raw_cos.abs() < 0.000_001 {
        0.0
    } else {
        raw_cos
    };
    let w = frame.width as f32;
    let h = frame.height as f32;
    let corners = [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)];
    let (min_x, min_y, max_x, max_y) = corners.iter().fold(
        (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
        |(min_x, min_y, max_x, max_y), &(x, y)| {
            let rx = x * cos - y * sin;
            let ry = x * sin + y * cos;
            (min_x.min(rx), min_y.min(ry), max_x.max(rx), max_y.max(ry))
        },
    );
    let width = (max_x - min_x).ceil().max(1.0) as u32;
    let height = (max_y - min_y).ceil().max(1.0) as u32;
    let mut rotated = image::RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let rx = min_x + x as f32 + 0.5;
            let ry = min_y + y as f32 + 0.5;
            let source_x = rx * cos + ry * sin - 0.5;
            let source_y = -rx * sin + ry * cos - 0.5;
            rotated.put_pixel(x, y, bilinear_sample(&image, source_x, source_y));
        }
    }

    let pixel_anchor_x = frame.body_anchor_x * w / frame.display_width;
    let pixel_anchor_y = frame.body_anchor_y * h / frame.display_height;
    let rotated_anchor_x = pixel_anchor_x * cos - pixel_anchor_y * sin - min_x;
    let rotated_anchor_y = pixel_anchor_x * sin + pixel_anchor_y * cos - min_y;
    let logical_scale = frame.display_width / w;

    DecodedFrame {
        pixels: Arc::new(rotated.into_raw()),
        width,
        height,
        display_width: width as f32 * logical_scale,
        display_height: height as f32 * logical_scale,
        body_anchor_x: rotated_anchor_x * logical_scale,
        body_anchor_y: rotated_anchor_y * logical_scale,
    }
}

fn bilinear_sample(image: &image::RgbaImage, x: f32, y: f32) -> image::Rgba<u8> {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = x - x.floor();
    let ty = y - y.floor();
    let mut channels = [0.0_f32; 4];

    for (offset_y, weight_y) in [(0, 1.0 - ty), (1, ty)] {
        for (offset_x, weight_x) in [(0, 1.0 - tx), (1, tx)] {
            let sample_x = x0 + offset_x;
            let sample_y = y0 + offset_y;
            if sample_x >= 0
                && sample_y >= 0
                && sample_x < image.width() as i32
                && sample_y < image.height() as i32
            {
                let pixel = image.get_pixel(sample_x as u32, sample_y as u32);
                let weight = weight_x * weight_y;
                for channel in 0..4 {
                    channels[channel] += pixel[channel] as f32 * weight;
                }
            }
        }
    }

    image::Rgba(channels.map(|channel| channel.round().clamp(0.0, 255.0) as u8))
}

pub fn orient_frames(frames: Vec<DecodedFrame>) -> Vec<[DecodedFrame; ORIENTATION_COUNT]> {
    frames
        .into_iter()
        .map(|frame| {
            let mut orientations = Vec::with_capacity(ORIENTATION_COUNT);
            orientations.push(frame);
            for turn in 1..ORIENTATION_COUNT {
                let rotated = rotate_frame(&orientations[0], turn as f32 * 45.0);
                orientations.push(rotated);
            }
            orientations.try_into().unwrap_or_else(|_| unreachable!())
        })
        .collect()
}

fn alpha_bounds(img: &image::RgbaImage) -> (u32, u32, u32, u32) {
    let (mut mnx, mut mny, mut mxx, mut mxy) = (img.width(), img.height(), 0u32, 0u32);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 0 {
            mnx = mnx.min(x);
            mny = mny.min(y);
            mxx = mxx.max(x);
            mxy = mxy.max(y);
        }
    }
    (mnx, mny, mxx, mxy)
}

fn alpha_centroid(img: &image::RgbaImage) -> (f32, f32) {
    let (mut wx, mut wy, mut ta) = (0.0_f64, 0.0_f64, 0.0_f64);
    for (x, y, p) in img.enumerate_pixels() {
        let a = p[3] as f64;
        if a > 0.0 {
            wx += x as f64 * a;
            wy += y as f64 * a;
            ta += a;
        }
    }
    if ta == 0.0 {
        return (img.width() as f32 / 2.0, img.height() as f32 / 2.0);
    }
    ((wx / ta) as f32, (wy / ta) as f32)
}

fn notify(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .show();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_frame() -> DecodedFrame {
        DecodedFrame {
            pixels: Arc::new(vec![0; 2 * 3 * 4]),
            width: 2,
            height: 3,
            display_width: 2.0,
            display_height: 3.0,
            body_anchor_x: 0.5,
            body_anchor_y: 1.0,
        }
    }

    #[test]
    fn quarter_turns_preserve_rotated_body_anchor() {
        let original = test_frame();
        let clockwise = rotate_frame(&original, 90.0);
        assert_eq!((clockwise.width, clockwise.height), (3, 2));
        assert_eq!(
            (clockwise.body_anchor_x, clockwise.body_anchor_y),
            (2.0, 0.5)
        );

        let upside_down = rotate_frame(&original, 180.0);
        assert_eq!((upside_down.width, upside_down.height), (2, 3));
        assert_eq!(
            (upside_down.body_anchor_x, upside_down.body_anchor_y),
            (1.5, 2.0)
        );
    }

    #[test]
    fn orientation_cache_contains_all_eight_directions() {
        let oriented = orient_frames(vec![test_frame()]);
        assert_eq!(oriented.len(), 1);
        assert_eq!(oriented[0].len(), ORIENTATION_COUNT);
        assert_eq!((oriented[0][0].width, oriented[0][0].height), (2, 3));
        assert_eq!((oriented[0][2].width, oriented[0][2].height), (3, 2));
        assert_eq!((oriented[0][4].width, oriented[0][4].height), (2, 3));
        assert_eq!((oriented[0][6].width, oriented[0][6].height), (3, 2));
    }

    #[test]
    fn embedded_frames_contain_distinct_crawling_poses() {
        let frames = load_sprite_frame_data();
        assert_eq!(frames.len(), crate::constants::TOTAL_FRAMES);
        assert!(frames
            .windows(2)
            .all(|pair| pair[0].pixels != pair[1].pixels));
        assert!(frames
            .iter()
            .all(|frame| frame.width <= PIXEL_SPRITE_FRAME_WIDTH));
        assert!(frames
            .iter()
            .all(|frame| frame.display_width <= DISPLAY_SPRITE_FRAME_WIDTH));

        let oriented = orient_frames(frames);
        let cache_bytes: usize = oriented
            .iter()
            .flat_map(|directions| directions.iter())
            .map(|frame| frame.pixels.len())
            .sum();
        assert!(
            cache_bytes <= 64 * 1024 * 1024,
            "orientation cache exceeded budget: {cache_bytes} bytes"
        );
        assert!(oriented
            .iter()
            .flat_map(|directions| directions.iter())
            .all(|frame| {
                frame.body_anchor_x >= 0.0
                    && frame.body_anchor_y >= 0.0
                    && frame.body_anchor_x <= frame.display_width
                    && frame.body_anchor_y <= frame.display_height
            }));
    }
}
