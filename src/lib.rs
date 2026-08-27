//! Reusable Ratatui components for terminal-native Unpeel Apps.
//!
//! Components cover a borderless filesystem explorer, Host-local path
//! dragging, consistent proportional scrollbars, and an optional Markdown
//! editing surface built on `tui-textarea-2`.

#![forbid(unsafe_code)]

mod drag;
mod explorer;
#[cfg(feature = "markdown-text-area")]
mod markdown_text_area;
mod scrollbar;
mod widgets;

pub use drag::{DRAG_MAP_FILENAME, DragRegion, DragSurface};
pub use explorer::{
    Explorer, ExplorerEntry, ExplorerEvent, ExplorerInput, ExplorerTheme, ExplorerWidget,
};
#[cfg(feature = "markdown-text-area")]
pub use markdown_text_area::{MarkdownTextArea, MarkdownTextAreaStyle};
pub use scrollbar::VerticalScrollbar;
pub use widgets::{DragSource, DraggablePath};
