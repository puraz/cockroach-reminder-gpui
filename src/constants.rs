//! Shared constants for the Cockroach Reminder app.

/// Number of animation frames per cockroach.
pub const TOTAL_FRAMES: usize = 13;

/// Pre-rendered directions, spaced at 45-degree increments.
pub const ORIENTATION_COUNT: usize = 8;

/// Aspect ratio of a frame image (1920x1080 -> height = width * 0.5625).
pub const FRAME_ASPECT: f32 = 1080.0 / 1920.0;

/// Embedded frame image bytes, indexed `0..TOTAL_FRAMES`.
pub const FRAME_BYTES: [&[u8]; TOTAL_FRAMES] = [
    include_bytes!("../assets/frames/001_1.1.0.png"),
    include_bytes!("../assets/frames/001_1.1.1.png"),
    include_bytes!("../assets/frames/001_1.1.2.png"),
    include_bytes!("../assets/frames/001_1.1.3.png"),
    include_bytes!("../assets/frames/001_1.1.4.png"),
    include_bytes!("../assets/frames/001_1.1.5.png"),
    include_bytes!("../assets/frames/001_1.1.6.png"),
    include_bytes!("../assets/frames/001_1.1.7.png"),
    include_bytes!("../assets/frames/001_1.1.8.png"),
    include_bytes!("../assets/frames/001_1.1.9.png"),
    include_bytes!("../assets/frames/001_1.1.10.png"),
    include_bytes!("../assets/frames/001_1.1.11.png"),
    include_bytes!("../assets/frames/001_1.1.12.png"),
];

/// Tray icon (template) PNG bytes.
pub const TRAY_ICON_BYTES: &[u8] = include_bytes!("../assets/trayIconTemplate@2x.png");
