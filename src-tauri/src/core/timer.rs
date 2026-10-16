use super::{now, Core};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Timer {
    pub phase: String,
    pub status: String,
    pub duration_seconds: u32,
    pub remaining_seconds: u32,
    pub deadline: Option<i64>,
    pub completed_focus: u32,
}
impl Default for Timer {
    fn default() -> Self {
        Self {
            phase: "focus".into(),
            status: "idle".into(),
            duration_seconds: 1500,
            remaining_seconds: 1500,
            deadline: None,
            completed_focus: 0,
        }
    }
}
impl Timer {
    pub fn at(mut self, time: i64) -> Self {
        if self.status == "running" {
            if let Some(deadline) = self.deadline {
                self.remaining_seconds = ((deadline - time).max(0) as u64).div_ceil(1000) as u32;
                if self.remaining_seconds == 0 {
                    self.status = "completed".into();
                    self.deadline = None;
                    if self.phase == "focus" {
                        self.completed_focus += 1;
                    }
                }
            }
        }
        self
    }
}
impl Core {
    pub fn timer(&self) -> Result<Timer, String> {
        Ok(self.kv_get::<Timer>("timer")?.unwrap_or_default().at(now()))
    }
    pub fn timer_tick(&self) -> Result<Option<Timer>, String> {
        let _guard = self.0.timer_lock.lock().map_err(|_| "Timer unavailable")?;
        let old = self.kv_get::<Timer>("timer")?.unwrap_or_default();
        let next = old.clone().at(now());
        if old.status == "running" && next.status == "completed" {
            self.kv_set("timer", &next)?;
            return Ok(Some(next));
        }
        Ok(None)
    }
    pub fn timer_action(&self, action: &str) -> Result<Timer, String> {
        let _guard = self.0.timer_lock.lock().map_err(|_| "Timer unavailable")?;
        let mut t = self.timer()?;
        let config = self.settings()?;
        match action {
            "start" => {
                if t.status == "running" {
                    return Ok(t);
                }
                if t.status == "completed" {
                    t.phase = if t.phase == "focus" {
                        if t.completed_focus.is_multiple_of(4) {
                            "long-break"
                        } else {
                            "short-break"
                        }
                    } else {
                        "focus"
                    }
                    .into();
                    t.status = "idle".into();
                }
                if t.status == "idle" {
                    t.duration_seconds = 60
                        * match t.phase.as_str() {
                            "short-break" => config.short_break_minutes,
                            "long-break" => config.long_break_minutes,
                            _ => config.focus_minutes,
                        };
                    t.remaining_seconds = t.duration_seconds;
                }
                t.deadline = Some(now() + i64::from(t.remaining_seconds) * 1000);
                t.status = "running".into();
            }
            "pause" => {
                if t.status == "running" {
                    t.status = "paused".into();
                    t.deadline = None;
                }
            }
            "reset" => {
                t.status = "idle".into();
                t.deadline = None;
                t.duration_seconds = 60
                    * match t.phase.as_str() {
                        "short-break" => config.short_break_minutes,
                        "long-break" => config.long_break_minutes,
                        _ => config.focus_minutes,
                    };
                t.remaining_seconds = t.duration_seconds;
            }
            _ => return Err("Unknown timer action".into()),
        }
        self.kv_set("timer", &t)?;
        Ok(t)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadline_survives_sleep_and_finishes_once() {
        let t = Timer {
            status: "running".into(),
            deadline: Some(10_000),
            ..Default::default()
        };
        let done = t.at(50_000);
        assert_eq!(done.status, "completed");
        assert_eq!(done.remaining_seconds, 0);
        assert_eq!(done.clone().at(60_000).completed_focus, 1);
    }
}
