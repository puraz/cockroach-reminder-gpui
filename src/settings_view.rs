//! Settings window UI, kept in parity with the iced version.

use crate::app::AppState;
use crate::timer::Phase;
use gpui::*;
use gpui_component::{
    checkbox::Checkbox,
    h_flex,
    scroll::ScrollableElement,
    slider::{Slider, SliderEvent, SliderState},
    v_flex,
};

const CANVAS: u32 = 0x0c0c0e;
const SURFACE: u32 = 0x151518;
const SURFACE_RAISED: u32 = 0x1d1d22;
const INK: u32 = 0xf4f0e8;
const RULE: u32 = 0x35353a;
const ACCENT: u32 = 0xc4a36a;
const ACCENT_DIM: u32 = 0x7c6542;
const ACCENT_WASH: u32 = 0x2b2620;

pub struct SettingsView {
    app: Entity<AppState>,
    interval: Entity<SliderState>,
    duration: Entity<SliderState>,
    count: Entity<SliderState>,
    size: Entity<SliderState>,
    speed: Entity<SliderState>,
    fast_probability: Entity<SliderState>,
    _subscriptions: Vec<Subscription>,
}

impl SettingsView {
    pub fn new(app: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let settings = app.read(cx).settings.clone();
        let interval = slider(cx, 1.0, 120.0, 1.0, settings.interval_minutes as f32);
        let duration = slider(cx, 3.0, 120.0, 1.0, settings.duration_seconds as f32);
        let count = slider(cx, 1.0, 50.0, 1.0, settings.cockroach_count as f32);
        let size = slider(cx, 10.0, 80.0, 1.0, settings.cockroach_size_percent);
        let speed = slider(cx, 5.0, 50.0, 0.5, settings.movement_percent);
        let fast_probability = slider(cx, 0.0, 100.0, 5.0, settings.fast_speed_probability * 100.0);

        let subscriptions = vec![
            cx.observe(&app, |_, _, cx| cx.notify()),
            cx.subscribe(&interval, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = event {
                    let minutes = value.start().round() as u32;
                    this.app.update(cx, |state, cx| {
                        state.settings.interval_minutes = minutes;
                        state.settings.clamp();
                        state.timer.update_interval(state.settings.interval_minutes);
                        state.settings.save();
                        cx.notify();
                    });
                    cx.notify();
                }
            }),
            cx.subscribe(&duration, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = event {
                    let seconds = value.start().round() as u32;
                    this.app.update(cx, |state, cx| {
                        state.settings.duration_seconds = seconds;
                        state.settings.clamp();
                        state.timer.update_duration(state.settings.duration_seconds);
                        state.settings.save();
                        cx.notify();
                    });
                    cx.notify();
                }
            }),
            cx.subscribe(&count, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = event {
                    let value = value.start().round() as u32;
                    this.update_settings(cx, move |state| {
                        state.settings.cockroach_count = value;
                    });
                }
            }),
            cx.subscribe(&size, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = event {
                    let value = value.start();
                    this.update_settings(cx, move |state| {
                        state.settings.cockroach_size_percent = value;
                    });
                }
            }),
            cx.subscribe(&speed, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = event {
                    let value = value.start();
                    this.update_settings(cx, move |state| {
                        state.settings.movement_percent = value;
                    });
                }
            }),
            cx.subscribe(&fast_probability, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = event {
                    let value = value.start() / 100.0;
                    this.update_settings(cx, move |state| {
                        state.settings.fast_speed_probability = value;
                    });
                }
            }),
        ];

        Self {
            app,
            interval,
            duration,
            count,
            size,
            speed,
            fast_probability,
            _subscriptions: subscriptions,
        }
    }

    fn update_settings(
        &self,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut AppState) + 'static,
    ) {
        self.app.update(cx, |state, cx| {
            update(state);
            state.settings.clamp();
            state.settings.save();
            cx.notify();
        });
        cx.notify();
    }

    fn slider_row(&self, label: &'static str, value: String, state: &Entity<SliderState>) -> Div {
        v_flex()
            .w_full()
            .gap(px(9.))
            .child(
                h_flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(14.))
                            .text_color(rgb(INK))
                            .child(label),
                    )
                    .child(
                        div()
                            .px(px(9.))
                            .py(px(5.))
                            .text_size(px(13.))
                            .text_color(rgb(ACCENT))
                            .child(value),
                    ),
            )
            .child(
                Slider::new(state)
                    .w_full()
                    .bg(rgb(ACCENT))
                    .text_color(rgb(CANVAS)),
            )
    }

    fn section(&self, content: impl IntoElement) -> Div {
        div()
            .w_full()
            .p(px(18.))
            .rounded(px(8.))
            .border_1()
            .border_color(rgb(RULE))
            .bg(rgb(SURFACE))
            .child(content)
    }

    fn status_card(&self, phase: Phase, formatted: String) -> Div {
        let (badge, line) = match phase {
            Phase::Running => ("计时中", format!("下次休息还有 {formatted}")),
            Phase::Break => ("休息中", format!("休息时间，还剩 {formatted}")),
            Phase::Paused => ("已暂停", format!("计时已暂停，剩余 {formatted}")),
            Phase::Idle => ("未开始", "计时器尚未开始".to_string()),
        };

        h_flex()
            .w_full()
            .items_center()
            .px(px(18.))
            .py(px(15.))
            .rounded(px(8.))
            .border_1()
            .border_color(rgb(ACCENT_DIM))
            .bg(rgb(SURFACE_RAISED))
            .child(
                div()
                    .flex_1()
                    .text_size(px(17.))
                    .text_color(rgb(INK))
                    .child(line),
            )
            .child(
                div()
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(7.))
                    .bg(rgb(ACCENT))
                    .text_size(px(13.))
                    .text_color(rgb(CANVAS))
                    .child(badge),
            )
    }

    fn action_button(&self, label: &'static str) -> Stateful<Div> {
        h_flex()
            .id(label)
            .flex_1()
            .h(px(42.))
            .justify_center()
            .items_center()
            .rounded(px(8.))
            .border_1()
            .border_color(rgb(RULE))
            .bg(rgb(SURFACE))
            .text_color(rgb(INK))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(SURFACE_RAISED)))
            .active(|style| style.bg(rgb(ACCENT_WASH)))
            .child(label)
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (phase, formatted, interval_minutes, duration_seconds, cockroach_count,
              cockroach_size_percent, movement_percent, fast_speed_probability,
              auto_start, launch_at_login, show_notifications) = {
            let state = self.app.read(cx);
            let s = &state.settings;
            (
                state.timer.phase,
                state.timer.formatted(),
                s.interval_minutes,
                s.duration_seconds,
                s.cockroach_count,
                s.cockroach_size_percent,
                s.movement_percent,
                s.fast_speed_probability,
                s.auto_start,
                s.launch_at_login,
                s.show_notifications,
            )
        };

        let timer_section = self.section(
            v_flex()
                .gap(px(14.))
                .child(self.slider_row(
                    "休息间隔",
                    format!("{} 分钟", interval_minutes),
                    &self.interval,
                ))
                .child(self.slider_row(
                    "显示时长",
                    format!("{} 秒", duration_seconds),
                    &self.duration,
                ))
                .child(self.slider_row(
                    "蟑螂数量",
                    format!("{} 只", cockroach_count),
                    &self.count,
                )),
        );

        let animation_section = self.section(
            v_flex()
                .gap(px(14.))
                .child(self.slider_row(
                    "蟑螂大小",
                    format!("{}%", cockroach_size_percent.round() as u32),
                    &self.size,
                ))
                .child(self.slider_row(
                    "移动速度",
                    format!("{:.1}%", movement_percent),
                    &self.speed,
                ))
                .child(self.slider_row(
                    "快速蟑螂概率",
                    format!(
                        "{}%",
                        (fast_speed_probability * 100.0).round() as u32
                    ),
                    &self.fast_probability,
                )),
        );

        let behavior_section = self.section(
            v_flex()
                .gap(px(12.))
                .child(
                    Checkbox::new("auto-start")
                        .label("启动应用时自动开启计时")
                        .checked(auto_start)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            let checked = *checked;
                            this.update_settings(cx, move |state| {
                                state.settings.auto_start = checked
                            });
                        })),
                )
                .child(
                    Checkbox::new("launch-at-login")
                        .label("开机自启动")
                        .checked(launch_at_login)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            let checked = *checked;
                            this.update_settings(cx, move |state| {
                                state.settings.launch_at_login = checked
                            });
                        })),
                )
                .child(
                    Checkbox::new("show-notifications")
                        .label("显示系统通知")
                        .checked(show_notifications)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            let checked = *checked;
                            this.update_settings(cx, move |state| {
                                state.settings.show_notifications = checked
                            });
                        })),
                ),
        );

        let actions = h_flex()
            .w_full()
            .gap(px(12.))
            .child(self.action_button("立即休息").on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.app.update(cx, |state, cx| {
                        state.timer.trigger_break();
                        cx.notify();
                    });
                }),
            ))
            .child(
                self.action_button(if phase == Phase::Running {
                    "暂停计时"
                } else {
                    "继续计时"
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.app.update(cx, |state, cx| {
                            match state.timer.phase {
                                Phase::Running => state.timer.pause(),
                                Phase::Paused => state.timer.resume(),
                                Phase::Idle => state.timer.start(),
                                Phase::Break => {}
                            }
                            cx.notify();
                        });
                    }),
                ),
            );

        div()
            .size_full()
            .bg(rgb(CANVAS))
            .text_color(rgb(INK))
            .child(
                div().size_full().overflow_y_scrollbar().child(
                    h_flex().w_full().justify_center().child(
                        v_flex()
                            .w_full()
                            .max_w(px(440.))
                            .px(px(10.))
                            .py(px(24.))
                            .gap(px(14.))
                            .child(
                                div()
                                    .text_size(px(29.))
                                    .text_color(rgb(INK))
                                    .child("定时休息，保护健康！"),
                            )
                            .child(self.status_card(phase, formatted))
                            .child(timer_section)
                            .child(animation_section)
                            .child(behavior_section)
                            .child(actions),
                    ),
                ),
            )
    }
}

fn slider(
    cx: &mut Context<SettingsView>,
    min: f32,
    max: f32,
    step: f32,
    value: f32,
) -> Entity<SliderState> {
    cx.new(|_| {
        SliderState::new()
            .min(min)
            .max(max)
            .step(step)
            .default_value(value)
    })
}
