//! 🪳 蟑螂提醒 (Cockroach Reminder) — GPUI port.

#![recursion_limit = "256"]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod cockroach;
mod config;
mod constants;
mod overlay_view;
mod platform;
mod settings_view;
mod timer;
mod tray;

use app::AppState;
use cockroach::Cockroach;
use overlay_view::{OverlayView, RenderedFrame};
use settings_view::SettingsView;
use timer::{Phase, Transition};
use tray::{Tray, TrayCommand};

use gpui::*;
use gpui_component::Root;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn dispose_overlay_windows(handles: &mut Vec<WindowHandle<OverlayView>>, cx: &mut AsyncApp) {
    for handle in handles.drain(..) {
        let _ = handle.update(cx, |view, window, _| {
            view.release_images(window);
            window.remove_window();
        });
    }
}

fn main() {
    platform::hide_dock();

    let application = gpui_platform::application().with_assets(gpui_component_assets::Assets);
    application.run(move |app_cx| {
        gpui_component::init(app_cx);
        gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, app_cx);
        let theme = gpui_component::Theme::global_mut(app_cx);
        let primary: Hsla = rgb(0xc4a36a).into();
        theme.primary = primary;
        theme.primary_hover = rgb(0xd7b978).into();
        theme.primary_foreground = rgb(0x0c0c0e).into();
        theme.tokens.primary = primary.into();

        // Background load sprite frames.
        let decoded: Arc<
            Mutex<Option<Vec<[app::DecodedFrame; constants::ORIENTATION_COUNT]>>>,
        > = Arc::new(Mutex::new(None));
        let d = decoded.clone();
        app_cx
            .background_spawn(async move {
                let decoded = app::orient_frames(app::load_sprite_frame_data());
                *d.lock().unwrap() = Some(decoded);
            })
            .detach();

        // Create shared state (Entity<AppState>).
        let state = app_cx.new(|_cx| AppState::new());

        // Create tray icon.
        let tray: Rc<RefCell<Option<Tray>>> = {
            let labels = state.read(app_cx).refresh_tray_labels();
            Rc::new(RefCell::new(Tray::new(
                &labels.0, &labels.1, labels.2, &labels.3,
            )))
        };

        // Main async loop: frame loading + timer + overlay + tray commands.
        let s = state.clone();
        let t = tray.clone();
        let dec = decoded.clone();
        app_cx
            .spawn(async move |cx: &mut AsyncApp| {
                let mut frames_loaded = false;
                let mut tick: u64 = 0;
                // Overlay windows are reused across breaks to keep Metal allocations stable.
                let mut overlay_handles: Vec<WindowHandle<OverlayView>> = Vec::new();
                let mut overlays_active = false;
                let mut break_initialized = false;
                let mut overlay_signature: Vec<(u64, u64, u32, u32)> = Vec::new();
                let mut overlay_retry_at: Option<std::time::Instant> = None;
                let mut settings_handle: Option<WindowHandle<Root>> = None;

                loop {
                    smol::Timer::after(Duration::from_millis(16)).await;
                    tick += 1;

                    // === Frame loading check (every ~200ms) ===
                    if !frames_loaded && tick.is_multiple_of(13) {
                        if let Ok(mut guard) = dec.try_lock() {
                            if let Some(decoded) = guard.take() {
                                drop(guard);
                                let frames =
                                    Rc::new(decoded.into_iter().map(RenderedFrame::new).collect());
                                cx.update_entity(&s, |st, cx| {
                                    if st.frames.is_none() {
                                        st.frames = Some(frames);
                                        cx.notify();
                                    }
                                });
                                frames_loaded = true;
                            }
                        }
                    }

                    // === Timer tick (every ~1s) ===
                    if tick.is_multiple_of(63) {
                        let labels = cx.update_entity(&s, |st, cx| {
                            match st.timer.tick() {
                                Some(Transition::EnteredBreak) => {}
                                Some(Transition::EnteredRunning) => st.clear_overlays(),
                                None => {}
                            }
                            cx.notify();
                            st.refresh_tray_labels()
                        });
                        if let Some(ref tr) = *t.borrow() {
                            tr.refresh(&labels.0, &labels.1, labels.2, &labels.3);
                        }
                    }

                    // === Overlay lifecycle ===
                    let frames_ok = cx.read_entity(&s, |st, _| st.frames.is_some());
                    let phase = cx.read_entity(&s, |st, _| st.timer.phase);

                    if phase == Phase::Break && !overlays_active && frames_ok {
                        if !break_initialized {
                            cx.update_entity(&s, |st, _| st.prepare_overlays());
                            break_initialized = true;
                        }

                        let infos: Vec<(f64, f64, f32, f32, Vec<Cockroach>)> =
                            cx.read_entity(&s, |st, _| {
                                st.overlays
                                    .iter()
                                    .map(|o| (o.x, o.y, o.width, o.height, o.cockroaches.clone()))
                                    .collect()
                            });
                        let frame_sources = cx.read_entity(&s, |st, _| {
                            st.frames.as_ref().expect("frames were checked").clone()
                        });
                        let signature: Vec<_> = infos
                            .iter()
                            .map(|(x, y, width, height, _)| {
                                (x.to_bits(), y.to_bits(), width.to_bits(), height.to_bits())
                            })
                            .collect();
                        let retry_ready = overlay_retry_at
                            .is_none_or(|retry_at| std::time::Instant::now() >= retry_at);

                        if !frame_sources.is_empty() && retry_ready {
                            if !overlay_handles.is_empty() && overlay_signature != signature {
                                dispose_overlay_windows(&mut overlay_handles, cx);
                                overlay_signature.clear();
                            }

                            // Use cx.update() to get &mut App for open_window.
                            if overlay_handles.is_empty() {
                                cx.update(|app_cx: &mut App| {
                                    for (x, y, w, h, roaches) in &infos {
                                        let bounds = Bounds {
                                            origin: point(px(*x as f32), px(*y as f32)),
                                            size: size(px(*w), px(*h)),
                                        };
                                        let fs = frame_sources.clone();
                                        let r = roaches.clone();
                                        if let Ok(handle) = app_cx.open_window(
                                            WindowOptions {
                                                window_bounds: Some(WindowBounds::Windowed(bounds)),
                                                window_background:
                                                    WindowBackgroundAppearance::Transparent,
                                                focus: false,
                                                show: true,
                                                kind: WindowKind::PopUp,
                                                is_movable: false,
                                                is_resizable: false,
                                                is_minimizable: false,
                                                display_id: None,
                                                titlebar: None,
                                                app_owns_titlebar_drag: false,
                                                app_id: None,
                                                window_min_size: None,
                                                icon: None,
                                                tabbing_identifier: None,
                                                window_decorations: None,
                                            },
                                            |window, cx| {
                                                window.set_input_region(Some(&[]));
                                                if let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(window) {
                                                    platform::configure_overlay(&handle.as_raw());
                                                }
                                                cx.new(|_| OverlayView::new(r, fs, *w, *h))
                                            },
                                        ) {
                                            overlay_handles.push(handle);
                                        }
                                    }
                                });
                            }

                            if overlay_handles.len() == infos.len() {
                                let mut update_failed = false;
                                for (handle, (_, _, _, _, roaches)) in
                                    overlay_handles.iter().zip(&infos)
                                {
                                    if handle
                                        .update(cx, |view, _, cx| {
                                            view.roaches.clone_from(roaches);
                                            cx.notify();
                                        })
                                        .is_err()
                                    {
                                        update_failed = true;
                                    }
                                }
                                if update_failed {
                                    dispose_overlay_windows(&mut overlay_handles, cx);
                                    overlay_signature.clear();
                                    overlay_retry_at =
                                        Some(std::time::Instant::now() + Duration::from_secs(1));
                                } else {
                                    overlays_active = true;
                                    overlay_signature = signature;
                                    overlay_retry_at = None;
                                }
                            } else {
                                dispose_overlay_windows(&mut overlay_handles, cx);
                                overlay_signature.clear();
                                overlay_retry_at =
                                    Some(std::time::Instant::now() + Duration::from_secs(1));
                            }
                        }
                    } else if phase != Phase::Break && break_initialized {
                        if overlays_active {
                            for handle in &overlay_handles {
                                let _ = handle.update(cx, |view, _, cx| {
                                    view.roaches.clear();
                                    cx.notify();
                                });
                            }
                        }
                        cx.update_entity(&s, |st, cx| {
                            st.clear_overlays();
                            cx.notify();
                        });
                        overlays_active = false;
                        break_initialized = false;
                        overlay_retry_at = None;
                    } else if phase == Phase::Break && overlays_active {
                        // Update animation and push data to overlay windows.
                        let now = std::time::Instant::now();
                        cx.update_entity(&s, |st, _| st.update_animation(now));

                        let all_roaches: Vec<Vec<Cockroach>> = cx.read_entity(&s, |st, _| {
                            st.overlays.iter().map(|o| o.cockroaches.clone()).collect()
                        });
                        let mut update_failed = false;
                        for (i, h) in overlay_handles.iter().enumerate() {
                            if let Some(ro) = all_roaches.get(i) {
                                if h.update(cx, |view, _, cx| {
                                    view.roaches.clone_from(ro);
                                    cx.notify();
                                })
                                .is_err()
                                {
                                    update_failed = true;
                                }
                            }
                        }
                        if update_failed {
                            dispose_overlay_windows(&mut overlay_handles, cx);
                            overlays_active = false;
                            overlay_signature.clear();
                            overlay_retry_at =
                                Some(std::time::Instant::now() + Duration::from_secs(1));
                        }
                    }

                    // === Tray command polling (every ~200ms) ===
                    if tick.is_multiple_of(13) {
                        if let Some(cmd) = tray::poll_command() {
                            match cmd {
                                TrayCommand::ToggleTimer => {
                                    let labels = cx.update_entity(&s, |st, cx| {
                                        match st.timer.phase {
                                            Phase::Running => st.timer.pause(),
                                            Phase::Paused => st.timer.resume(),
                                            Phase::Idle => st.timer.start(),
                                            Phase::Break => {}
                                        }
                                        cx.notify();
                                        st.refresh_tray_labels()
                                    });
                                    if let Some(ref tr) = *t.borrow() {
                                        tr.refresh(&labels.0, &labels.1, labels.2, &labels.3);
                                    };
                                }
                                TrayCommand::TriggerBreak => {
                                    let labels = cx.update_entity(&s, |st, cx| {
                                        st.timer.trigger_break();
                                        cx.notify();
                                        st.refresh_tray_labels()
                                    });
                                    if let Some(ref tr) = *t.borrow() {
                                        tr.refresh(&labels.0, &labels.1, labels.2, &labels.3);
                                    };
                                }
                                TrayCommand::OpenSettings => {
                                    cx.update(|app| app.activate(true));
                                    let activated = settings_handle
                                        .and_then(|handle| {
                                            handle
                                                .update(cx, |_, window, _| window.activate_window())
                                                .ok()
                                                .map(|_| true)
                                        })
                                        .unwrap_or(false);

                                    if !activated {
                                        let s2 = s.clone();
                                        let opened = cx.update(|app| {
                                            let bounds = WindowBounds::centered(
                                                size(px(480.), px(690.)),
                                                app,
                                            );
                                            app.open_window(
                                                WindowOptions {
                                                    window_bounds: Some(bounds),
                                                    window_background:
                                                        WindowBackgroundAppearance::Opaque,
                                                    focus: true,
                                                    show: true,
                                                    kind: WindowKind::Normal,
                                                    is_movable: true,
                                                    is_resizable: true,
                                                    is_minimizable: true,
                                                    display_id: None,
                                                    titlebar: Some(TitlebarOptions {
                                                        title: Some("蟑螂提醒设置".into()),
                                                        ..Default::default()
                                                    }),
                                                    app_owns_titlebar_drag: false,
                                                    app_id: None,
                                                    window_min_size: Some(size(px(400.), px(560.))),
                                                    icon: None,
                                                    tabbing_identifier: None,
                                                    window_decorations: None,
                                                },
                                                move |window, cx| {
                                                    let view = cx.new(|cx| {
                                                        SettingsView::new(s2.clone(), cx)
                                                    });
                                                    cx.new(|cx| Root::new(view, window, cx))
                                                },
                                            )
                                        });
                                        if let Ok(handle) = opened {
                                            let _ = handle.update(cx, |_, window, _| {
                                                window.activate_window();
                                            });
                                            settings_handle = Some(handle);
                                        }
                                    }

                                    let labels = cx.update_entity(&s, |st, cx| {
                                        st.settings_view_opened = true;
                                        cx.notify();
                                        st.refresh_tray_labels()
                                    });
                                    let guard = t.borrow();
                                    if let Some(ref tr) = *guard {
                                        tr.refresh(&labels.0, &labels.1, labels.2, &labels.3);
                                    };
                                }
                                TrayCommand::Quit => {
                                    cx.update(|app_cx: &mut App| app_cx.quit());
                                    break;
                                }
                            }
                        }
                    }
                }
            })
            .detach();
    });
}
