//! Opinionated Ratatui-first components for terminal-powered Unpeel Apps.
//!
//! Apps render normally through Ratatui and can optionally publish the same
//! component state to SwiftUI/AppKit or web wrappers through `unpeel.ui/1`.
//! Components cover a borderless filesystem explorer and single-line input,
//! dark/light theming, popup menus, adjacent-agent handoff, Host-local path
//! dragging, typed workspace/project/worktree/user detection,
//! project-relative path labels, preferred-editor opening, consistent
//! proportional scrollbars, and an optional Markdown editor built
//! on `tui-textarea-2`.

#![deny(unsafe_code)]

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
#[allow(unsafe_code)]
mod process_security;
mod scrollbar;
mod theme;
mod ui;
mod ui_auth;
mod ui_bridge;
mod ui_state;
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
pub use markdown_text_area::{
    MarkdownEditor, MarkdownEditorConfig, MarkdownEditorEvent, MarkdownEditorEventError,
    MarkdownEditorStyle, MarkdownTextArea, MarkdownTextAreaStyle,
};
pub use menu::{MenuItem, MenuItemTone, MenuTheme, PopupMenu};
pub use navigator::Navigator;
pub use path::display_path_from_root;
pub use scrollbar::VerticalScrollbar;
pub use theme::{
    APP_ACCENT_ENV, ColorScheme, KitTheme, SELECTABLE_LEFT_PADDING, ThemeMonitor, hosted_accent,
    hosted_accent_for_scheme,
};
pub use ui::{
    ActionId, AppInstanceId, AppMetadata, ClientId, EventId, MAX_SAFE_UI_INTEGER,
    MAX_UI_FRAME_BYTES, MarkdownEditorActions, MarkdownEditorSpec, MarkdownPresentation, NodeId,
    ParticipantId, RendererId, TextEdit, TextPosition, TextRange, TextSelection,
    UI_DELTA_CAPABILITY, UI_PROTOCOL_MAX_VERSION, UI_PROTOCOL_MIN_VERSION, UI_PROTOCOL_NAME,
    UI_PROTOCOL_VERSION, UI_SOCKET_ENV, UI_TOKEN_ENV, UiAck, UiAckStatus, UiAction, UiAttach,
    UiAttached, UiComponent, UiDelta, UiDeltaOperation, UiErrorMessage, UiEvent, UiEventKind,
    UiEventValue, UiGrant, UiLifecycle, UiMessage, UiNode, UiParticipant, UiParticipantKind,
    UiPresence, UiPresenceMember, UiProtocolError, UiRendererMetadata, UiRendererState,
    UiRequestSnapshot, UiSnapshot, UiValidationError, ViewId, decode_ui_frame, encode_ui_frame,
    negotiate_ui_protocol_version, read_ui_message, write_ui_message,
};
pub use ui_auth::{
    UI_PARTICIPANT_TOKEN_PREFIX, UI_PARTICIPANT_TOKEN_VERSION, UiParticipantTokenClaims,
    UiParticipantTokenError, UiParticipantTokenIssuer, UiParticipantTokenVerifier,
};
pub use ui_bridge::{UiBridge, UiBridgeError, UiBridgeEvent, UiEventOutcome};
pub use ui_state::{
    UI_STATE_FILENAME, UI_STATE_FORMAT, UI_STATE_FORMAT_VERSION, UiSavedState, UiStateError,
    UiStateStore,
};
pub use widgets::{DragSource, DraggablePath};
