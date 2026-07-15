//! Break timer state machine.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Running,
    Break,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    EnteredBreak,
    EnteredRunning,
}

pub struct Timer {
    pub phase: Phase,
    pub remaining_ms: i64,
    pub interval_minutes: u32,
    pub duration_seconds: u32,
}

impl Timer {
    pub fn new(interval_minutes: u32, duration_seconds: u32) -> Self {
        Self {
            phase: Phase::Idle,
            remaining_ms: 0,
            interval_minutes,
            duration_seconds,
        }
    }

    pub fn start(&mut self) {
        self.remaining_ms = self.interval_minutes as i64 * 60 * 1000;
        self.phase = Phase::Running;
    }

    pub fn pause(&mut self) {
        if self.phase == Phase::Running {
            self.phase = Phase::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = Phase::Running;
        }
    }

    pub fn trigger_break(&mut self) {
        self.phase = Phase::Break;
        self.remaining_ms = self.duration_seconds as i64 * 1000;
    }

    pub fn tick(&mut self) -> Option<Transition> {
        if self.phase != Phase::Running && self.phase != Phase::Break {
            return None;
        }
        self.remaining_ms -= 1000;
        if self.remaining_ms <= 0 {
            match self.phase {
                Phase::Running => {
                    self.trigger_break();
                    Some(Transition::EnteredBreak)
                }
                Phase::Break => {
                    self.start();
                    Some(Transition::EnteredRunning)
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn remaining(&self) -> (i64, i64) {
        let total = self.remaining_ms.max(0);
        (total / 60000, (total % 60000) / 1000)
    }

    pub fn formatted(&self) -> String {
        let (m, s) = self.remaining();
        format!("{:02}:{:02}", m, s)
    }

    pub fn update_interval(&mut self, minutes: u32) {
        self.interval_minutes = minutes;
        if self.phase == Phase::Running {
            self.start();
        }
    }

    pub fn update_duration(&mut self, seconds: u32) {
        self.duration_seconds = seconds;
    }
}
