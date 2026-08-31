//! Opinionated Ratatui components for ordinary terminal Apps.
//!
//! The component APIs require no Unpeel runtime or bridge setup and work in any
//! terminal. The default-on `ui-bridge` feature additionally lets a hosted App
//! publish semantic state to SwiftUI or web; disabling default features removes
//! all socket, authentication, persistence-envelope, and UI protocol code.
//! Markdown editing and terminal Media decoding/rendering are separate opt-in
//! features, so ordinary Apps pay only for the components they use.

#![deny(unsafe_code)]

mod agent;
mod click;
mod components;
mod context;
mod drag;
mod drop_target;
mod editor;
mod explorer;
mod host;
mod input;
mod keyboard;
#[cfg(feature = "markdown-text-area")]
mod markdown_interaction;
#[cfg(feature = "markdown-text-area")]
mod markdown_text_area;
#[cfg(any(feature = "media", feature = "ui-bridge"))]
mod media;
mod menu;
mod navigator;
mod path;
#[cfg(feature = "ui-bridge")]
#[allow(unsafe_code)]
mod process_security;
mod scrollbar;
mod selectable;
mod theme;
#[cfg(feature = "ui-bridge")]
mod ui;
#[cfg(feature = "ui-bridge")]
mod ui_auth;
#[cfg(feature = "ui-bridge")]
mod ui_bridge;
#[cfg(feature = "ui-bridge")]
mod ui_state;
mod widgets;

pub use agent::{
    AgentBridge, AgentError, AgentProjectContext, clipboard_sequence, is_hosted, path_reference,
    send_reference_to_agent, send_to_agent,
};
pub use click::{DEFAULT_DOUBLE_CLICK_INTERVAL, DoubleClickTracker};
pub use components::{
    ComponentValidationError, INPUT_COMPONENT_CAPABILITY, Input, LIST_COMPONENT_CAPABILITY,
    LIST_ITEM_ACTIVATE_CAPABILITY, LIST_ITEM_COMPONENT_CAPABILITY, LIST_ITEM_METADATA_CAPABILITY,
    List, ListItem, ListItemSlot, PAGE_BACK_CAPABILITY, PAGE_COMPONENT_CAPABILITY, Page,
    PageBodySlot, PageHeaderSlot, PageTheme, PageWidget, TOGGLE_COMPONENT_CAPABILITY, Toggle,
};
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
pub use markdown_interaction::{
    MARKDOWN_INSERT_ITEMS, MarkdownBlockKind, MarkdownEditorInteraction, MarkdownInsertItem,
    MarkdownInteractionOutcome, visible_markdown_insert_items,
};
#[cfg(feature = "markdown-text-area")]
pub use markdown_text_area::{
    MarkdownEditor, MarkdownEditorStyle, MarkdownTextArea, MarkdownTextAreaStyle,
};
#[cfg(all(feature = "markdown-text-area", feature = "ui-bridge"))]
pub use markdown_text_area::{MarkdownEditorConfig, MarkdownEditorEvent, MarkdownEditorEventError};
#[cfg(any(feature = "media", feature = "ui-bridge"))]
pub use media::{
    MAX_INLINE_MEDIA_BYTES, MEDIA_COMPONENT_CAPABILITY, MediaCellSize, MediaFit, MediaPixelSize,
    MediaPointSize, MediaSource, MediaSpec, MediaSpecError,
};
#[cfg(feature = "media")]
pub use media::{Media, MediaError, MediaPicker, MediaProtocolType};
pub use menu::{MenuItem, MenuItemTone, MenuTheme, PopupMenu};
pub use navigator::Navigator;
pub use path::display_path_from_root;
pub use scrollbar::VerticalScrollbar;
pub use selectable::SelectableRow;
pub use theme::{
    APP_ACCENT_ENV, ColorScheme, KitTheme, SELECTABLE_LEFT_PADDING, ThemeMonitor, hosted_accent,
    hosted_accent_for_scheme,
};
#[cfg(feature = "ui-bridge")]
pub use ui::{
    ActionId, AppInstanceId, AppMetadata, ClientId, EventId, MAX_SAFE_UI_INTEGER,
    MAX_UI_FRAME_BYTES, MarkdownEditorActions, MarkdownEditorSpec, MarkdownPresentation, NodeId,
    ParticipantId, RendererId, TextEdit, TextPosition, TextRange, TextSelection,
    UI_DELTA_CAPABILITY, UI_MARKDOWN_EDITOR_CAPABILITY, UI_PROTOCOL_MAX_VERSION,
    UI_PROTOCOL_MIN_VERSION, UI_PROTOCOL_NAME, UI_PROTOCOL_VERSION, UI_SOCKET_ENV, UI_TOKEN_ENV,
    UiAck, UiAckStatus, UiAction, UiAttach, UiAttached, UiComponent, UiDelta, UiDeltaOperation,
    UiErrorMessage, UiEvent, UiEventKind, UiEventValue, UiGrant, UiLifecycle, UiMessage, UiNode,
    UiParticipant, UiParticipantKind, UiPresence, UiPresenceMember, UiProtocolError,
    UiRendererMetadata, UiRendererState, UiRequestSnapshot, UiSnapshot, UiValidationError, ViewId,
    decode_ui_frame, encode_ui_frame, markdown_delta_operations, negotiate_ui_protocol_version,
    read_ui_message, write_ui_message,
};
#[cfg(feature = "ui-bridge")]
pub use ui_auth::{
    UI_PARTICIPANT_TOKEN_PREFIX, UI_PARTICIPANT_TOKEN_VERSION, UiParticipantTokenClaims,
    UiParticipantTokenError, UiParticipantTokenIssuer, UiParticipantTokenVerifier,
};
#[cfg(feature = "ui-bridge")]
pub use ui_bridge::{UiBridge, UiBridgeError, UiBridgeEvent, UiEventOutcome};
#[cfg(feature = "ui-bridge")]
pub use ui_state::{
    UI_STATE_FILENAME, UI_STATE_FORMAT, UI_STATE_FORMAT_VERSION, UiSavedState, UiStateError,
    UiStateStore,
};
pub use widgets::{DragSource, DraggablePath};
