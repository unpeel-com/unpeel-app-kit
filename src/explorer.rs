use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthChar;

use crate::{ColorScheme, DragSurface, KitTheme, SELECTABLE_LEFT_PADDING, VerticalScrollbar};

/// One item in the current directory shown by [`Explorer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorerEntry {
    path: PathBuf,
    name: String,
    directory: bool,
    symlink: bool,
    parent: bool,
}

impl ExplorerEntry {
    /// Absolute Host-local path represented by this row.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Sanitized filename without the directory suffix.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether this entry can be entered as a directory.
    #[must_use]
    pub const fn is_directory(&self) -> bool {
        self.directory
    }

    /// Whether this path itself is a symbolic link.
    #[must_use]
    pub const fn is_symlink(&self) -> bool {
        self.symlink
    }

    /// Whether this is the synthetic `../` entry.
    #[must_use]
    pub const fn is_parent(&self) -> bool {
        self.parent
    }

    /// Display label, including `/` for navigable directories.
    #[must_use]
    pub fn display_name(&self) -> String {
        if self.parent {
            "../".to_owned()
        } else if self.directory {
            format!("{}/", self.name)
        } else {
            self.name.clone()
        }
    }
}

/// Backend-neutral actions understood by [`Explorer::handle`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ExplorerInput {
    Up,
    Down,
    First,
    Last,
    PageUp,
    PageDown,
    Parent,
    Open,
    ToggleHidden,
    Refresh,
    FocusFilter,
    BlurFilter,
    ClearFilter,
    FilterCharacter(char),
    FilterBackspace,
    #[default]
    None,
}

/// Observable result of handling an [`ExplorerInput`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ExplorerEvent {
    #[default]
    None,
    SelectionChanged,
    DirectoryChanged(PathBuf),
    FileActivated(PathBuf),
    Refreshed,
    FilterChanged,
    FilterFocusChanged,
}

/// Borderless visual styling for [`Explorer`].
///
/// There is deliberately no `Block` or border field. Apps can place the
/// component inside their own layout without inheriting IDE-like chrome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorerTheme {
    pub style: Style,
    pub filter: Style,
    pub filter_focused: Style,
    pub filter_placeholder: Style,
    pub path: Style,
    pub item: Style,
    pub directory: Style,
    pub symlink: Style,
    pub parent: Style,
    pub selected: Style,
    pub empty: Style,
    pub scrollbar_track: Style,
    pub scrollbar_thumb: Style,
    /// Optional marker whose width is reserved on every row to avoid jitter.
    pub selected_symbol: Option<String>,
    /// Blank columns before the header and item labels.
    pub left_padding: u16,
    /// Rows kept visible above and below the selection when possible.
    pub scroll_padding: usize,
}

impl Default for ExplorerTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl ExplorerTheme {
    /// Defaults tuned for a dark terminal while leaving its base background
    /// untouched.
    #[must_use]
    pub const fn dark() -> Self {
        Self::for_palette(KitTheme::dark())
    }

    /// Defaults tuned for a light terminal while leaving its base background
    /// untouched.
    #[must_use]
    pub const fn light() -> Self {
        Self::for_palette(KitTheme::light())
    }

    #[must_use]
    pub const fn for_color_scheme(scheme: ColorScheme) -> Self {
        Self::for_palette(KitTheme::for_scheme(scheme))
    }

    #[must_use]
    pub fn detected() -> Self {
        Self::for_color_scheme(ColorScheme::detect())
    }

    const fn for_palette(palette: KitTheme) -> Self {
        Self {
            style: Style::new(),
            filter: Style::new().fg(palette.muted),
            filter_focused: Style::new().fg(palette.text).add_modifier(Modifier::BOLD),
            filter_placeholder: Style::new().fg(palette.subtle),
            path: Style::new().add_modifier(Modifier::BOLD),
            item: Style::new().fg(palette.text),
            directory: Style::new().fg(palette.accent),
            symlink: Style::new().fg(Color::Cyan),
            parent: Style::new().fg(palette.accent),
            selected: palette.selected_row,
            empty: Style::new().fg(palette.subtle),
            scrollbar_track: palette.scrollbar_track,
            scrollbar_thumb: palette.scrollbar_thumb,
            selected_symbol: None,
            left_padding: SELECTABLE_LEFT_PADDING,
            scroll_padding: 1,
        }
    }
}

/// A flat, borderless filesystem explorer for Ratatui Apps.
///
/// The component owns the current directory, selection, hidden-file policy,
/// paging, viewport scroll, rendering, and terminal-cell drag registration.
/// Input conversion and what to do when a file is activated remain App-owned.
#[derive(Debug)]
pub struct Explorer {
    cwd: PathBuf,
    all_entries: Vec<ExplorerEntry>,
    entries: Vec<ExplorerEntry>,
    selected: usize,
    show_hidden: bool,
    show_filter: bool,
    filter: String,
    filter_focused: bool,
    show_path: bool,
    theme: ExplorerTheme,
    scroll: usize,
    viewport_rows: usize,
    area: Rect,
    filter_area: Rect,
    list_area: Rect,
}

impl Explorer {
    /// Opens a directory, or opens a file's parent and selects that file.
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let requested = fs::canonicalize(path)?;
        let metadata = fs::metadata(&requested)?;
        let (cwd, preferred) = if metadata.is_dir() {
            (requested, None)
        } else {
            let parent = requested.parent().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "file has no parent directory")
            })?;
            (parent.to_path_buf(), Some(requested))
        };
        let all_entries = read_entries(&cwd, false)?;
        let entries = all_entries.clone();
        let selected = preferred
            .as_deref()
            .and_then(|path| entries.iter().position(|entry| entry.path == path))
            .unwrap_or(0)
            .min(entries.len().saturating_sub(1));
        Ok(Self {
            cwd,
            all_entries,
            entries,
            selected,
            show_hidden: false,
            show_filter: true,
            filter: String::new(),
            filter_focused: false,
            show_path: true,
            theme: ExplorerTheme::default(),
            scroll: 0,
            viewport_rows: 12,
            area: Rect::default(),
            filter_area: Rect::default(),
            list_area: Rect::default(),
        })
    }

    /// Applies a borderless visual theme.
    #[must_use]
    pub fn with_theme(mut self, theme: ExplorerTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Replaces the visual theme without changing navigation state.
    pub fn set_theme(&mut self, theme: ExplorerTheme) {
        self.theme = theme;
    }

    #[must_use]
    pub const fn theme(&self) -> &ExplorerTheme {
        &self.theme
    }

    /// Shows or hides the always-borderless filter row.
    pub const fn set_show_filter(&mut self, show: bool) {
        self.show_filter = show;
    }

    #[must_use]
    pub const fn show_filter(&self) -> bool {
        self.show_filter
    }

    /// Current case-insensitive filename filter.
    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
    }

    #[must_use]
    pub const fn filter_focused(&self) -> bool {
        self.filter_focused
    }

    /// Replaces the filter, dropping terminal control characters.
    pub fn set_filter(&mut self, filter: impl Into<String>) -> bool {
        let next = filter
            .into()
            .chars()
            .filter(|character| !character.is_control())
            .collect::<String>();
        if next == self.filter {
            return false;
        }
        let preferred = self.selected().map(|entry| entry.path.clone());
        self.filter = next;
        self.rebuild_filtered(preferred.as_deref());
        true
    }

    /// Clears the filename filter while preserving the selected path.
    pub fn clear_filter(&mut self) -> bool {
        self.set_filter(String::new())
    }

    /// Focuses or blurs the filter row.
    pub fn set_filter_focused(&mut self, focused: bool) -> bool {
        let changed = self.filter_focused != focused;
        self.filter_focused = focused;
        changed
    }

    /// Shows or hides the draggable current-directory header.
    pub const fn set_show_path(&mut self, show: bool) {
        self.show_path = show;
    }

    #[must_use]
    pub const fn show_path(&self) -> bool {
        self.show_path
    }

    /// Absolute current directory.
    #[must_use]
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Entries in display order after filtering, including `../` when a
    /// parent exists. The parent remains available for navigation even when
    /// its label does not match the filter.
    #[must_use]
    pub fn entries(&self) -> &[ExplorerEntry] {
        &self.entries
    }

    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    #[must_use]
    pub fn selected(&self) -> Option<&ExplorerEntry> {
        self.entries.get(self.selected)
    }

    #[must_use]
    pub const fn show_hidden(&self) -> bool {
        self.show_hidden
    }

    #[must_use]
    pub const fn scroll_offset(&self) -> usize {
        self.scroll
    }

    /// Number of matching files and folders, excluding the synthetic parent.
    #[must_use]
    pub fn match_count(&self) -> usize {
        self.entries.iter().filter(|entry| !entry.parent).count()
    }

    /// Number of unfiltered files and folders, excluding the synthetic parent.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.all_entries
            .iter()
            .filter(|entry| !entry.parent)
            .count()
    }

    /// Most recently rendered outer area.
    #[must_use]
    pub const fn area(&self) -> Rect {
        self.area
    }

    /// Most recently rendered filter row.
    #[must_use]
    pub const fn filter_area(&self) -> Rect {
        self.filter_area
    }

    /// Most recently rendered item viewport, excluding filter, path header,
    /// and scrollbar.
    #[must_use]
    pub const fn list_area(&self) -> Rect {
        self.list_area
    }

    /// Returns the item under a terminal cell from the most recent render.
    #[must_use]
    pub fn entry_at(&self, position: Position) -> Option<&ExplorerEntry> {
        self.entry_index_at(position)
            .and_then(|index| self.entries.get(index))
    }

    /// Selects the item under a terminal cell from the most recent render.
    pub fn select_at(&mut self, position: Position) -> bool {
        let Some(index) = self.entry_index_at(position) else {
            return false;
        };
        self.set_selected_index(index)
    }

    /// Selects an entry by index, clamped to the available entries.
    pub fn set_selected_index(&mut self, index: usize) -> bool {
        if self.entries.is_empty() {
            self.selected = 0;
            return false;
        }
        let next = index.min(self.entries.len() - 1);
        let changed = next != self.selected;
        self.selected = next;
        changed
    }

    /// Selects the entry matching an absolute path.
    pub fn select_path(&mut self, path: impl AsRef<Path>) -> bool {
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.path == path.as_ref())
        else {
            return false;
        };
        self.set_selected_index(index)
    }

    /// Changes directory and resets selection to its first entry.
    pub fn set_cwd(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let cwd = canonical_directory(path.as_ref())?;
        let all_entries = read_entries(&cwd, self.show_hidden)?;
        self.cwd = cwd;
        self.all_entries = all_entries;
        self.filter.clear();
        self.filter_focused = false;
        self.rebuild_filtered(None);
        Ok(())
    }

    /// Refreshes the current directory while preserving the selected path.
    pub fn refresh(&mut self) -> io::Result<()> {
        let preferred = self.selected().map(|entry| entry.path.clone());
        let previous = self.selected;
        let all_entries = read_entries(&self.cwd, self.show_hidden)?;
        self.all_entries = all_entries;
        self.rebuild_filtered(preferred.as_deref());
        if preferred.is_none() {
            self.selected = previous.min(self.entries.len().saturating_sub(1));
        }
        self.clamp_scroll();
        Ok(())
    }

    /// Changes hidden-file visibility while preserving selection when possible.
    pub fn set_show_hidden(&mut self, show: bool) -> io::Result<()> {
        if show == self.show_hidden {
            return Ok(());
        }
        let preferred = self.selected().map(|entry| entry.path.clone());
        let previous = self.selected;
        let all_entries = read_entries(&self.cwd, show)?;
        self.all_entries = all_entries;
        self.show_hidden = show;
        self.rebuild_filtered(preferred.as_deref());
        if preferred.is_none() {
            self.selected = previous.min(self.entries.len().saturating_sub(1));
        }
        self.clamp_scroll();
        Ok(())
    }

    /// Updates navigation state from a backend-neutral action.
    pub fn handle(&mut self, input: ExplorerInput) -> io::Result<ExplorerEvent> {
        match input {
            ExplorerInput::Up => Ok(self.move_selection(-1)),
            ExplorerInput::Down => Ok(self.move_selection(1)),
            ExplorerInput::First => Ok(self.select_boundary(false)),
            ExplorerInput::Last => Ok(self.select_boundary(true)),
            ExplorerInput::PageUp => Ok(self.move_selection(-(self.viewport_rows as isize))),
            ExplorerInput::PageDown => Ok(self.move_selection(self.viewport_rows as isize)),
            ExplorerInput::Parent => self.open_parent(),
            ExplorerInput::Open => self.open_selected(),
            ExplorerInput::ToggleHidden => {
                self.set_show_hidden(!self.show_hidden)?;
                Ok(ExplorerEvent::Refreshed)
            }
            ExplorerInput::Refresh => {
                self.refresh()?;
                Ok(ExplorerEvent::Refreshed)
            }
            ExplorerInput::FocusFilter => {
                if self.set_filter_focused(true) {
                    Ok(ExplorerEvent::FilterFocusChanged)
                } else {
                    Ok(ExplorerEvent::None)
                }
            }
            ExplorerInput::BlurFilter => {
                if self.set_filter_focused(false) {
                    Ok(ExplorerEvent::FilterFocusChanged)
                } else {
                    Ok(ExplorerEvent::None)
                }
            }
            ExplorerInput::ClearFilter => {
                if self.clear_filter() {
                    Ok(ExplorerEvent::FilterChanged)
                } else {
                    Ok(ExplorerEvent::None)
                }
            }
            ExplorerInput::FilterCharacter(character) => {
                if character.is_control() {
                    return Ok(ExplorerEvent::None);
                }
                let mut filter = self.filter.clone();
                filter.push(character);
                self.set_filter(filter);
                Ok(ExplorerEvent::FilterChanged)
            }
            ExplorerInput::FilterBackspace => {
                let mut filter = self.filter.clone();
                if filter.pop().is_some() {
                    self.set_filter(filter);
                    Ok(ExplorerEvent::FilterChanged)
                } else {
                    Ok(ExplorerEvent::None)
                }
            }
            ExplorerInput::None => Ok(ExplorerEvent::None),
        }
    }

    /// Returns a Ratatui widget that also registers every rendered path with
    /// the supplied drag surface.
    pub fn widget<'explorer, 'drag>(
        &'explorer mut self,
        drags: &'drag mut DragSurface,
    ) -> ExplorerWidget<'explorer, 'drag> {
        ExplorerWidget {
            explorer: self,
            drags,
        }
    }

    fn entry_index_at(&self, position: Position) -> Option<usize> {
        if !self.list_area.contains(position) {
            return None;
        }
        let index = self
            .scroll
            .saturating_add(usize::from(position.y.saturating_sub(self.list_area.y)));
        (index < self.entries.len()).then_some(index)
    }

    fn rebuild_filtered(&mut self, preferred: Option<&Path>) {
        let needle = self.filter.to_lowercase();
        self.entries = self
            .all_entries
            .iter()
            .filter(|entry| {
                entry.parent || needle.is_empty() || entry.name.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect();
        let first_match = self
            .entries
            .iter()
            .position(|entry| !entry.parent)
            .unwrap_or(0);
        self.selected = preferred
            .and_then(|path| {
                self.entries.iter().position(|entry| {
                    entry.path == path && (self.filter.is_empty() || !entry.parent)
                })
            })
            .unwrap_or(first_match)
            .min(self.entries.len().saturating_sub(1));
        self.scroll = 0;
    }

    fn move_selection(&mut self, delta: isize) -> ExplorerEvent {
        let len = self.entries.len();
        if len == 0 || delta == 0 {
            return ExplorerEvent::None;
        }
        let next = if delta < 0 {
            if delta == -1 && self.selected == 0 {
                len - 1
            } else {
                self.selected.saturating_sub(delta.unsigned_abs())
            }
        } else {
            let candidate = self.selected.saturating_add(delta as usize);
            if candidate >= len {
                if delta == 1 { 0 } else { len - 1 }
            } else {
                candidate
            }
        };
        if self.set_selected_index(next) {
            ExplorerEvent::SelectionChanged
        } else {
            ExplorerEvent::None
        }
    }

    fn select_boundary(&mut self, last: bool) -> ExplorerEvent {
        let index = if last {
            self.entries.len().saturating_sub(1)
        } else {
            0
        };
        if self.set_selected_index(index) {
            ExplorerEvent::SelectionChanged
        } else {
            ExplorerEvent::None
        }
    }

    fn open_parent(&mut self) -> io::Result<ExplorerEvent> {
        let Some(parent) = self.cwd.parent().map(Path::to_path_buf) else {
            return Ok(ExplorerEvent::None);
        };
        self.change_directory(parent)
    }

    fn open_selected(&mut self) -> io::Result<ExplorerEvent> {
        let Some(entry) = self.selected().cloned() else {
            return Ok(ExplorerEvent::None);
        };
        self.filter_focused = false;
        if entry.directory {
            self.change_directory(entry.path)
        } else {
            Ok(ExplorerEvent::FileActivated(entry.path))
        }
    }

    fn change_directory(&mut self, path: PathBuf) -> io::Result<ExplorerEvent> {
        let previous = self.cwd.clone();
        let cwd = canonical_directory(&path)?;
        let all_entries = read_entries(&cwd, self.show_hidden)?;
        let selected = all_entries
            .iter()
            .position(|entry| entry.path == previous)
            .unwrap_or(0)
            .min(all_entries.len().saturating_sub(1));
        self.cwd = cwd;
        self.all_entries = all_entries;
        self.filter.clear();
        self.filter_focused = false;
        self.entries = self.all_entries.clone();
        self.selected = selected;
        self.scroll = 0;
        Ok(ExplorerEvent::DirectoryChanged(self.cwd.clone()))
    }

    fn render(&mut self, area: Rect, buffer: &mut Buffer, drags: &mut DragSurface) {
        self.area = area;
        buffer.set_style(area, self.theme.style);
        if area.is_empty() {
            self.filter_area = Rect::default();
            self.list_area = Rect::default();
            self.viewport_rows = 0;
            self.scroll = 0;
            return;
        }

        let filter_height = u16::from(self.show_filter && area.height > 0);
        self.filter_area = if filter_height == 1 {
            let filter_area = Rect::new(area.x, area.y, area.width, 1);
            self.render_filter(filter_area, buffer);
            filter_area
        } else {
            Rect::default()
        };

        let path_height = u16::from(self.show_path && area.height > filter_height);
        if path_height == 1 {
            let header = Rect::new(area.x, area.y.saturating_add(filter_height), area.width, 1);
            let label_width = area.width.saturating_sub(self.theme.left_padding);
            let label = format!(
                "{}{}",
                " ".repeat(usize::from(self.theme.left_padding.min(area.width))),
                truncate_start(&self.cwd.display().to_string(), label_width),
            );
            let style = self.theme.style.patch(self.theme.path);
            buffer.set_style(header, style);
            Line::styled(label, style).render(header, buffer);
            drags.register(header, &self.cwd);
        }

        let header_height = filter_height.saturating_add(path_height);
        let list_y = area.y.saturating_add(header_height);
        let list_height = area.height.saturating_sub(header_height);
        let overflow = list_height > 0 && self.entries.len() > usize::from(list_height);
        let list_width = area.width.saturating_sub(u16::from(overflow));
        self.list_area = Rect::new(area.x, list_y, list_width, list_height);
        self.viewport_rows = usize::from(list_height);
        self.ensure_selected_visible();

        let visible = usize::from(list_height);
        if self.match_count() == 0 && visible > self.entries.len() {
            let label = format!(
                "{}{}",
                " ".repeat(usize::from(
                    self.theme.left_padding.min(self.list_area.width)
                )),
                if self.filter.is_empty() {
                    "empty folder"
                } else {
                    "no matches"
                },
            );
            let message_area = Rect::new(
                self.list_area.x,
                self.list_area.y.saturating_add(self.entries.len() as u16),
                self.list_area.width,
                1,
            );
            Line::styled(label, self.theme.style.patch(self.theme.empty))
                .render(message_area, buffer);
        }

        for (slot, index) in (self.scroll..self.entries.len()).take(visible).enumerate() {
            let row = Rect::new(
                self.list_area.x,
                self.list_area.y.saturating_add(slot as u16),
                self.list_area.width,
                1,
            );
            let entry = &self.entries[index];
            let selected = index == self.selected;
            let style = self.entry_style(entry, selected);
            buffer.set_style(row, style);
            let label = self.entry_label(entry, selected, row.width);
            Line::styled(label, style).render(row, buffer);
            drags.register(row, &entry.path);
        }

        if overflow {
            let scrollbar = Rect::new(
                area.right().saturating_sub(1),
                list_y,
                area.width.min(1),
                list_height,
            );
            VerticalScrollbar::new(self.entries.len(), usize::from(list_height), self.scroll)
                .track_style(self.theme.scrollbar_track)
                .thumb_style(self.theme.scrollbar_thumb)
                .render(scrollbar, buffer);
        }
    }

    fn render_filter(&self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let padding = " ".repeat(usize::from(self.theme.left_padding.min(area.width)));
        let prompt = "/ ";
        let cursor = if self.filter_focused { "▏" } else { "" };
        let reserved = display_width(&padding)
            .saturating_add(display_width(prompt))
            .saturating_add(display_width(cursor));
        let available = usize::from(area.width).saturating_sub(reserved);
        let content = if self.filter.is_empty() {
            truncate_end("Filter files", available)
        } else {
            truncate_start(&self.filter, u16::try_from(available).unwrap_or(u16::MAX))
        };
        let active = if self.filter_focused {
            self.theme.filter_focused
        } else {
            self.theme.filter
        };
        let content_style = if self.filter.is_empty() && !self.filter_focused {
            self.theme.filter_placeholder
        } else {
            active
        };
        let style = self.theme.style.patch(active);
        buffer.set_style(area, style);
        Line::from(vec![
            Span::styled(padding, style),
            Span::styled(prompt, style),
            Span::styled(content, self.theme.style.patch(content_style)),
            Span::styled(cursor, self.theme.style.patch(self.theme.filter_focused)),
        ])
        .render(area, buffer);
    }

    fn entry_style(&self, entry: &ExplorerEntry, selected: bool) -> Style {
        let kind = if entry.parent {
            self.theme.parent
        } else if entry.symlink {
            self.theme.symlink
        } else if entry.directory {
            self.theme.directory
        } else {
            self.theme.item
        };
        let style = self.theme.style.patch(kind);
        if selected {
            style.patch(self.theme.selected)
        } else {
            style
        }
    }

    fn entry_label(&self, entry: &ExplorerEntry, selected: bool, width: u16) -> String {
        if width == 0 {
            return String::new();
        }
        let padding = " ".repeat(usize::from(self.theme.left_padding.min(width)));
        let symbol = self.theme.selected_symbol.as_deref().unwrap_or("");
        let marker = if selected {
            symbol.to_owned()
        } else {
            " ".repeat(display_width(symbol))
        };
        let prefix = format!("{padding}{marker}");
        let remaining = usize::from(width).saturating_sub(display_width(&prefix));
        format!("{prefix}{}", truncate_end(&entry.display_name(), remaining))
    }

    fn ensure_selected_visible(&mut self) {
        let viewport = self.viewport_rows;
        let total = self.entries.len();
        if viewport == 0 || total == 0 {
            self.scroll = 0;
            return;
        }
        let padding = self
            .theme
            .scroll_padding
            .min(viewport.saturating_sub(1) / 2);
        if self.selected < self.scroll.saturating_add(padding) {
            self.scroll = self.selected.saturating_sub(padding);
        } else {
            let protected_bottom = self.scroll.saturating_add(viewport).saturating_sub(padding);
            if self.selected >= protected_bottom {
                self.scroll = self
                    .selected
                    .saturating_add(padding)
                    .saturating_add(1)
                    .saturating_sub(viewport);
            }
        }
        self.clamp_scroll();
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self
            .scroll
            .min(self.entries.len().saturating_sub(self.viewport_rows));
    }
}

/// Renderable view returned by [`Explorer::widget`].
pub struct ExplorerWidget<'explorer, 'drag> {
    explorer: &'explorer mut Explorer,
    drags: &'drag mut DragSurface,
}

impl Widget for ExplorerWidget<'_, '_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        self.explorer.render(area, buffer, self.drags);
    }
}

fn canonical_directory(path: &Path) -> io::Result<PathBuf> {
    let path = fs::canonicalize(path)?;
    if fs::metadata(&path)?.is_dir() {
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a directory", path.display()),
        ))
    }
}

fn read_entries(cwd: &Path, show_hidden: bool) -> io::Result<Vec<ExplorerEntry>> {
    let mut entries = Vec::new();
    if let Some(parent) = cwd.parent() {
        entries.push(ExplorerEntry {
            path: parent.to_path_buf(),
            name: "..".to_owned(),
            directory: true,
            symlink: false,
            parent: true,
        });
    }

    let mut children = fs::read_dir(cwd)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let raw_name = entry.file_name().to_string_lossy().into_owned();
            if !show_hidden && raw_name.starts_with('.') {
                return None;
            }
            let path = entry.path();
            let symlink = fs::symlink_metadata(&path)
                .ok()
                .is_some_and(|metadata| metadata.file_type().is_symlink());
            let directory = fs::metadata(&path)
                .ok()
                .is_some_and(|metadata| metadata.is_dir());
            Some(ExplorerEntry {
                path,
                name: sanitize_label(&raw_name),
                directory,
                symlink,
                parent: false,
            })
        })
        .collect::<Vec<_>>();
    children.sort_by(|left, right| {
        right
            .directory
            .cmp(&left.directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.path.cmp(&right.path))
    });
    entries.extend(children);
    Ok(entries)
}

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|character| {
            if character.is_control() {
                '�'
            } else {
                character
            }
        })
        .collect()
}

fn truncate_end(text: &str, width: usize) -> String {
    let mut output = String::new();
    let mut used = 0usize;
    for character in text.chars() {
        let character_width = character.width().unwrap_or(0);
        if used.saturating_add(character_width) > width {
            break;
        }
        output.push(character);
        used = used.saturating_add(character_width);
    }
    output
}

fn truncate_start(text: &str, width: u16) -> String {
    let width = usize::from(width);
    if display_width(text) <= width {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".to_owned();
    }
    let mut suffix = Vec::new();
    let mut used = 1usize;
    for character in text.chars().rev() {
        let character_width = character.width().unwrap_or(0);
        if used.saturating_add(character_width) > width {
            break;
        }
        suffix.push(character);
        used = used.saturating_add(character_width);
    }
    suffix.reverse();
    format!("…{}", suffix.into_iter().collect::<String>())
}

fn display_width(text: &str) -> usize {
    text.chars()
        .map(|character| character.width().unwrap_or(0))
        .sum()
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::Widget as _;

    use super::*;

    fn entry_names(explorer: &Explorer) -> Vec<String> {
        explorer
            .entries()
            .iter()
            .map(ExplorerEntry::display_name)
            .collect()
    }

    #[test]
    fn flat_listing_hides_dotfiles_and_sorts_directories_first() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("z-dir")).unwrap();
        fs::create_dir(temp.path().join("a-dir")).unwrap();
        fs::write(temp.path().join("a.txt"), "a").unwrap();
        fs::write(temp.path().join(".secret"), "secret").unwrap();

        let mut explorer = Explorer::new(temp.path()).unwrap();
        assert_eq!(entry_names(&explorer), ["../", "a-dir/", "z-dir/", "a.txt"]);

        explorer.set_show_hidden(true).unwrap();
        assert_eq!(
            entry_names(&explorer),
            ["../", "a-dir/", "z-dir/", ".secret", "a.txt"]
        );
    }

    #[test]
    fn open_and_parent_navigation_return_observable_events() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("folder");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("note.md"), "hello").unwrap();

        let mut explorer = Explorer::new(temp.path()).unwrap();
        let folder = fs::canonicalize(folder).unwrap();
        assert!(explorer.select_path(&folder));
        assert_eq!(
            explorer.handle(ExplorerInput::Open).unwrap(),
            ExplorerEvent::DirectoryChanged(folder.clone())
        );
        assert_eq!(explorer.cwd(), folder);
        let note = folder.join("note.md");
        assert!(explorer.select_path(&note));
        assert_eq!(
            explorer.handle(ExplorerInput::Open).unwrap(),
            ExplorerEvent::FileActivated(note)
        );

        let previous = fs::canonicalize(temp.path()).unwrap();
        assert_eq!(
            explorer.handle(ExplorerInput::Parent).unwrap(),
            ExplorerEvent::DirectoryChanged(previous.clone())
        );
        assert_eq!(explorer.cwd(), previous);
        assert_eq!(explorer.selected().unwrap().path(), folder);
    }

    #[test]
    fn selection_wraps_and_pages_by_the_rendered_viewport() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..10 {
            fs::write(temp.path().join(format!("{index}.txt")), "x").unwrap();
        }
        let mut explorer = Explorer::new(temp.path()).unwrap();
        let mut drags = DragSurface::disabled();
        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);
        explorer.widget(&mut drags).render(area, &mut buffer);

        assert_eq!(explorer.selected_index(), 0);
        explorer.handle(ExplorerInput::Up).unwrap();
        assert_eq!(explorer.selected_index(), explorer.entries().len() - 1);
        explorer.handle(ExplorerInput::Down).unwrap();
        assert_eq!(explorer.selected_index(), 0);
        explorer.handle(ExplorerInput::PageDown).unwrap();
        assert_eq!(explorer.selected_index(), 3);
    }

    #[test]
    fn borderless_render_is_full_row_selected_and_registers_every_path() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("folder")).unwrap();
        fs::write(temp.path().join("note.md"), "hello").unwrap();
        let theme = ExplorerTheme {
            selected: Style::new().bg(Color::Red),
            ..ExplorerTheme::default()
        };
        let mut explorer = Explorer::new(temp.path()).unwrap().with_theme(theme);
        let mut drags = DragSurface::disabled();
        let area = Rect::new(0, 0, 24, 5);
        let mut buffer = Buffer::empty(area);
        drags.begin_frame();

        explorer.widget(&mut drags).render(area, &mut buffer);

        assert_ne!(buffer[(0, 0)].symbol(), "┌");
        assert_eq!(buffer[(23, 2)].bg, Color::Red);
        assert_eq!(buffer[(0, 2)].symbol(), " ");
        assert_eq!(buffer[(1, 2)].symbol(), " ");
        assert_eq!(drags.regions().len(), 4);
        assert_eq!(drags.regions()[0].area, Rect::new(0, 1, 24, 1));
        assert_eq!(
            drags.regions()[0].path,
            fs::canonicalize(temp.path()).unwrap()
        );
        assert!(
            drags
                .regions()
                .iter()
                .all(|region| region.path.is_absolute())
        );
    }

    #[test]
    fn overflow_uses_shared_scrollbar_and_hit_testing_excludes_it() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..8 {
            fs::write(temp.path().join(format!("{index}.txt")), "x").unwrap();
        }
        let mut explorer = Explorer::new(temp.path()).unwrap();
        let mut drags = DragSurface::disabled();
        let area = Rect::new(2, 3, 12, 4);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 10));

        explorer.widget(&mut drags).render(area, &mut buffer);

        assert_eq!(explorer.list_area(), Rect::new(2, 5, 11, 2));
        assert_eq!(buffer[(13, 5)].symbol(), "┃");
        assert!(explorer.entry_at(Position::new(2, 5)).is_some());
        assert!(explorer.entry_at(Position::new(13, 5)).is_none());
        assert!(explorer.select_at(Position::new(2, 6)));
        assert_eq!(explorer.selected_index(), 1);
    }

    #[test]
    fn filter_is_case_insensitive_keeps_parent_and_selects_first_match() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("Source")).unwrap();
        fs::write(temp.path().join("README.md"), "hello").unwrap();
        fs::write(temp.path().join("notes.txt"), "hello").unwrap();

        let mut explorer = Explorer::new(temp.path()).unwrap();
        assert_eq!(
            explorer.handle(ExplorerInput::FocusFilter).unwrap(),
            ExplorerEvent::FilterFocusChanged
        );
        for character in "Me.MD".chars() {
            assert_eq!(
                explorer
                    .handle(ExplorerInput::FilterCharacter(character))
                    .unwrap(),
                ExplorerEvent::FilterChanged
            );
        }

        assert_eq!(entry_names(&explorer), ["../", "README.md"]);
        assert_eq!(explorer.match_count(), 1);
        assert_eq!(explorer.total_count(), 3);
        assert_eq!(explorer.selected().unwrap().display_name(), "README.md");
        assert!(explorer.filter_focused());
    }

    #[test]
    fn filter_backspace_is_unicode_safe_and_clear_restores_entries() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("界.txt"), "hello").unwrap();
        fs::write(temp.path().join("other.txt"), "hello").unwrap();
        let mut explorer = Explorer::new(temp.path()).unwrap();

        explorer.set_filter("界");
        assert_eq!(entry_names(&explorer), ["../", "界.txt"]);
        assert_eq!(
            explorer.handle(ExplorerInput::FilterBackspace).unwrap(),
            ExplorerEvent::FilterChanged
        );
        assert_eq!(explorer.filter(), "");
        assert_eq!(explorer.match_count(), 2);
        assert_eq!(
            explorer.handle(ExplorerInput::FilterBackspace).unwrap(),
            ExplorerEvent::None
        );
        assert_eq!(
            explorer
                .handle(ExplorerInput::FilterCharacter('\n'))
                .unwrap(),
            ExplorerEvent::None
        );
    }

    #[test]
    fn labels_are_control_safe_and_wide_character_aware() {
        assert_eq!(sanitize_label("bad\u{1b}name\n"), "bad�name�");
        assert_eq!(truncate_end("ab界c", 4), "ab界");
        assert_eq!(truncate_end("ab界c", 3), "ab");
        assert_eq!(truncate_start("/long/界/path", 7), "…/path");
    }
}
