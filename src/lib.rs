//! Reusable Ratatui components for terminal-native Unpeel Apps.
//!
//! Components cover a borderless filesystem explorer and single-line input,
//! dark/light theming, popup menus, adjacent-agent handoff, Host-local path
//! dragging, typed workspace/project/worktree/user detection,
//! project-relative path labels, preferred-editor opening, consistent
//! proportional scrollbars, and an optional Markdown editing surface built
//! on `tui-textarea-2`.

#![forbid(unsafe_code)]

mod agent;
mod click;
mod context;
mod drag;
mod drop_target;
mod editor;
mod explorer;
mod host;
mod input;
mod keyboard;
#[cfg(feature = "markdown-text-area")]
mod markdown_text_area;
mod menu;
mod navigator;
mod path;
mod scrollbar;
mod theme;
mod widgets;

pub use agent::{
    AgentBridge, AgentError, AgentProjectContext, clipboard_sequence, is_hosted, path_reference,
    send_reference_to_agent, send_to_agent,
};
pub use click::{DEFAULT_DOUBLE_CLICK_INTERVAL, DoubleClickTracker};
pub use context::{
    AppContext, AppMode, ProjectContext, UnpeelUser, WorkspaceContext, WorktreeContext,
};
pub use drag::{DRAG_MAP_FILENAME, DragRegion, DragSurface};
pub use drop_target::{
    DROP_TARGET_EVENT_FILENAME, DROP_TARGET_MAP_FILENAME, DropTargetEvent, DropTargetRegion,
    DropTargetSurface,
};
pub use editor::{EditorBridge, EditorError, open_in_editor};
pub use explorer::{
    Explorer, ExplorerEntry, ExplorerEvent, ExplorerInput, ExplorerTheme, ExplorerWidget,
};
pub use host::AppReporter;
pub use input::{InputField, InputFieldAction, InputFieldTheme, InputFieldWidget};
pub use keyboard::KeyboardEnhancementGuard;
#[cfg(feature = "markdown-text-area")]
pub use markdown_text_area::{MarkdownTextArea, MarkdownTextAreaStyle};
pub use menu::{MenuItem, MenuItemTone, MenuTheme, PopupMenu};
pub use navigator::Navigator;
pub use path::display_path_from_root;
pub use scrollbar::VerticalScrollbar;
pub use theme::{
    APP_ACCENT_ENV, ColorScheme, KitTheme, SELECTABLE_LEFT_PADDING, ThemeMonitor, hosted_accent,
    hosted_accent_for_scheme,
};
pub use widgets::{DragSource, DraggablePath};
