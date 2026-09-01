//! Shared focus navigation and viewport state for App Kit row collections.

use std::ops::{Deref, DerefMut};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent};
use ratatui::layout::{Position, Rect};
use serde::{Deserialize, Serialize};

use crate::{TerminalPointerPhase, TerminalPointerState};

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

/// How single-row movement behaves at the first and last item.
///
/// Flat Lists clamp. Explorer retains its established wrap behavior while
/// using the exact same focus/viewport engine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowBoundaryBehavior {
    #[default]
    Clamp,
    Wrap,
}

/// Primary semantic role of the focused row.
///
/// The navigation layer needs only this closed behavior hint; component ids,
/// actions, persistence, and routing remain App-owned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowPrimaryRole {
    #[default]
    Static,
    Toggle,
    Checkmark,
    Disclosure,
    Command,
    Destructive,
}

impl RowPrimaryRole {
    #[must_use]
    pub const fn is_interactive(self) -> bool {
        !matches!(self, Self::Static)
    }
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

/// Role-aware result of interpreting one key for a focused row collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKeyDecision {
    Navigate(ListNavigationAction),
    InvokePrimary,
}

/// Role-aware result of clicking one row in a terminal collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowPointerDecision {
    Select(usize),
    InvokePrimary(usize),
}

/// App Kit's standard list keymap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListKeymap {
    space_pages_down: bool,
    character_aliases: bool,
}

impl ListKeymap {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            space_pages_down: false,
            character_aliases: true,
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

    /// Enables or disables `j`/`k`/`g`/`G`/`q` navigation aliases.
    ///
    /// Explorer disables them because printable characters focus its filter.
    #[must_use]
    pub const fn character_aliases(mut self, enabled: bool) -> Self {
        self.character_aliases = enabled;
        self
    }

    #[must_use]
    pub const fn uses_character_aliases(&self) -> bool {
        self.character_aliases
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
            KeyCode::Char('j') if self.character_aliases && !command_modifier => {
                Some(ListNavigationAction::Down)
            }
            KeyCode::Char('k') if self.character_aliases && !command_modifier => {
                Some(ListNavigationAction::Up)
            }
            KeyCode::Char('g') if self.character_aliases && !command_modifier => {
                Some(ListNavigationAction::First)
            }
            KeyCode::Char('G') if self.character_aliases && !command_modifier => {
                Some(ListNavigationAction::Last)
            }
            KeyCode::Char('q') if self.character_aliases && !command_modifier => {
                Some(ListNavigationAction::Back)
            }
            _ => None,
        }
    }

    /// Applies the one App Kit keyboard decision table for a focused row.
    ///
    /// Enter invokes the row's primary role. Space invokes only a Toggle;
    /// otherwise it pages down. Escape/back aliases remain navigation so the
    /// Page/App can route them through its authoritative back action.
    #[must_use]
    pub fn decision_for_key(
        &self,
        key: &KeyEvent,
        primary_role: RowPrimaryRole,
    ) -> Option<RowKeyDecision> {
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
        if command_modifier {
            return None;
        }
        match key.code {
            KeyCode::Enter => primary_role
                .is_interactive()
                .then_some(RowKeyDecision::InvokePrimary),
            KeyCode::Char(' ') if primary_role == RowPrimaryRole::Toggle => {
                Some(RowKeyDecision::InvokePrimary)
            }
            KeyCode::Char(' ') => Some(RowKeyDecision::Navigate(ListNavigationAction::PageDown)),
            _ => self.action_for_key(key).map(RowKeyDecision::Navigate),
        }
    }
}

/// Selection, scroll offset, and last-rendered geometry for a row collection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowNavigationState {
    selected: Option<usize>,
    offset: usize,
    viewport_rows: usize,
    scroll_padding: usize,
    page_overlap: usize,
    page_behavior: ListPageBehavior,
    boundary_behavior: RowBoundaryBehavior,
    reveal_selected: bool,
    rows_area: Rect,
    pointer: TerminalPointerState,
}

impl Default for RowNavigationState {
    fn default() -> Self {
        Self {
            selected: None,
            offset: 0,
            viewport_rows: 0,
            scroll_padding: 0,
            page_overlap: 1,
            page_behavior: ListPageBehavior::Selection,
            boundary_behavior: RowBoundaryBehavior::Clamp,
            reveal_selected: true,
            rows_area: Rect::default(),
            pointer: TerminalPointerState::new(),
        }
    }
}

impl RowNavigationState {
    #[must_use]
    pub const fn new(selected: Option<usize>) -> Self {
        Self {
            selected,
            offset: 0,
            viewport_rows: 0,
            scroll_padding: 0,
            page_overlap: 1,
            page_behavior: ListPageBehavior::Selection,
            boundary_behavior: RowBoundaryBehavior::Clamp,
            reveal_selected: true,
            rows_area: Rect::new(0, 0, 0, 0),
            pointer: TerminalPointerState::new(),
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
    pub const fn pointer(&self) -> TerminalPointerState {
        self.pointer
    }

    pub const fn set_pointer(&mut self, pointer: TerminalPointerState) {
        self.pointer = pointer;
    }

    /// Feeds renderer-local pointer state shared by List, Tree, and Explorer.
    pub fn track_mouse(&mut self, event: &MouseEvent) -> bool {
        self.pointer.track(event)
    }

    #[must_use]
    pub fn pointer_phase_at(&self, index: usize) -> TerminalPointerPhase {
        self.row_area(index)
            .map_or(TerminalPointerPhase::Idle, |area| self.pointer.phase(area))
    }

    #[must_use]
    pub fn hovered_item(&self, item_count: usize) -> Option<usize> {
        self.pointer
            .position()
            .and_then(|position| self.item_at(position, item_count))
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

    pub const fn set_boundary_behavior(&mut self, behavior: RowBoundaryBehavior) {
        self.boundary_behavior = behavior;
    }

    #[must_use]
    pub const fn boundary_behavior(&self) -> RowBoundaryBehavior {
        self.boundary_behavior
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
            ListNavigationAction::Down
                if self.boundary_behavior == RowBoundaryBehavior::Wrap
                    && current == item_count - 1 =>
            {
                0
            }
            ListNavigationAction::Up
                if self.boundary_behavior == RowBoundaryBehavior::Wrap && current == 0 =>
            {
                item_count - 1
            }
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

    #[must_use]
    fn row_area(&self, index: usize) -> Option<Rect> {
        let slot = index.checked_sub(self.offset)?;
        let row = u16::try_from(slot).ok()?;
        (row < self.rows_area.height).then(|| {
            Rect::new(
                self.rows_area.x,
                self.rows_area.y.saturating_add(row),
                self.rows_area.width,
                1,
            )
        })
    }
}

/// List-specific render state layered over behavior-agnostic row focus.
///
/// Navigation methods are available through `Deref`; the spinner frame stays
/// here so Explorer/Tree do not inherit presentation state from the focus
/// engine they share with List.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListState {
    navigation: RowNavigationState,
    spinner_frame: usize,
}

impl ListState {
    #[must_use]
    pub const fn new(selected: Option<usize>) -> Self {
        Self {
            navigation: RowNavigationState::new(selected),
            spinner_frame: 0,
        }
    }

    #[must_use]
    pub const fn navigation(&self) -> &RowNavigationState {
        &self.navigation
    }

    pub const fn navigation_mut(&mut self) -> &mut RowNavigationState {
        &mut self.navigation
    }

    #[must_use]
    pub const fn spinner_frame(&self) -> usize {
        self.spinner_frame
    }

    pub const fn set_spinner_frame(&mut self, frame: usize) {
        self.spinner_frame = frame;
    }
}

impl Deref for ListState {
    type Target = RowNavigationState;

    fn deref(&self) -> &Self::Target {
        &self.navigation
    }
}

impl DerefMut for ListState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.navigation
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
            standard.decision_for_key(&key(KeyCode::Enter), RowPrimaryRole::Toggle),
            Some(RowKeyDecision::InvokePrimary)
        );
        assert_eq!(
            standard.decision_for_key(&key(KeyCode::Char(' ')), RowPrimaryRole::Toggle),
            Some(RowKeyDecision::InvokePrimary)
        );
        assert_eq!(
            standard.decision_for_key(&key(KeyCode::Char(' ')), RowPrimaryRole::Disclosure),
            Some(RowKeyDecision::Navigate(ListNavigationAction::PageDown))
        );
        assert_eq!(
            standard.decision_for_key(&key(KeyCode::Enter), RowPrimaryRole::Static),
            None
        );
        assert_eq!(
            standard.decision_for_key(
                &KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL),
                RowPrimaryRole::Command,
            ),
            None
        );
        assert_eq!(
            standard
                .character_aliases(false)
                .action_for_key(&key(KeyCode::Char('j'))),
            None
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

    #[test]
    fn explorer_boundary_policy_wraps_steps_but_clamps_pages() {
        let mut state = RowNavigationState::new(Some(0));
        state.set_boundary_behavior(RowBoundaryBehavior::Wrap);
        state.set_navigation(0, 0, ListPageBehavior::Selection);
        state.prepare(Rect::new(0, 0, 20, 3), 10);
        assert_eq!(
            state.navigate(ListNavigationAction::Up, 10),
            ListNavigationOutcome::SelectionChanged(9)
        );
        assert_eq!(
            state.navigate(ListNavigationAction::Down, 10),
            ListNavigationOutcome::SelectionChanged(0)
        );
        assert_eq!(
            state.navigate(ListNavigationAction::PageUp, 10),
            ListNavigationOutcome::None
        );
        state.select(Some(9), 10);
        assert_eq!(
            state.navigate(ListNavigationAction::PageDown, 10),
            ListNavigationOutcome::None
        );
    }
}
