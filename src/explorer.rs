use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthChar;

use crate::{DragSurface, VerticalScrollbar};

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
}

/// Borderless visual styling for [`Explorer`].
///
/// There is deliberately no `Block` or border field. Apps can place the
/// component inside their own layout without inheriting IDE-like chrome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorerTheme {
    pub style: Style,
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
        Self {
            style: Style::new(),
            path: Style::new().add_modifier(Modifier::BOLD),
            item: Style::new().fg(Color::White),
            directory: Style::new().fg(Color::LightBlue),
            symlink: Style::new().fg(Color::Cyan),
            parent: Style::new().fg(Color::LightBlue),
            selected: Style::new().bg(Color::DarkGray),
            empty: Style::new().fg(Color::DarkGray),
            scrollbar_track: Style::new().fg(Color::DarkGray),
            scrollbar_thumb: Style::new().fg(Color::Gray),
            selected_symbol: None,
            left_padding: 1,
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
    entries: Vec<ExplorerEntry>,
    selected: usize,
    show_hidden: bool,
    show_path: bool,
    theme: ExplorerTheme,
    scroll: usize,
    viewport_rows: usize,
    area: Rect,
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
        let entries = read_entries(&cwd, false)?;
        let selected = preferred
            .as_deref()
            .and_then(|path| entries.iter().position(|entry| entry.path == path))
            .unwrap_or(0)
            .min(entries.len().saturating_sub(1));
        Ok(Self {
            cwd,
            entries,
            selected,
            show_hidden: false,
            show_path: true,
            theme: ExplorerTheme::default(),
            scroll: 0,
            viewport_rows: 12,
            area: Rect::default(),
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

    /// Entries in display order, including `../` when a parent exists.
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

    /// Most recently rendered outer area.
    #[must_use]
    pub const fn area(&self) -> Rect {
        self.area
    }

    /// Most recently rendered item viewport, excluding header and scrollbar.
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
        let entries = read_entries(&cwd, self.show_hidden)?;
        self.cwd = cwd;
        self.entries = entries;
        self.selected = 0;
        self.scroll = 0;
        Ok(())
    }

    /// Refreshes the current directory while preserving the selected path.
    pub fn refresh(&mut self) -> io::Result<()> {
        let preferred = self.selected().map(|entry| entry.path.clone());
        let previous = self.selected;
        let entries = read_entries(&self.cwd, self.show_hidden)?;
        self.entries = entries;
        self.selected = preferred
            .as_deref()
            .and_then(|path| self.entries.iter().position(|entry| entry.path == path))
            .unwrap_or(previous)
            .min(self.entries.len().saturating_sub(1));
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
        let entries = read_entries(&self.cwd, show)?;
        self.entries = entries;
        self.show_hidden = show;
        self.selected = preferred
            .as_deref()
            .and_then(|path| self.entries.iter().position(|entry| entry.path == path))
            .unwrap_or(previous)
            .min(self.entries.len().saturating_sub(1));
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
        if entry.directory {
            self.change_directory(entry.path)
        } else {
            Ok(ExplorerEvent::FileActivated(entry.path))
        }
    }

    fn change_directory(&mut self, path: PathBuf) -> io::Result<ExplorerEvent> {
        let previous = self.cwd.clone();
        let cwd = canonical_directory(&path)?;
        let entries = read_entries(&cwd, self.show_hidden)?;
        let selected = entries
            .iter()
            .position(|entry| entry.path == previous)
            .unwrap_or(0)
            .min(entries.len().saturating_sub(1));
        self.cwd = cwd;
        self.entries = entries;
        self.selected = selected;
        self.scroll = 0;
        Ok(ExplorerEvent::DirectoryChanged(self.cwd.clone()))
    }

    fn render(&mut self, area: Rect, buffer: &mut Buffer, drags: &mut DragSurface) {
        self.area = area;
        buffer.set_style(area, self.theme.style);
        if area.is_empty() {
            self.list_area = Rect::default();
            self.viewport_rows = 0;
            self.scroll = 0;
            return;
        }

        let header_height = u16::from(self.show_path && area.height > 0);
        if header_height == 1 {
            let header = Rect::new(area.x, area.y, area.width, 1);
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

        let list_y = area.y.saturating_add(header_height);
        let list_height = area.height.saturating_sub(header_height);
        let overflow = list_height > 0 && self.entries.len() > usize::from(list_height);
        let list_width = area.width.saturating_sub(u16::from(overflow));
        self.list_area = Rect::new(area.x, list_y, list_width, list_height);
        self.viewport_rows = usize::from(list_height);
        self.ensure_selected_visible();

        if self.entries.is_empty() && !self.list_area.is_empty() {
            let label = format!(
                "{}empty folder",
                " ".repeat(usize::from(
                    self.theme.left_padding.min(self.list_area.width)
                )),
            );
            Line::styled(label, self.theme.style.patch(self.theme.empty))
                .render(self.list_area, buffer);
        }

        let visible = usize::from(list_height);
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
        assert_eq!(explorer.selected_index(), 4);
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
        assert_eq!(buffer[(23, 1)].bg, Color::Red);
        assert_eq!(drags.regions().len(), 4);
        assert_eq!(drags.regions()[0].area, Rect::new(0, 0, 24, 1));
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

        assert_eq!(explorer.list_area(), Rect::new(2, 4, 11, 3));
        assert_eq!(buffer[(13, 4)].symbol(), "┃");
        assert!(explorer.entry_at(Position::new(2, 4)).is_some());
        assert!(explorer.entry_at(Position::new(13, 4)).is_none());
        assert!(explorer.select_at(Position::new(2, 5)));
        assert_eq!(explorer.selected_index(), 1);
    }

    #[test]
    fn labels_are_control_safe_and_wide_character_aware() {
        assert_eq!(sanitize_label("bad\u{1b}name\n"), "bad�name�");
        assert_eq!(truncate_end("ab界c", 4), "ab界");
        assert_eq!(truncate_end("ab界c", 3), "ab");
        assert_eq!(truncate_start("/long/界/path", 7), "…/path");
    }
}
