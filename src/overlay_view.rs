//! Transparent overlay window — one per display, shows crawling cockroaches.

use crate::app::{frame_to_image_source, DecodedFrame};
use crate::cockroach::Cockroach;
use crate::constants::ORIENTATION_COUNT;
use gpui::*;
use rand::Rng;
use std::rc::Rc;

/// A rendered sprite frame with its GPUI ImageSource.
#[derive(Clone)]
pub struct OrientedFrame {
    pub source: ImageSource,
    pub width: f32,
    pub height: f32,
    pub body_anchor_x: f32,
    pub body_anchor_y: f32,
}

#[derive(Clone)]
pub struct RenderedFrame {
    pub orientations: [OrientedFrame; ORIENTATION_COUNT],
}

impl RenderedFrame {
    pub fn new(frames: [DecodedFrame; ORIENTATION_COUNT]) -> Self {
        Self {
            orientations: frames.map(|rotated| {
                let width = rotated.display_width;
                let height = rotated.display_height;
                let body_anchor_x = rotated.body_anchor_x;
                let body_anchor_y = rotated.body_anchor_y;
                OrientedFrame {
                    source: frame_to_image_source(rotated),
                    width,
                    height,
                    body_anchor_x,
                    body_anchor_y,
                }
            }),
        }
    }
}

/// An overlay window's root view.
pub struct OverlayView {
    pub roaches: Vec<Cockroach>,
    frames: Rc<Vec<RenderedFrame>>,
    pub width: f32,
    pub height: f32,
}

impl OverlayView {
    pub fn new(
        roaches: Vec<Cockroach>,
        frames: Rc<Vec<RenderedFrame>>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            roaches,
            frames,
            width,
            height,
        }
    }

    #[allow(dead_code)]
    pub fn update_roaches(&mut self, rng: &mut impl Rng, now_ms: f32) {
        for roach in &mut self.roaches {
            roach.update(rng, now_ms, self.width, self.height);
        }
    }

    pub fn release_images(&self, window: &mut Window) {
        for frame in self.frames.iter() {
            for orientation in &frame.orientations {
                if let ImageSource::Render(image) = &orientation.source {
                    let _ = window.drop_image(image.clone());
                }
            }
        }
    }
}

impl Render for OverlayView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut elements: Vec<Div> = Vec::new();

        for (index, roach) in self.roaches.iter().enumerate() {
            if !roach.is_drawable() {
                continue;
            }

            let fi = roach.cur_frame.min(self.frames.len() - 1);
            let turns = ((roach.angle_deg().rem_euclid(360.0) + 22.5) / 45.0).floor() as usize
                % ORIENTATION_COUNT;
            let rf = &self.frames[fi].orientations[turns];

            let el_w = roach.el_width(self.width);
            let el_h = roach.el_height(self.width);
            debug_assert!((el_w / 1920.0 - el_h / 1080.0).abs() < 0.0001);
            let scale = el_w / 1920.0;

            let dw = rf.width * scale;
            let dh = rf.height * scale;
            let ox = -rf.body_anchor_x * scale;
            let oy = -rf.body_anchor_y * scale;
            elements.push(
                div()
                    .absolute()
                    .left(px(roach.center_x + ox))
                    .top(px(roach.center_y + oy))
                    .w(px(dw))
                    .h(px(dh))
                    .child(
                        img(rf.source.clone())
                            .id(("cockroach", index))
                            .w(px(dw))
                            .h(px(dh)),
                    ),
            );
        }

        div().size_full().overflow_hidden().children(elements)
    }
}
