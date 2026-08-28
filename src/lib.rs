//! Reusable Ratatui components for terminal-native Unpeel Apps.
//!
//! Components cover a borderless filesystem explorer, dark/light theming,
//! popup menus, adjacent-agent handoff, Host-local path dragging, consistent
//! proportional scrollbars, and an optional Markdown editing surface built
//! on `tui-textarea-2`.

#![forbid(unsafe_code)]

mod agent;
mod click;
mod drag;
mod explorer;
mod keyboard;
#[cfg(feature = "markdown-text-area")]
mod markdown_text_area;
mod menu;
mod scrollbar;
mod theme;
mod widgets;

pub use agent::{
    AgentBridge, AgentError, clipboard_sequence, is_hosted, path_reference, send_to_agent,
};
pub use click::{DEFAULT_DOUBLE_CLICK_INTERVAL, DoubleClickTracker};
pub use drag::{DRAG_MAP_FILENAME, DragRegion, DragSurface};
pub use explorer::{
    Explorer, ExplorerEntry, ExplorerEvent, ExplorerInput, ExplorerTheme, ExplorerWidget,
};
pub use keyboard::KeyboardEnhancementGuard;
#[cfg(feature = "markdown-text-area")]
pub use markdown_text_area::{MarkdownTextArea, MarkdownTextAreaStyle};
pub use menu::{MenuItem, MenuItemTone, MenuTheme, PopupMenu};
pub use scrollbar::VerticalScrollbar;
pub use theme::{ColorScheme, KitTheme, SELECTABLE_LEFT_PADDING};
pub use widgets::{DragSource, DraggablePath};
