//! A single crawling cockroach.

use crate::constants::{FRAME_ASPECT, TOTAL_FRAMES};
use rand::Rng;
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
pub struct AnimConfig {
    pub size_percent: f32,
    pub normal_fps: f32,
    pub fast_min_fps: f32,
    pub fast_max_fps: f32,
    pub fast_probability: f32,
    pub movement_percent: f32,
}

#[derive(Clone)]
pub struct Cockroach {
    cfg: AnimConfig,
    start_x: f32,
    start_y: f32,
    angle_deg: f32,
    vx: f32,
    vy: f32,
    movement_interval_ms: f32,
    animation_interval_ms: f32,
    spawn_delay_ms: f32,
    spawned: bool,
    visible: bool,
    travel_start_ms: f32,
    pub cur_frame: usize,
    pub center_x: f32,
    pub center_y: f32,
}

impl Cockroach {
    pub fn new(rng: &mut impl Rng, cfg: AnimConfig, width: f32, height: f32) -> Self {
        let is_fast = rng.gen::<f32>() < cfg.fast_probability;
        let fps = if is_fast {
            cfg.fast_min_fps + rng.gen::<f32>() * (cfg.fast_max_fps - cfg.fast_min_fps)
        } else {
            cfg.normal_fps
        };

        let mut c = Self {
            cfg,
            start_x: 0.0,
            start_y: 0.0,
            angle_deg: 0.0,
            vx: 0.0,
            vy: 0.0,
            movement_interval_ms: 1000.0 / fps,
            animation_interval_ms: 1000.0 / fps.min(24.0),
            spawn_delay_ms: rng.gen::<f32>() * 3000.0,
            spawned: false,
            visible: false,
            travel_start_ms: 0.0,
            cur_frame: 0,
            center_x: -1.0e6,
            center_y: -1.0e6,
        };
        c.init_random_position(rng, width, height);
        c
    }

    pub fn el_width(&self, width: f32) -> f32 {
        self.cfg.size_percent / 100.0 * width
    }

    pub fn el_height(&self, width: f32) -> f32 {
        self.el_width(width) * FRAME_ASPECT
    }

    pub fn angle_deg(&self) -> f32 {
        self.angle_deg
    }

    pub fn is_drawable(&self) -> bool {
        self.spawned && self.visible
    }

    fn init_random_position(&mut self, rng: &mut impl Rng, w: f32, h: f32) {
        let side = rng.gen_range(0..4);
        let padding = 100.0;

        let angle_offset = rng.gen_range(-1_i32..=1) as f32 * 45.0;
        let (x, y, base_angle) = match side {
            0 => {
                // Top
                (rng.gen::<f32>() * w, -padding, 90.0)
            }
            1 => {
                // Right
                (w + padding, rng.gen::<f32>() * h, 180.0)
            }
            2 => {
                // Bottom
                (rng.gen::<f32>() * w, h + padding, 270.0)
            }
            _ => {
                // Left
                (-padding, rng.gen::<f32>() * h, 0.0)
            }
        };
        let target_angle = (base_angle + angle_offset).rem_euclid(360.0);

        self.start_x = x;
        self.start_y = y;
        self.angle_deg = target_angle;

        let rad = target_angle * PI / 180.0;
        self.vx = rad.cos();
        self.vy = rad.sin();

        self.center_x = x;
        self.center_y = y;
    }

    pub fn update(&mut self, rng: &mut impl Rng, now_ms: f32, w: f32, h: f32) {
        if !self.spawned {
            if now_ms >= self.spawn_delay_ms {
                self.spawned = true;
                self.visible = true;
                self.travel_start_ms = now_ms;
                self.update_motion(rng, now_ms, w, h);
            }
            return;
        }

        self.update_motion(rng, now_ms, w, h);
    }

    fn update_motion(&mut self, rng: &mut impl Rng, now_ms: f32, w: f32, h: f32) {
        let elapsed = (now_ms - self.travel_start_ms).max(0.0);
        let animation_progress = elapsed / self.animation_interval_ms;
        self.cur_frame = (animation_progress.floor() as usize) % TOTAL_FRAMES;

        let el_w = self.el_width(w);
        let movement = self.cfg.movement_percent / 100.0;
        let movement_progress = elapsed / self.movement_interval_ms;
        let offset = (movement_progress / TOTAL_FRAMES as f32) * (el_w * movement);

        let cur_x = offset * self.vx;
        let cur_y = offset * self.vy;
        self.center_x = self.start_x + cur_x;
        self.center_y = self.start_y + cur_y;

        let margin = el_w;
        if self.center_x < -margin
            || self.center_x > w + margin
            || self.center_y < -margin
            || self.center_y > h + margin
        {
            self.reset(rng, now_ms, w, h);
        }
    }

    fn reset(&mut self, rng: &mut impl Rng, now_ms: f32, w: f32, h: f32) {
        self.travel_start_ms = now_ms;
        self.cur_frame = 0;
        self.init_random_position(rng, w, h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};
    use std::collections::HashSet;

    #[test]
    fn spawned_cockroach_moves_between_animation_ticks() {
        let mut rng = StdRng::seed_from_u64(7);
        let cfg = AnimConfig {
            size_percent: 35.0,
            normal_fps: 10.0,
            fast_min_fps: 10.0,
            fast_max_fps: 60.0,
            fast_probability: 0.0,
            movement_percent: 13.5,
        };
        let mut cockroach = Cockroach::new(&mut rng, cfg, 1920.0, 1080.0);

        cockroach.update(&mut rng, 3_001.0, 1920.0, 1080.0);
        let initial = (cockroach.center_x, cockroach.center_y);
        cockroach.update(&mut rng, 3_101.0, 1920.0, 1080.0);

        assert!(cockroach.is_drawable());
        assert_ne!((cockroach.center_x, cockroach.center_y), initial);
    }

    #[test]
    fn movement_direction_matches_rendered_eighth_turn() {
        let cfg = AnimConfig {
            size_percent: 35.0,
            normal_fps: 10.0,
            fast_min_fps: 10.0,
            fast_max_fps: 60.0,
            fast_probability: 0.5,
            movement_percent: 13.5,
        };

        let mut angles = HashSet::new();
        for seed in 0..64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let cockroach = Cockroach::new(&mut rng, cfg, 1920.0, 1080.0);
            assert_eq!(cockroach.angle_deg.rem_euclid(45.0), 0.0);
            let radians = cockroach.angle_deg.to_radians();
            assert!((cockroach.vx - radians.cos()).abs() < 0.0001);
            assert!((cockroach.vy - radians.sin()).abs() < 0.0001);
            angles.insert(cockroach.angle_deg as i32);
        }
        assert!(
            angles.len() >= 6,
            "expected varied directions, got {angles:?}"
        );
    }

    #[test]
    fn fast_movement_keeps_leg_animation_human_visible() {
        let mut rng = StdRng::seed_from_u64(9);
        let cfg = AnimConfig {
            size_percent: 35.0,
            normal_fps: 10.0,
            fast_min_fps: 60.0,
            fast_max_fps: 60.0,
            fast_probability: 1.0,
            movement_percent: 13.5,
        };
        let cockroach = Cockroach::new(&mut rng, cfg, 1920.0, 1080.0);

        assert_eq!(cockroach.movement_interval_ms, 1000.0 / 60.0);
        assert_eq!(cockroach.animation_interval_ms, 1000.0 / 24.0);
    }
}
