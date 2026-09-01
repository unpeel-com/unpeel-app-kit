//! Shared, clamped navigation and viewport state for App Kit lists.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Position, Rect};
use serde::{Deserialize, Serialize};

/// What PageUp/PageDown change for a list.
///
/// Catalogs normally move selection. Dashboards such as Usage can retain a
/// selected item while paging only the viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListPageBehavior {
    #[default]
    Selection,
    Scroll,
}

/// Closed keyboard vocabulary shared by every flat App Kit list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListNavigationAction {
    Down,
    Up,
    First,
    Last,
    PageDown,
    PageUp,
    Activate,
    Back,
}

/// Observable result after applying one navigation action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListNavigationOutcome {
    None,
    SelectionChanged(usize),
    Scrolled(usize),
    Activate(usize),
    Back,
}

/// App Kit's standard list keymap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListKeymap {
    space_pages_down: bool,
}

impl ListKeymap {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            space_pages_down: false,
        }
    }

    /// Enables Space as an alias for PageDown for Apps that already expose it.
    #[must_use]
    pub const fn space_pages_down(mut self, enabled: bool) -> Self {
        self.space_pages_down = enabled;
        self
    }

    #[must_use]
    pub const fn pages_down_with_space(&self) -> bool {
        self.space_pages_down
    }

    /// Maps a crossterm press/repeat event into the shared list vocabulary.
    #[must_use]
    pub fn action_for_key(&self, key: &KeyEvent) -> Option<ListNavigationAction> {
        if matches!(key.kind, KeyEventKind::Release) {
            return None;
        }
        let command_modifier = key.modifiers.intersects(
            KeyModifiers::CONTROL
                | KeyModifiers::ALT
                | KeyModifiers::SUPER
                | KeyModifiers::HYPER
                | KeyModifiers::META,
        );
        match key.code {
            KeyCode::Down => Some(ListNavigationAction::Down),
            KeyCode::Up => Some(ListNavigationAction::Up),
            KeyCode::Home => Some(ListNavigationAction::First),
            KeyCode::End => Some(ListNavigationAction::Last),
            KeyCode::PageDown => Some(ListNavigationAction::PageDown),
            KeyCode::PageUp => Some(ListNavigationAction::PageUp),
            KeyCode::Enter => Some(ListNavigationAction::Activate),
            KeyCode::Esc => Some(ListNavigationAction::Back),
            KeyCode::Char(' ') if self.space_pages_down && !command_modifier => {
                Some(ListNavigationAction::PageDown)
            }
            KeyCode::Char('j') if !command_modifier => Some(ListNavigationAction::Down),
            KeyCode::Char('k') if !command_modifier => Some(ListNavigationAction::Up),
            KeyCode::Char('g') if !command_modifier => Some(ListNavigationAction::First),
            KeyCode::Char('G') if !command_modifier => Some(ListNavigationAction::Last),
            KeyCode::Char('q') if !command_modifier => Some(ListNavigationAction::Back),
            _ => None,
        }
    }
}

/// Selection, scroll offset, and last-rendered geometry for one flat list.
///
/// Selection always clamps at the first/last item. The state never wraps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListState {
    selected: Option<usize>,
    offset: usize,
    viewport_rows: usize,
    scroll_padding: usize,
    page_overlap: usize,
    page_behavior: ListPageBehavior,
    reveal_selected: bool,
    rows_area: Rect,
    spinner_frame: usize,
}

impl Default for ListState {
    fn default() -> Self {
        Self {
            selected: None,
            offset: 0,
            viewport_rows: 0,
            scroll_padding: 0,
            page_overlap: 1,
            page_behavior: ListPageBehavior::Selection,
            reveal_selected: true,
            rows_area: Rect::default(),
            spinner_frame: 0,
        }
    }
}

impl ListState {
    #[must_use]
    pub const fn new(selected: Option<usize>) -> Self {
        Self {
            selected,
            offset: 0,
            viewport_rows: 0,
            scroll_padding: 0,
            page_overlap: 1,
            page_behavior: ListPageBehavior::Selection,
            reveal_selected: true,
            rows_area: Rect::new(0, 0, 0, 0),
            spinner_frame: 0,
        }
    }

    #[must_use]
    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    #[must_use]
    pub const fn viewport_rows(&self) -> usize {
        self.viewport_rows
    }

    #[must_use]
    pub const fn rows_area(&self) -> Rect {
        self.rows_area
    }

    #[must_use]
    pub const fn spinner_frame(&self) -> usize {
        self.spinner_frame
    }

    pub const fn set_spinner_frame(&mut self, frame: usize) {
        self.spinner_frame = frame;
    }

    pub const fn set_navigation(
        &mut self,
        scroll_padding: usize,
        page_overlap: usize,
        page_behavior: ListPageBehavior,
    ) {
        self.scroll_padding = scroll_padding;
        self.page_overlap = page_overlap;
        self.page_behavior = page_behavior;
    }

    pub fn select(&mut self, selected: Option<usize>, item_count: usize) -> bool {
        let next = if item_count == 0 {
            None
        } else {
            selected.map(|index| index.min(item_count - 1))
        };
        let changed = next != self.selected;
        self.selected = next;
        if changed {
            self.reveal_selected = true;
        }
        changed
    }

    pub const fn request_reveal(&mut self) {
        self.reveal_selected = true;
    }

    pub fn set_offset(&mut self, offset: usize, item_count: usize) {
        self.offset = offset.min(self.max_offset(item_count));
        self.reveal_selected = false;
    }

    pub fn scroll_by(&mut self, delta: isize, item_count: usize) -> bool {
        let next = self
            .offset
            .saturating_add_signed(delta)
            .min(self.max_offset(item_count));
        let changed = next != self.offset;
        self.offset = next;
        self.reveal_selected = false;
        changed
    }

    #[must_use]
    pub fn max_offset(&self, item_count: usize) -> usize {
        item_count.saturating_sub(self.viewport_rows)
    }

    /// Applies one shared key action using the last rendered viewport size.
    pub fn navigate(
        &mut self,
        action: ListNavigationAction,
        item_count: usize,
    ) -> ListNavigationOutcome {
        if action == ListNavigationAction::Back {
            return ListNavigationOutcome::Back;
        }
        if item_count == 0 {
            self.selected = None;
            self.offset = 0;
            return ListNavigationOutcome::None;
        }
        let current = self.selected.unwrap_or(0).min(item_count - 1);
        if action == ListNavigationAction::Activate {
            self.selected = Some(current);
            return ListNavigationOutcome::Activate(current);
        }
        let page = self.viewport_rows.saturating_sub(self.page_overlap).max(1);
        if self.page_behavior == ListPageBehavior::Scroll
            && matches!(
                action,
                ListNavigationAction::PageDown | ListNavigationAction::PageUp
            )
        {
            let delta = if action == ListNavigationAction::PageDown {
                isize::try_from(page).unwrap_or(isize::MAX)
            } else {
                -isize::try_from(page).unwrap_or(isize::MAX)
            };
            return if self.scroll_by(delta, item_count) {
                ListNavigationOutcome::Scrolled(self.offset)
            } else {
                ListNavigationOutcome::None
            };
        }
        let next = match action {
            ListNavigationAction::Down => current.saturating_add(1),
            ListNavigationAction::Up => current.saturating_sub(1),
            ListNavigationAction::First => 0,
            ListNavigationAction::Last => item_count - 1,
            ListNavigationAction::PageDown => current.saturating_add(page),
            ListNavigationAction::PageUp => current.saturating_sub(page),
            ListNavigationAction::Activate | ListNavigationAction::Back => current,
        }
        .min(item_count - 1);
        if self.select(Some(next), item_count) {
            self.reveal(item_count);
            ListNavigationOutcome::SelectionChanged(next)
        } else {
            self.reveal(item_count);
            ListNavigationOutcome::None
        }
    }

    /// Updates geometry and resolves selection/scroll bounds before rendering.
    pub(crate) fn prepare(&mut self, rows_area: Rect, item_count: usize) {
        self.rows_area = rows_area;
        self.viewport_rows = usize::from(rows_area.height);
        if item_count == 0 {
            self.selected = None;
            self.offset = 0;
            self.reveal_selected = false;
            return;
        }
        if let Some(selected) = self.selected {
            self.selected = Some(selected.min(item_count - 1));
        }
        self.offset = self.offset.min(self.max_offset(item_count));
        if self.reveal_selected {
            self.reveal(item_count);
        }
    }

    pub fn reveal(&mut self, item_count: usize) {
        let Some(selected) = self.selected else {
            self.offset = self.offset.min(self.max_offset(item_count));
            self.reveal_selected = false;
            return;
        };
        if self.viewport_rows == 0 {
            self.offset = 0;
            return;
        }
        let padding = self
            .scroll_padding
            .min(self.viewport_rows.saturating_sub(1) / 2);
        let first = self.offset.saturating_add(padding);
        let last_exclusive = self
            .offset
            .saturating_add(self.viewport_rows.saturating_sub(padding));
        if selected < first {
            self.offset = selected.saturating_sub(padding);
        } else if selected >= last_exclusive {
            self.offset = selected
                .saturating_add(1)
                .saturating_add(padding)
                .saturating_sub(self.viewport_rows);
        }
        self.offset = self.offset.min(self.max_offset(item_count));
        self.reveal_selected = false;
    }

    /// Maps a point in the last rendered row rectangle to a model index.
    #[must_use]
    pub fn item_at(&self, position: Position, item_count: usize) -> Option<usize> {
        if !self.rows_area.contains(position) {
            return None;
        }
        let index = self
            .offset
            .saturating_add(usize::from(position.y.saturating_sub(self.rows_area.y)));
        (index < item_count).then_some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn keymap_is_shared_and_space_is_explicit() {
        let standard = ListKeymap::new();
        assert_eq!(
            standard.action_for_key(&key(KeyCode::Char('j'))),
            Some(ListNavigationAction::Down)
        );
        assert_eq!(
            standard.action_for_key(&key(KeyCode::Char('q'))),
            Some(ListNavigationAction::Back)
        );
        assert_eq!(standard.action_for_key(&key(KeyCode::Char(' '))), None);
        assert_eq!(
            standard
                .space_pages_down(true)
                .action_for_key(&key(KeyCode::Char(' '))),
            Some(ListNavigationAction::PageDown)
        );
    }

    #[test]
    fn selection_clamps_and_reveals_with_padding() {
        let mut state = ListState::new(Some(0));
        state.set_navigation(1, 1, ListPageBehavior::Selection);
        state.prepare(Rect::new(0, 0, 20, 4), 10);
        assert_eq!(
            state.navigate(ListNavigationAction::Up, 10),
            ListNavigationOutcome::None
        );
        assert_eq!(state.selected(), Some(0));
        for _ in 0..9 {
            state.navigate(ListNavigationAction::Down, 10);
        }
        assert_eq!(state.selected(), Some(9));
        assert_eq!(state.offset(), 6);
        assert_eq!(
            state.navigate(ListNavigationAction::Down, 10),
            ListNavigationOutcome::None
        );
        assert_eq!(state.selected(), Some(9));
    }

    #[test]
    fn page_actions_can_move_selection_or_only_scroll() {
        let mut selection = ListState::new(Some(1));
        selection.prepare(Rect::new(0, 0, 20, 5), 20);
        assert_eq!(
            selection.navigate(ListNavigationAction::PageDown, 20),
            ListNavigationOutcome::SelectionChanged(5)
        );

        let mut scroll = ListState::new(Some(1));
        scroll.set_navigation(0, 1, ListPageBehavior::Scroll);
        scroll.prepare(Rect::new(0, 0, 20, 5), 20);
        assert_eq!(
            scroll.navigate(ListNavigationAction::PageDown, 20),
            ListNavigationOutcome::Scrolled(4)
        );
        assert_eq!(scroll.selected(), Some(1));
    }
}
