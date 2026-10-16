use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(self) -> i32 {
        self.right - self.left
    }
    pub fn height(self) -> i32 {
        self.bottom - self.top
    }
    pub fn intersects(self, other: Self) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }
    pub fn contains(self, x: i32, y: i32, margin: i32) -> bool {
        x >= self.left - margin
            && x < self.right + margin
            && y >= self.top - margin
            && y < self.bottom + margin
    }
}

impl From<windows::Win32::Foundation::RECT> for Rect {
    fn from(r: windows::Win32::Foundation::RECT) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

/// Merge occupied intervals before calculating gaps; nested UIA rectangles must
/// never manufacture space inside another control. Inputs are physical pixels.
pub fn largest_gap(bar: Rect, occupied: &[Rect], minimum: i32) -> Option<(i32, i32)> {
    if bar.width() <= 0 || bar.height() <= 0 || minimum <= 0 {
        return None;
    }
    let mut intervals: Vec<_> = occupied
        .iter()
        .filter(|r| r.intersects(bar))
        .map(|r| (r.left.max(bar.left), r.right.min(bar.right)))
        .filter(|(a, b)| b > a)
        .collect();
    if intervals.is_empty() {
        return None;
    } // Empty UIA output is not proof of empty taskbar space.
    intervals.sort_unstable();
    let mut cursor = bar.left;
    let mut gaps = Vec::new();
    for (left, right) in intervals {
        if left - cursor >= minimum {
            gaps.push((cursor, left));
        }
        cursor = cursor.max(right);
    }
    if bar.right - cursor >= minimum {
        gaps.push((cursor, bar.right));
    }
    // Largest usable gap; prefer the rightmost one on ties (icons → system tray).
    gaps.into_iter().max_by_key(|(a, b)| (b - a, *a))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(a: i32, b: i32) -> Rect {
        Rect {
            left: a,
            right: b,
            top: 1040,
            bottom: 1080,
        }
    }
    #[test]
    fn overlapping_controls_are_merged() {
        assert_eq!(
            largest_gap(
                rect(0, 1920),
                &[rect(0, 200), rect(150, 800), rect(1800, 1920)],
                168
            ),
            Some((800, 1800))
        );
    }
    #[test]
    fn widths_are_physical_at_each_required_scale() {
        for scale in [1.0, 1.25, 1.5] {
            let minimum = (168.0_f64 * scale).ceil() as i32;
            assert!(largest_gap(
                rect(0, 1920),
                &[rect(0, 800), rect(800 + minimum, 1920)],
                minimum
            )
            .is_some());
            assert!(largest_gap(
                rect(0, 1920),
                &[rect(0, 800), rect(799 + minimum, 1920)],
                minimum
            )
            .is_none());
        }
    }
    #[test]
    fn empty_or_full_output_falls_back() {
        assert!(largest_gap(rect(0, 1920), &[], 168).is_none());
        assert!(largest_gap(rect(0, 1920), &[rect(0, 1920)], 168).is_none());
    }
    #[test]
    fn negative_monitor_coordinates_and_right_ties() {
        assert_eq!(
            largest_gap(
                rect(-1920, 0),
                &[rect(-1920, -1820), rect(-1220, -1000), rect(-400, 0)],
                168
            ),
            Some((-1000, -400))
        );
    }
}
