//! Synthetic game feed for Milestone 1: produces the same observations the
//! capture loop will, so the UI exercises the real stats engine.

use battle_core::stats::{ExpReading, Line, LineKind, Observation, Session};

const STACK_CAPACITY: usize = 6;
const FADE_PER_TICK: u8 = 6;

pub struct MockFeed {
    rng: u64,
    level: u32,
    exp: u64,
    next_kill_ms: u64,
    screen: Vec<Line>,
}

impl MockFeed {
    pub fn new() -> Self {
        // Near the end of level 59 so a level-up shows up within a few minutes.
        Self { rng: 0x5eed, level: 59, exp: 1_120_000, next_kill_ms: 0, screen: Vec::new() }
    }

    pub fn tick(&mut self, t_ms: u64, session: &mut Session) {
        for line in &mut self.screen {
            line.freshness = line.freshness.saturating_sub(FADE_PER_TICK);
        }
        self.screen.retain(|l| l.freshness > 60);

        if t_ms >= self.next_kill_ms {
            self.kill();
            self.next_kill_ms = t_ms + 1_500 + self.rand(2_500);
        }

        let max = mock_max(self.level);
        let pct = (self.exp * 10_000 / max) as u32;
        session.observe(t_ms, Observation::Exp(ExpReading::new(self.level, self.exp, pct)));
        session.observe(t_ms, Observation::Stack(self.screen.clone()));
    }

    fn kill(&mut self) {
        let gained = 70 + self.rand(20);
        self.exp += gained;
        let max = mock_max(self.level);
        if self.exp >= max {
            self.exp -= max;
            self.level += 1;
        }
        self.push(Line::new(LineKind::Exp, gained, 255));
        if self.rand(10) < 6 {
            let mesos = 10 + self.rand(15);
            self.push(Line::new(LineKind::Meso, mesos, 255));
        }
    }

    fn push(&mut self, line: Line) {
        self.screen.push(line);
        if self.screen.len() > STACK_CAPACITY {
            self.screen.remove(0);
        }
    }

    fn rand(&mut self, below: u64) -> u64 {
        self.rng = self.rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.rng >> 33) % below
    }
}

fn mock_max(level: u32) -> u64 {
    1_147_000 + u64::from(level.saturating_sub(59)) * 75_000
}
