#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollState {
    /// Zero-based index of the first visible line in the current viewport.
    pub offset: usize,
    /// Number of lines visible in the current viewport (minimum 1).
    pub page_size: usize,
}

impl ScrollState {
    pub fn set_page_size(&mut self, page_size: usize, total_lines: usize) {
        self.page_size = page_size.max(1);
        self.clamp(total_lines);
    }

    pub fn clamp(&mut self, total_lines: usize) {
        let max_offset = self.max_offset(total_lines);
        if self.offset > max_offset {
            self.offset = max_offset;
        }
    }

    pub fn scroll_by(&mut self, delta: i32, total_lines: usize) {
        if delta < 0 {
            self.offset = self.offset.saturating_sub(delta.unsigned_abs() as usize);
        } else {
            self.offset = self.offset.saturating_add(delta as usize);
        }
        self.clamp(total_lines);
    }

    pub fn page_up(&mut self, total_lines: usize) {
        let delta = self.page_size.max(1) as i32;
        self.scroll_by(-delta, total_lines);
    }

    pub fn page_down(&mut self, total_lines: usize) {
        let delta = self.page_size.max(1) as i32;
        self.scroll_by(delta, total_lines);
    }

    pub fn home(&mut self) {
        self.offset = 0;
    }

    pub fn end(&mut self, total_lines: usize) {
        self.offset = self.max_offset(total_lines);
    }

    pub fn max_offset(&self, total_lines: usize) -> usize {
        total_lines.saturating_sub(self.page_size.max(1))
    }
}

#[cfg(test)]
mod tests {
    use crate::app::scroll_state::ScrollState;

    #[test]
    fn set_page_size_keeps_page_size_at_least_one() {
        let mut scroll = ScrollState {
            offset: 0,
            page_size: 3,
        };
        scroll.set_page_size(0, 10);
        assert_eq!(scroll.page_size, 1);
    }

    #[test]
    fn clamp_limits_offset_to_last_page() {
        let mut scroll = ScrollState {
            offset: 20,
            page_size: 5,
        };
        scroll.clamp(9);
        assert_eq!(scroll.offset, 4);
    }

    #[test]
    fn scroll_by_moves_and_clamps() {
        let mut scroll = ScrollState {
            offset: 2,
            page_size: 3,
        };
        scroll.scroll_by(10, 8);
        assert_eq!(scroll.offset, 5);
        scroll.scroll_by(-10, 8);
        assert_eq!(scroll.offset, 0);
    }

    #[test]
    fn page_navigation_uses_page_size() {
        let mut scroll = ScrollState {
            offset: 6,
            page_size: 4,
        };
        scroll.page_up(20);
        assert_eq!(scroll.offset, 2);
        scroll.page_down(20);
        assert_eq!(scroll.offset, 6);
    }

    #[test]
    fn home_and_end_jump_to_bounds() {
        let mut scroll = ScrollState {
            offset: 6,
            page_size: 4,
        };
        scroll.home();
        assert_eq!(scroll.offset, 0);
        scroll.end(15);
        assert_eq!(scroll.offset, 11);
    }

    #[test]
    fn max_offset_is_zero_when_content_fits() {
        let scroll = ScrollState {
            offset: 0,
            page_size: 10,
        };
        assert_eq!(scroll.max_offset(6), 0);
    }
}
