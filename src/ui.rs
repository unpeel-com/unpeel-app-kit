//! Durable App Kit UI values shared by terminal, native, and web renderers.
//!
//! The terminal-backed Rust process owns application state. A trusted local
//! workspace broker attaches native or remote renderers to that process,
//! stamps authenticated participant identity onto their events, and can
//! reconnect without restarting the App.
//!
//! Fields added to a recognized message, component, or value are ignored for
//! forward compatibility. Message, event, and event-value discriminators stay
//! closed. Renderer packages treat an unknown component root as a signal to
//! expose the pane's complete TUI without closing the attachment; the
//! authoritative Rust App constructs only the known [`UiComponent`] variants.

use std::fmt;
use std::io::{self, BufRead, Read, Write};

use serde::{Deserialize, Serialize};

use crate::components::{ListItem, Page, PageBodySlot};
use crate::content::{ContentLine, ContentSelection};
use crate::media::{MediaPixelSize, MediaSource, MediaSpec};
use crate::semantic_menu::SemanticMenu;
use crate::surface::{CanvasPage, SurfaceReference, SurfaceSpec};
use crate::tree::{Tree, TreeChildState, TreeItem};

/// Stable protocol name carried by every independently replayable frame.
pub const UI_PROTOCOL_NAME: &str = "unpeel.ui";
/// Oldest App Kit component protocol version implemented by this build.
pub const UI_PROTOCOL_MIN_VERSION: u32 = 1;
/// Newest App Kit component protocol version implemented by this build.
pub const UI_PROTOCOL_MAX_VERSION: u32 = 1;
/// Current App Kit component protocol version used by new messages.
///
/// Keep this alias for callers that construct messages directly. Attachments
/// negotiate between [`UI_PROTOCOL_MIN_VERSION`] and
/// [`UI_PROTOCOL_MAX_VERSION`] before exchanging versioned messages.
pub const UI_PROTOCOL_VERSION: u32 = UI_PROTOCOL_MAX_VERSION;
/// Stable Unix socket path chosen by the workspace/session owner.
///
/// The terminal App binds this path. The existing native or headless Host
/// connects to it, so its lifetime follows the terminal App rather
/// than a replaceable renderer.
pub const UI_SOCKET_ENV: &str = "UNPEEL_UI_SOCKET";
/// Per-App-session participant-token signing key retained by the Host and App.
/// Renderers receive only route-bound credentials derived from this key.
pub const UI_TOKEN_ENV: &str = "UNPEEL_UI_TOKEN";
/// Renderer capability required before the App sends revision deltas.
pub const UI_DELTA_CAPABILITY: &str = "serverDelta";
/// Renderer capability for the v1 Markdown editor component.
pub const UI_MARKDOWN_EDITOR_CAPABILITY: &str = "markdownEditor";
/// Largest individual JSON payload accepted by the protocol.
pub const MAX_UI_FRAME_BYTES: usize = 16 * 1024 * 1024;
/// Largest integer represented exactly by Swift and JavaScript renderers.
pub const MAX_SAFE_UI_INTEGER: u64 = 9_007_199_254_740_991;

macro_rules! string_id {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

string_id!(
    /// Identity of the currently running terminal App process.
    AppInstanceId
);
string_id!(
    /// Stable opaque identity supplied by the authenticated workspace broker.
    ParticipantId
);
string_id!(
    /// Stable identity for one participant device or browser installation.
    ClientId
);
string_id!(
    /// Identity for one replaceable SwiftUI or web renderer instance.
    RendererId
);
string_id!(
    /// Logical App view, such as `main` or `document/readme`.
    ViewId
);
string_id!(
    /// Client-generated idempotency key for one renderer event.
    EventId
);
string_id!(
    /// Stable identity for one component.
    NodeId
);
string_id!(
    /// Stable action declared by an interactive component.
    ActionId
);
string_id!(
    /// Broker-attested access grant. Apps may define additional grant names.
    UiGrant
);

impl UiGrant {
    pub const VIEW: &'static str = "view";
    pub const INTERACT: &'static str = "interact";
    pub const EDIT: &'static str = "edit";
    pub const COMMAND: &'static str = "command";
    pub const ADMIN: &'static str = "admin";
    pub const ALL: &'static str = "*";
}

/// Human-facing identity advertised by an App process during attachment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AppMetadata {
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: None,
        }
    }

    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Host-attested participant category. Agents use the same presence, grants,
/// events, and revision path as people rather than a privileged side channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UiParticipantKind {
    #[default]
    Human,
    Agent,
    Service,
}

/// Authenticated workspace participant attached by the trusted Host.
///
/// `id` is opaque and Host-scoped. The optional label is presentation-only;
/// Apps must not interpret either field as an email address or account claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiParticipant {
    pub id: ParticipantId,
    #[serde(default)]
    pub kind: UiParticipantKind,
    /// Calling Session when the participant is an agent or App worker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grants: Vec<UiGrant>,
}

impl UiParticipant {
    #[must_use]
    pub fn new(id: impl Into<ParticipantId>) -> Self {
        Self {
            id: id.into(),
            kind: UiParticipantKind::Human,
            source_session_id: None,
            display_name: None,
            color: None,
            grants: Vec::new(),
        }
    }

    #[must_use]
    pub const fn kind(mut self, kind: UiParticipantKind) -> Self {
        self.kind = kind;
        self
    }

    #[must_use]
    pub fn source_session_id(mut self, source_session_id: impl Into<String>) -> Self {
        self.source_session_id = Some(source_session_id.into());
        self
    }

    #[must_use]
    pub fn display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    #[must_use]
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    #[must_use]
    pub fn grants<I, S>(mut self, grants: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<UiGrant>,
    {
        self.grants = grants.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn allows(&self, grant: &str) -> bool {
        self.grants.iter().any(|candidate| {
            matches!(candidate.as_str(), UiGrant::ALL) || candidate.as_str() == grant
        })
    }
}

/// Description of one replaceable renderer process or browser view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiRendererMetadata {
    pub id: RendererId,
    /// Renderer family such as `swiftUI` or `web`.
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
}

impl UiRendererMetadata {
    #[must_use]
    pub fn new(id: impl Into<RendererId>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            capabilities: Vec::new(),
        }
    }

    #[must_use]
    pub fn capabilities<I, S>(mut self, capabilities: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.capabilities = capabilities.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn supports(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|candidate| candidate == capability)
    }
}

/// Renderer and terminal visibility reported by the Host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiRendererState {
    pub renderer_visible: bool,
    pub terminal_visible: bool,
}

impl UiRendererState {
    #[must_use]
    pub const fn terminal() -> Self {
        Self {
            renderer_visible: false,
            terminal_visible: true,
        }
    }

    #[must_use]
    pub const fn component() -> Self {
        Self {
            renderer_visible: true,
            terminal_visible: false,
        }
    }

    #[must_use]
    pub const fn hidden() -> Self {
        Self {
            renderer_visible: false,
            terminal_visible: false,
        }
    }
}

impl Default for UiRendererState {
    fn default() -> Self {
        Self::terminal()
    }
}

/// Initial authenticated attachment sent by the Host or a scoped local agent.
///
/// Participant identity and grants are not caller-selected fields. They are
/// recovered from `participantToken`, whose signature binds this exact client,
/// renderer, view, and App Session.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAttach {
    pub protocol: String,
    pub min_protocol_version: u32,
    pub max_protocol_version: u32,
    pub participant_token: String,
    pub client_id: ClientId,
    pub renderer: UiRendererMetadata,
    pub view_id: ViewId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_app_instance_id: Option<AppInstanceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_revision: Option<u64>,
    #[serde(default)]
    pub state: UiRendererState,
}

impl UiAttach {
    #[must_use]
    pub fn new(
        participant_token: impl Into<String>,
        client_id: impl Into<ClientId>,
        renderer: UiRendererMetadata,
        view_id: impl Into<ViewId>,
    ) -> Self {
        Self {
            protocol: UI_PROTOCOL_NAME.to_owned(),
            min_protocol_version: UI_PROTOCOL_MIN_VERSION,
            max_protocol_version: UI_PROTOCOL_MAX_VERSION,
            participant_token: participant_token.into(),
            client_id: client_id.into(),
            renderer,
            view_id: view_id.into(),
            expected_app_instance_id: None,
            last_seen_revision: None,
            state: UiRendererState::default(),
        }
    }

    #[must_use]
    pub fn resume(mut self, app_instance_id: impl Into<AppInstanceId>, revision: u64) -> Self {
        self.expected_app_instance_id = Some(app_instance_id.into());
        self.last_seen_revision = Some(revision);
        self
    }

    #[must_use]
    pub const fn state(mut self, state: UiRendererState) -> Self {
        self.state = state;
        self
    }

    /// Advertises the inclusive protocol range supported by the renderer.
    #[must_use]
    pub const fn protocol_versions(mut self, minimum: u32, maximum: u32) -> Self {
        self.min_protocol_version = minimum;
        self.max_protocol_version = maximum;
        self
    }
}

impl fmt::Debug for UiAttach {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UiAttach")
            .field("protocol", &self.protocol)
            .field("min_protocol_version", &self.min_protocol_version)
            .field("max_protocol_version", &self.max_protocol_version)
            .field("participant_token", &"[REDACTED]")
            .field("client_id", &self.client_id)
            .field("renderer", &self.renderer)
            .field("view_id", &self.view_id)
            .field("expected_app_instance_id", &self.expected_app_instance_id)
            .field("last_seen_revision", &self.last_seen_revision)
            .field("state", &self.state)
            .finish()
    }
}

/// Successful App acknowledgement of one renderer attachment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAttached {
    pub protocol: String,
    /// Version selected from the renderer and App ranges for this connection.
    pub protocol_version: u32,
    pub min_protocol_version: u32,
    pub max_protocol_version: u32,
    pub app: AppMetadata,
    pub app_instance_id: AppInstanceId,
    pub participant_id: ParticipantId,
    pub client_id: ClientId,
    pub renderer_id: RendererId,
    pub view_id: ViewId,
    pub resumed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<u64>,
}

/// Zero-based text location. Columns count UTF-16 code units.
///
/// Cocoa and JavaScript index text this way. Ratatui adapters validate scalar
/// boundaries and translate positions to character-wise cursors.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextPosition {
    pub line: u32,
    pub utf16_column: u32,
}

impl TextPosition {
    #[must_use]
    pub const fn new(line: u32, utf16_column: u32) -> Self {
        Self { line, utf16_column }
    }
}

/// Half-open text range whose start must not follow its end.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

impl TextRange {
    #[must_use]
    pub const fn new(start: TextPosition, end: TextPosition) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub fn ordered(first: TextPosition, second: TextPosition) -> Self {
        if first <= second {
            Self::new(first, second)
        } else {
            Self::new(second, first)
        }
    }
}

/// Selection orientation preserved for native focus and keyboard behavior.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TextSelection {
    pub anchor: TextPosition,
    pub head: TextPosition,
}

impl TextSelection {
    #[must_use]
    pub const fn caret(position: TextPosition) -> Self {
        Self {
            anchor: position,
            head: position,
        }
    }

    #[must_use]
    pub fn range(self) -> TextRange {
        TextRange::ordered(self.anchor, self.head)
    }
}

/// One renderer-originated replacement against a Markdown document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: TextRange,
    pub text: String,
}

impl TextEdit {
    #[must_use]
    pub fn new(range: TextRange, text: impl Into<String>) -> Self {
        Self {
            range,
            text: text.into(),
        }
    }
}

/// Markdown presentation requested by a renderer or App.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarkdownPresentation {
    #[default]
    Source,
    Preview,
    Split,
}

/// Closed Markdown menu entry points. Renderers request one of these; the App
/// owns whether it is valid at the current selection and publishes the Menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarkdownMenuTrigger {
    Slash,
    Palette,
}

impl MarkdownMenuTrigger {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Slash => "slash",
            Self::Palette => "palette",
        }
    }
}

impl std::str::FromStr for MarkdownMenuTrigger {
    type Err = UiProtocolError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "slash" => Ok(Self::Slash),
            "palette" => Ok(Self::Palette),
            _ => Err(UiProtocolError::InvalidMessage(format!(
                "unknown Markdown menu trigger {value:?}"
            ))),
        }
    }
}

impl MarkdownPresentation {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Preview => "preview",
            Self::Split => "split",
        }
    }
}

impl std::str::FromStr for MarkdownPresentation {
    type Err = UiProtocolError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "source" => Ok(Self::Source),
            "preview" => Ok(Self::Preview),
            "split" => Ok(Self::Split),
            _ => Err(UiProtocolError::InvalidMessage(format!(
                "unknown Markdown presentation {value:?}"
            ))),
        }
    }
}

/// Action identifiers supported by a Markdown renderer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownEditorActions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace_range: Option<ActionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_selection: Option<ActionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save: Option<ActionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undo: Option<ActionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redo: Option<ActionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_presentation: Option<ActionId>,
    /// Optional because plain Markdown editors need not expose App-owned
    /// slash/palette behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_menu: Option<ActionId>,
}

impl MarkdownEditorActions {
    pub const REPLACE_RANGE: &'static str = "replace-range";
    pub const SET_SELECTION: &'static str = "set-selection";
    pub const SAVE: &'static str = "save";
    pub const UNDO: &'static str = "undo";
    pub const REDO: &'static str = "redo";
    pub const SET_PRESENTATION: &'static str = "set-presentation";
    pub const OPEN_MENU: &'static str = "open-menu";

    #[must_use]
    pub fn editable() -> Self {
        Self {
            replace_range: Some(Self::REPLACE_RANGE.into()),
            set_selection: Some(Self::SET_SELECTION.into()),
            save: Some(Self::SAVE.into()),
            undo: Some(Self::UNDO.into()),
            redo: Some(Self::REDO.into()),
            set_presentation: Some(Self::SET_PRESENTATION.into()),
            open_menu: None,
        }
    }

    #[must_use]
    pub fn read_only() -> Self {
        Self {
            replace_range: None,
            set_selection: Some(Self::SET_SELECTION.into()),
            save: None,
            undo: None,
            redo: None,
            set_presentation: Some(Self::SET_PRESENTATION.into()),
            open_menu: None,
        }
    }
}

impl Default for MarkdownEditorActions {
    fn default() -> Self {
        Self::editable()
    }
}

/// Owned Markdown component state interpreted by Ratatui, Swift, or web.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownEditorSpec {
    pub text: String,
    pub selection: TextSelection,
    #[serde(default)]
    pub presentation: MarkdownPresentation,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub placeholder: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub actions: MarkdownEditorActions,
    /// Server-owned slash/palette menu, anchored to the renderer-local caret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insert_menu: Option<SemanticMenu>,
    /// Closed context-menu descriptor. Renderers open it at their own pointer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_menu: Option<SemanticMenu>,
}

impl MarkdownEditorSpec {
    #[must_use]
    pub fn new(text: impl Into<String>, selection: TextSelection) -> Self {
        Self {
            text: text.into(),
            selection,
            presentation: MarkdownPresentation::Source,
            read_only: false,
            dirty: false,
            placeholder: String::new(),
            title: None,
            actions: MarkdownEditorActions::editable(),
            insert_menu: None,
            context_menu: None,
        }
    }

    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self.actions = if read_only {
            MarkdownEditorActions::read_only()
        } else {
            MarkdownEditorActions::editable()
        };
        self
    }

    #[must_use]
    pub const fn presentation(mut self, presentation: MarkdownPresentation) -> Self {
        self.presentation = presentation;
        self
    }

    #[must_use]
    pub const fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    #[must_use]
    pub fn insert_menu(mut self, menu: SemanticMenu) -> Self {
        self.insert_menu = Some(menu);
        self
    }

    #[must_use]
    pub fn context_menu(mut self, menu: SemanticMenu) -> Self {
        self.context_menu = Some(menu);
        self
    }

    fn validate(&self, path: &str) -> Result<(), UiValidationError> {
        validate_position(&self.text, self.selection.anchor).map_err(|message| {
            UiValidationError::new(format!("{path}.selection.anchor"), message)
        })?;
        validate_position(&self.text, self.selection.head)
            .map_err(|message| UiValidationError::new(format!("{path}.selection.head"), message))?;
        validate_action_set(&self.actions, self.read_only, path)?;
        for (slot, menu) in [
            ("insertMenu", self.insert_menu.as_ref()),
            ("contextMenu", self.context_menu.as_ref()),
        ] {
            if let Some(menu) = menu {
                menu.validate().map_err(|error| {
                    UiValidationError::new(
                        format!("{path}.{slot}.{}", error.path.replacen("menu.", "", 1)),
                        error.message,
                    )
                })?;
            }
        }
        Ok(())
    }
}

/// Closed v1 component vocabulary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UiComponent {
    CanvasPage(CanvasPage),
    MarkdownEditor(MarkdownEditorSpec),
    Media(MediaSpec),
    Menu(SemanticMenu),
    Page(Page),
    Surface(SurfaceSpec),
    Tree(Tree),
}

impl UiComponent {
    /// Stable discriminated kind used on the wire.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::CanvasPage(_) => "canvasPage",
            Self::MarkdownEditor(_) => "markdownEditor",
            Self::Media(_) => "media",
            Self::Menu(_) => "menu",
            Self::Page(_) => "page",
            Self::Surface(_) => "surface",
            Self::Tree(_) => "tree",
        }
    }

    /// Capability an attached semantic renderer must advertise for this root.
    #[must_use]
    pub const fn required_capability(&self) -> &'static str {
        match self {
            Self::CanvasPage(_) => crate::CANVAS_PAGE_COMPONENT_CAPABILITY,
            Self::MarkdownEditor(_) => UI_MARKDOWN_EDITOR_CAPABILITY,
            Self::Media(_) => crate::MEDIA_COMPONENT_CAPABILITY,
            Self::Menu(_) => crate::MENU_COMPONENT_CAPABILITY,
            Self::Page(_) => crate::PAGE_COMPONENT_CAPABILITY,
            Self::Surface(_) => crate::SURFACE_COMPONENT_CAPABILITY,
            Self::Tree(_) => crate::TREE_COMPONENT_CAPABILITY,
        }
    }

    /// Capabilities needed to render this exact closed component tree.
    #[must_use]
    pub fn required_capabilities(&self) -> Vec<&'static str> {
        match self {
            Self::CanvasPage(page) => page.required_capabilities(),
            Self::MarkdownEditor(editor) => {
                let mut capabilities = vec![UI_MARKDOWN_EDITOR_CAPABILITY];
                if editor.insert_menu.is_some() || editor.context_menu.is_some() {
                    capabilities.extend([
                        crate::MENU_COMPONENT_CAPABILITY,
                        crate::MENU_ANCHOR_CAPABILITY,
                    ]);
                }
                capabilities
            }
            Self::Media(_) => vec![crate::MEDIA_COMPONENT_CAPABILITY],
            Self::Menu(menu) => menu.required_capabilities().to_vec(),
            Self::Page(page) => page.required_capabilities(),
            Self::Surface(_) => vec![crate::SURFACE_COMPONENT_CAPABILITY],
            Self::Tree(tree) => tree.required_capabilities(),
        }
    }
}

/// A keyed component.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiNode {
    pub id: NodeId,
    #[serde(flatten)]
    pub element: UiComponent,
}

impl UiNode {
    #[must_use]
    pub fn canvas_page(id: impl Into<NodeId>, page: CanvasPage) -> Self {
        Self {
            id: id.into(),
            element: UiComponent::CanvasPage(page),
        }
    }

    #[must_use]
    pub fn markdown_editor(id: impl Into<NodeId>, editor: MarkdownEditorSpec) -> Self {
        Self {
            id: id.into(),
            element: UiComponent::MarkdownEditor(editor),
        }
    }

    #[must_use]
    pub fn media(id: impl Into<NodeId>, media: MediaSpec) -> Self {
        Self {
            id: id.into(),
            element: UiComponent::Media(media),
        }
    }

    #[must_use]
    pub fn menu(id: impl Into<NodeId>, menu: SemanticMenu) -> Self {
        Self {
            id: id.into(),
            element: UiComponent::Menu(menu),
        }
    }

    #[must_use]
    pub fn page(id: impl Into<NodeId>, page: Page) -> Self {
        Self {
            id: id.into(),
            element: UiComponent::Page(page),
        }
    }

    #[must_use]
    pub fn surface(id: impl Into<NodeId>, surface: SurfaceSpec) -> Self {
        Self {
            id: id.into(),
            element: UiComponent::Surface(surface),
        }
    }

    #[must_use]
    pub fn tree(id: impl Into<NodeId>, tree: Tree) -> Self {
        Self {
            id: id.into(),
            element: UiComponent::Tree(tree),
        }
    }

    pub fn validate(&self) -> Result<(), UiValidationError> {
        validate_identifier(self.id.as_str(), "root.id")?;
        match &self.element {
            UiComponent::CanvasPage(page) => page.validate().map_err(|error| {
                UiValidationError::new(error.path.replacen("canvasPage", "root", 1), error.message)
            }),
            UiComponent::MarkdownEditor(editor) => editor.validate("root"),
            UiComponent::Media(media) => media.validate().map_err(|error| {
                UiValidationError::new(error.path.replacen("media", "root", 1), error.message)
            }),
            UiComponent::Menu(menu) => menu.validate().map_err(|error| {
                UiValidationError::new(error.path.replacen("menu", "root", 1), error.message)
            }),
            UiComponent::Page(page) => page.validate().map_err(|error| {
                UiValidationError::new(error.path.replacen("page", "root", 1), error.message)
            }),
            UiComponent::Surface(surface) => surface.validate().map_err(|error| {
                UiValidationError::new(error.path.replacen("surface", "root", 1), error.message)
            }),
            UiComponent::Tree(tree) => tree.validate().map_err(|error| {
                UiValidationError::new(error.path.replacen("tree", "root", 1), error.message)
            }),
        }
    }

    /// Capability required to render this node semantically.
    #[must_use]
    pub const fn required_capability(&self) -> &'static str {
        self.element.required_capability()
    }

    /// Capabilities needed to render this node and each constrained slot.
    #[must_use]
    pub fn required_capabilities(&self) -> Vec<&'static str> {
        self.element.required_capabilities()
    }
}

/// Builds component-specific operations between two Markdown projections.
///
/// Text is reduced to one contiguous UTF-16 range replacement and selection
/// is emitted independently, so terminal pointer selection and native/web
/// selection stay synchronized without replacing a potentially large root.
/// A root replacement is returned when the nodes are not the same Markdown
/// component.
#[must_use]
pub fn markdown_delta_operations(previous: &UiNode, next: &UiNode) -> Vec<UiDeltaOperation> {
    let (UiComponent::MarkdownEditor(previous_editor), UiComponent::MarkdownEditor(next_editor)) =
        (&previous.element, &next.element)
    else {
        return vec![UiDeltaOperation::ReplaceRoot { root: next.clone() }];
    };
    if previous.id != next.id {
        return vec![UiDeltaOperation::ReplaceRoot { root: next.clone() }];
    }

    let mut operations = Vec::new();
    if previous_editor.text != next_editor.text {
        operations.push(UiDeltaOperation::MarkdownReplaceRange {
            node_id: next.id.clone(),
            edit: contiguous_text_edit(&previous_editor.text, &next_editor.text),
        });
    }
    if previous_editor.selection != next_editor.selection {
        operations.push(UiDeltaOperation::MarkdownSetSelection {
            node_id: next.id.clone(),
            selection: next_editor.selection,
        });
    }
    if previous_editor.presentation != next_editor.presentation {
        operations.push(UiDeltaOperation::MarkdownSetPresentation {
            node_id: next.id.clone(),
            presentation: next_editor.presentation,
        });
    }
    if previous_editor.dirty != next_editor.dirty {
        operations.push(UiDeltaOperation::MarkdownSetDirty {
            node_id: next.id.clone(),
            dirty: next_editor.dirty,
        });
    }
    if previous_editor.read_only != next_editor.read_only {
        operations.push(UiDeltaOperation::MarkdownSetReadOnly {
            node_id: next.id.clone(),
            read_only: next_editor.read_only,
        });
    }
    if previous_editor.title != next_editor.title {
        operations.push(UiDeltaOperation::MarkdownSetTitle {
            node_id: next.id.clone(),
            title: next_editor.title.clone(),
        });
    }
    if previous_editor.placeholder != next_editor.placeholder {
        operations.push(UiDeltaOperation::MarkdownSetPlaceholder {
            node_id: next.id.clone(),
            placeholder: next_editor.placeholder.clone(),
        });
    }
    if previous_editor.actions != next_editor.actions {
        operations.push(UiDeltaOperation::MarkdownSetActions {
            node_id: next.id.clone(),
            actions: next_editor.actions.clone(),
        });
    }
    if previous_editor.insert_menu != next_editor.insert_menu
        || previous_editor.context_menu != next_editor.context_menu
    {
        operations.push(UiDeltaOperation::MarkdownSetMenus {
            node_id: next.id.clone(),
            insert_menu: next_editor.insert_menu.clone(),
            context_menu: next_editor.context_menu.clone(),
        });
    }
    operations
}

/// Builds compact operations between two authoritative Tree projections.
///
/// Entry collections are replaced through the keyed child-splice primitive;
/// selection, filter text, and display location remain independent so focus
/// updates do not resend a large hierarchy.
#[must_use]
pub fn tree_delta_operations(previous: &UiNode, next: &UiNode) -> Vec<UiDeltaOperation> {
    let (UiComponent::Tree(previous_tree), UiComponent::Tree(next_tree)) =
        (&previous.element, &next.element)
    else {
        return vec![UiDeltaOperation::ReplaceRoot { root: next.clone() }];
    };
    if previous.id != next.id
        || previous_tree.label != next_tree.label
        || previous_tree.presentation != next_tree.presentation
        || previous_tree.empty_message != next_tree.empty_message
        || previous_tree.primary_action != next_tree.primary_action
        || previous_tree.context_menu != next_tree.context_menu
        || previous_tree.actions != next_tree.actions
        || previous_tree.filter.as_ref().map(|filter| {
            (
                &filter.id,
                &filter.label,
                &filter.placeholder,
                &filter.set_value,
            )
        }) != next_tree.filter.as_ref().map(|filter| {
            (
                &filter.id,
                &filter.label,
                &filter.placeholder,
                &filter.set_value,
            )
        })
    {
        return vec![UiDeltaOperation::ReplaceRoot { root: next.clone() }];
    }

    let mut operations = Vec::new();
    if previous_tree.location != next_tree.location {
        operations.push(UiDeltaOperation::TreeSetLocation {
            node_id: next.id.clone(),
            location: next_tree.location.clone(),
        });
    }
    if previous_tree.filter.as_ref().map(|filter| &filter.value)
        != next_tree.filter.as_ref().map(|filter| &filter.value)
        && let Some(filter) = &next_tree.filter
    {
        operations.push(UiDeltaOperation::TreeSetFilter {
            filter_id: filter.id.clone(),
            value: filter.value.clone(),
        });
    }
    if previous_tree.items != next_tree.items {
        operations.push(UiDeltaOperation::TreeSpliceChildren {
            node_id: next.id.clone(),
            parent_id: None,
            index: 0,
            delete_count: u64::try_from(previous_tree.items.len()).unwrap_or(u64::MAX),
            items: next_tree.items.clone(),
        });
    }
    if previous_tree.selected_id != next_tree.selected_id {
        operations.push(UiDeltaOperation::TreeSetSelection {
            node_id: next.id.clone(),
            selected_id: next_tree.selected_id.clone(),
        });
    }
    operations
}

/// Builds compact operations for one stable Page projection.
///
/// List focus and Content line ranges remain independent. Content collection
/// changes use one splice so large issue bodies and patches never require a
/// snapshot-per-change fallback.
#[must_use]
pub fn page_delta_operations(previous: &UiNode, next: &UiNode) -> Vec<UiDeltaOperation> {
    let (UiComponent::Page(previous_page), UiComponent::Page(next_page)) =
        (&previous.element, &next.element)
    else {
        return vec![UiDeltaOperation::ReplaceRoot { root: next.clone() }];
    };
    if previous.id != next.id
        || previous_page.title != next_page.title
        || previous_page.back != next_page.back
        || previous_page.header != next_page.header
    {
        return vec![UiDeltaOperation::ReplaceRoot { root: next.clone() }];
    }
    match (&previous_page.body, &next_page.body) {
        (PageBodySlot::List(previous_list), PageBodySlot::List(next_list)) => {
            if previous_list.id != next_list.id
                || previous_list.items != next_list.items
                || previous_list.empty_message != next_list.empty_message
                || previous_list.select != next_list.select
                || previous_list.scroll_padding != next_list.scroll_padding
                || previous_list.page_overlap != next_list.page_overlap
                || previous_list.page_behavior != next_list.page_behavior
                || previous_list.space_pages_down != next_list.space_pages_down
                || previous_list.context_menu != next_list.context_menu
            {
                return vec![UiDeltaOperation::ReplaceRoot { root: next.clone() }];
            }
            if previous_list.selected_id != next_list.selected_id {
                vec![UiDeltaOperation::ListSetSelection {
                    list_id: next_list.id.clone(),
                    selected_id: next_list.selected_id.clone(),
                }]
            } else {
                Vec::new()
            }
        }
        (PageBodySlot::Content(previous_content), PageBodySlot::Content(next_content)) => {
            if previous_content.id != next_content.id
                || previous_content.label != next_content.label
                || previous_content.wrap != next_content.wrap
                || previous_content.font != next_content.font
                || previous_content.empty_message != next_content.empty_message
                || previous_content.select != next_content.select
                || previous_content.context_menu != next_content.context_menu
            {
                return vec![UiDeltaOperation::ReplaceRoot { root: next.clone() }];
            }
            let mut operations = Vec::new();
            if previous_content.lines != next_content.lines {
                operations.push(UiDeltaOperation::ContentSpliceLines {
                    content_id: next_content.id.clone(),
                    index: 0,
                    delete_count: u64::try_from(previous_content.lines.len()).unwrap_or(u64::MAX),
                    lines: next_content.lines.clone(),
                });
            }
            if previous_content.selection != next_content.selection {
                operations.push(UiDeltaOperation::ContentSetSelection {
                    content_id: next_content.id.clone(),
                    selection: next_content.selection.clone(),
                });
            }
            operations
        }
        _ => vec![UiDeltaOperation::ReplaceRoot { root: next.clone() }],
    }
}

fn contiguous_text_edit(previous: &str, next: &str) -> TextEdit {
    let previous = previous.chars().collect::<Vec<_>>();
    let next = next.chars().collect::<Vec<_>>();
    let mut prefix = 0usize;
    while prefix < previous.len() && prefix < next.len() && previous[prefix] == next[prefix] {
        prefix += 1;
    }
    let mut suffix = 0usize;
    while suffix < previous.len().saturating_sub(prefix)
        && suffix < next.len().saturating_sub(prefix)
        && previous[previous.len() - suffix - 1] == next[next.len() - suffix - 1]
    {
        suffix += 1;
    }
    let start = text_position_at_character_offset(&previous, prefix);
    let end = text_position_at_character_offset(&previous, previous.len() - suffix);
    let replacement = next[prefix..next.len() - suffix].iter().collect::<String>();
    TextEdit::new(TextRange::new(start, end), replacement)
}

fn text_position_at_character_offset(characters: &[char], offset: usize) -> TextPosition {
    let mut line = 0u32;
    let mut utf16_column = 0u32;
    for character in characters.iter().take(offset) {
        if *character == '\n' {
            line = line.saturating_add(1);
            utf16_column = 0;
        } else {
            utf16_column = utf16_column
                .saturating_add(u32::try_from(character.len_utf16()).unwrap_or(u32::MAX));
        }
    }
    TextPosition::new(line, utf16_column)
}

/// Complete projection for one logical view and renderer client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSnapshot {
    pub protocol: String,
    pub protocol_version: u32,
    pub app_instance_id: AppInstanceId,
    pub client_id: ClientId,
    pub view_id: ViewId,
    pub revision: u64,
    pub root: UiNode,
}

impl UiSnapshot {
    #[must_use]
    pub fn new(
        app_instance_id: impl Into<AppInstanceId>,
        client_id: impl Into<ClientId>,
        view_id: impl Into<ViewId>,
        revision: u64,
        root: UiNode,
    ) -> Self {
        Self {
            protocol: UI_PROTOCOL_NAME.to_owned(),
            protocol_version: UI_PROTOCOL_VERSION,
            app_instance_id: app_instance_id.into(),
            client_id: client_id.into(),
            view_id: view_id.into(),
            revision,
            root,
        }
    }

    /// Applies a contiguous server delta and returns the next complete state.
    pub fn applying(&self, delta: &UiDelta) -> Result<Self, UiProtocolError> {
        validate_delta(delta)?;
        if self.protocol != delta.protocol
            || self.protocol_version != delta.protocol_version
            || self.app_instance_id != delta.app_instance_id
            || self.client_id != delta.client_id
            || self.view_id != delta.view_id
        {
            return Err(UiProtocolError::InvalidMessage(
                "delta route does not match the current snapshot".to_owned(),
            ));
        }
        if self.revision != delta.base_revision {
            return Err(UiProtocolError::InvalidMessage(format!(
                "delta base revision {} does not match snapshot revision {}",
                delta.base_revision, self.revision
            )));
        }
        let mut root = self.root.clone();
        root.apply_delta_operations(&delta.operations)
            .map_err(UiProtocolError::InvalidView)?;
        let mut snapshot = Self::new(
            delta.app_instance_id.clone(),
            delta.client_id.clone(),
            delta.view_id.clone(),
            delta.revision,
            root,
        );
        snapshot.protocol_version = delta.protocol_version;
        Ok(snapshot)
    }
}

/// One ordered mutation inside a revision delta.
///
/// Component-specific operations keep large documents and future grids
/// efficient. `replaceRoot` remains a complete fallback for uncommon changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UiDeltaOperation {
    ReplaceRoot {
        root: UiNode,
    },
    MarkdownReplaceRange {
        node_id: NodeId,
        edit: TextEdit,
    },
    MarkdownSetSelection {
        node_id: NodeId,
        selection: TextSelection,
    },
    MarkdownSetPresentation {
        node_id: NodeId,
        presentation: MarkdownPresentation,
    },
    MarkdownSetDirty {
        node_id: NodeId,
        dirty: bool,
    },
    MarkdownSetReadOnly {
        node_id: NodeId,
        read_only: bool,
    },
    MarkdownSetTitle {
        node_id: NodeId,
        title: Option<String>,
    },
    MarkdownSetPlaceholder {
        node_id: NodeId,
        placeholder: String,
    },
    MarkdownSetActions {
        node_id: NodeId,
        actions: MarkdownEditorActions,
    },
    MarkdownSetMenus {
        node_id: NodeId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        insert_menu: Option<SemanticMenu>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context_menu: Option<SemanticMenu>,
    },
    MenuSetSelection {
        node_id: NodeId,
        selected_id: Option<String>,
    },
    /// Swaps image bytes by reference and updates their intrinsic metadata.
    MediaSetSource {
        node_id: NodeId,
        source: MediaSource,
        intrinsic: MediaPixelSize,
    },
    /// Routes this box to another authorized retained-scene stream.
    SurfaceSetReference {
        node_id: NodeId,
        reference: SurfaceReference,
    },
    /// Sets one Toggle and its containing ListItem's denormalized done state.
    ToggleSetValue {
        node_id: String,
        value: bool,
    },
    /// Sets one selection-mode Checkmark without changing row focus.
    CheckmarkSetValue {
        node_id: String,
        value: bool,
    },
    InputSetValue {
        node_id: String,
        value: String,
    },
    ListInsertItem {
        list_id: String,
        index: u64,
        item: ListItem,
    },
    ListSetSelection {
        list_id: String,
        selected_id: Option<String>,
    },
    ListRemoveItem {
        list_id: String,
        item_id: String,
    },
    ContentSetSelection {
        content_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selection: Option<ContentSelection>,
    },
    ContentSpliceLines {
        content_id: String,
        index: u64,
        delete_count: u64,
        lines: Vec<ContentLine>,
    },
    TreeSetSelection {
        node_id: NodeId,
        selected_id: Option<String>,
    },
    TreeSetFilter {
        filter_id: String,
        value: String,
    },
    TreeSetLocation {
        node_id: NodeId,
        location: String,
    },
    TreeSpliceChildren {
        node_id: NodeId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_id: Option<String>,
        index: u64,
        delete_count: u64,
        items: Vec<TreeItem>,
    },
    TreeSetChildState {
        node_id: NodeId,
        item_id: String,
        child_state: TreeChildState,
    },
    TreeSetExpanded {
        node_id: NodeId,
        item_id: String,
        expanded: bool,
    },
}

impl UiDeltaOperation {
    #[must_use]
    pub fn markdown_replace_range(node_id: impl Into<NodeId>, edit: TextEdit) -> Self {
        Self::MarkdownReplaceRange {
            node_id: node_id.into(),
            edit,
        }
    }

    #[must_use]
    pub fn markdown_set_selection(node_id: impl Into<NodeId>, selection: TextSelection) -> Self {
        Self::MarkdownSetSelection {
            node_id: node_id.into(),
            selection,
        }
    }

    #[must_use]
    pub fn media_set_source(
        node_id: impl Into<NodeId>,
        source: MediaSource,
        intrinsic: MediaPixelSize,
    ) -> Self {
        Self::MediaSetSource {
            node_id: node_id.into(),
            source,
            intrinsic,
        }
    }

    #[must_use]
    pub fn surface_set_reference(node_id: impl Into<NodeId>, reference: SurfaceReference) -> Self {
        Self::SurfaceSetReference {
            node_id: node_id.into(),
            reference,
        }
    }

    #[must_use]
    pub fn toggle_set_value(node_id: impl Into<String>, value: bool) -> Self {
        Self::ToggleSetValue {
            node_id: node_id.into(),
            value,
        }
    }

    #[must_use]
    pub fn checkmark_set_value(node_id: impl Into<String>, value: bool) -> Self {
        Self::CheckmarkSetValue {
            node_id: node_id.into(),
            value,
        }
    }

    #[must_use]
    pub fn input_set_value(node_id: impl Into<String>, value: impl Into<String>) -> Self {
        Self::InputSetValue {
            node_id: node_id.into(),
            value: value.into(),
        }
    }

    #[must_use]
    pub fn list_insert_item(list_id: impl Into<String>, index: u64, item: ListItem) -> Self {
        Self::ListInsertItem {
            list_id: list_id.into(),
            index,
            item,
        }
    }

    #[must_use]
    pub fn list_remove_item(list_id: impl Into<String>, item_id: impl Into<String>) -> Self {
        Self::ListRemoveItem {
            list_id: list_id.into(),
            item_id: item_id.into(),
        }
    }

    #[must_use]
    pub fn list_set_selection(list_id: impl Into<String>, selected_id: Option<String>) -> Self {
        Self::ListSetSelection {
            list_id: list_id.into(),
            selected_id,
        }
    }

    #[must_use]
    pub fn content_set_selection(
        content_id: impl Into<String>,
        selection: Option<ContentSelection>,
    ) -> Self {
        Self::ContentSetSelection {
            content_id: content_id.into(),
            selection,
        }
    }

    #[must_use]
    pub fn tree_set_selection(node_id: impl Into<NodeId>, selected_id: Option<String>) -> Self {
        Self::TreeSetSelection {
            node_id: node_id.into(),
            selected_id,
        }
    }

    #[must_use]
    pub fn tree_set_filter(filter_id: impl Into<String>, value: impl Into<String>) -> Self {
        Self::TreeSetFilter {
            filter_id: filter_id.into(),
            value: value.into(),
        }
    }

    fn validate(&self, path: &str) -> Result<(), UiProtocolError> {
        match self {
            Self::ReplaceRoot { root } => root.validate().map_err(UiProtocolError::InvalidView),
            Self::MarkdownReplaceRange { node_id, edit } => {
                validate_identifier(node_id.as_str(), &format!("{path}.nodeId"))
                    .map_err(UiProtocolError::InvalidView)?;
                if edit.range.start > edit.range.end {
                    return Err(UiProtocolError::InvalidMessage(format!(
                        "{path}.edit range is reversed"
                    )));
                }
                Ok(())
            }
            Self::MediaSetSource {
                node_id,
                source,
                intrinsic,
            } => {
                validate_identifier(node_id.as_str(), &format!("{path}.nodeId"))
                    .map_err(UiProtocolError::InvalidView)?;
                MediaSpec::new(source.clone(), *intrinsic, "")
                    .validate()
                    .map_err(|error| {
                        UiProtocolError::InvalidView(UiValidationError::new(
                            format!("{path}.{}", error.path.replacen("media.", "", 1)),
                            error.message,
                        ))
                    })
            }
            Self::SurfaceSetReference { node_id, reference } => {
                validate_identifier(node_id.as_str(), &format!("{path}.nodeId"))
                    .map_err(UiProtocolError::InvalidView)?;
                SurfaceSpec::new(reference.clone())
                    .validate()
                    .map_err(|error| {
                        UiProtocolError::InvalidView(UiValidationError::new(
                            format!("{path}.{}", error.path.replacen("surface.", "", 1)),
                            error.message,
                        ))
                    })
            }
            Self::ToggleSetValue { node_id, .. }
            | Self::CheckmarkSetValue { node_id, .. }
            | Self::InputSetValue { node_id, .. } => {
                validate_identifier(node_id, &format!("{path}.nodeId"))
                    .map_err(UiProtocolError::InvalidView)
            }
            Self::ListInsertItem {
                list_id,
                index,
                item,
            } => {
                validate_identifier(list_id, &format!("{path}.listId"))
                    .map_err(UiProtocolError::InvalidView)?;
                if *index > MAX_SAFE_UI_INTEGER {
                    return Err(UiProtocolError::InvalidMessage(format!(
                        "{path}.index exceeds the cross-platform safe integer {MAX_SAFE_UI_INTEGER}"
                    )));
                }
                item.validate(&format!("{path}.item")).map_err(|error| {
                    UiProtocolError::InvalidView(UiValidationError::new(error.path, error.message))
                })
            }
            Self::ListRemoveItem { list_id, item_id } => {
                validate_identifier(list_id, &format!("{path}.listId"))
                    .map_err(UiProtocolError::InvalidView)?;
                validate_identifier(item_id, &format!("{path}.itemId"))
                    .map_err(UiProtocolError::InvalidView)
            }
            Self::ListSetSelection {
                list_id,
                selected_id,
            } => {
                validate_identifier(list_id, &format!("{path}.listId"))
                    .map_err(UiProtocolError::InvalidView)?;
                if let Some(selected_id) = selected_id {
                    validate_identifier(selected_id, &format!("{path}.selectedId"))
                        .map_err(UiProtocolError::InvalidView)?;
                }
                Ok(())
            }
            Self::ContentSetSelection {
                content_id,
                selection,
            } => {
                validate_identifier(content_id, &format!("{path}.contentId"))
                    .map_err(UiProtocolError::InvalidView)?;
                if let Some(selection) = selection {
                    validate_identifier(
                        &selection.anchor_id,
                        &format!("{path}.selection.anchorId"),
                    )
                    .map_err(UiProtocolError::InvalidView)?;
                    validate_identifier(&selection.head_id, &format!("{path}.selection.headId"))
                        .map_err(UiProtocolError::InvalidView)?;
                }
                Ok(())
            }
            Self::ContentSpliceLines {
                content_id,
                index,
                delete_count,
                lines,
            } => {
                validate_identifier(content_id, &format!("{path}.contentId"))
                    .map_err(UiProtocolError::InvalidView)?;
                if *index > MAX_SAFE_UI_INTEGER || *delete_count > MAX_SAFE_UI_INTEGER {
                    return Err(UiProtocolError::InvalidMessage(format!(
                        "{path} Content splice exceeds the cross-platform safe integer {MAX_SAFE_UI_INTEGER}"
                    )));
                }
                let probe = crate::Content::new("content-probe", "Content", lines.clone());
                probe.validate("contentSplice.lines").map_err(|error| {
                    UiProtocolError::InvalidView(UiValidationError::new(error.path, error.message))
                })
            }
            Self::TreeSetSelection {
                node_id,
                selected_id,
            } => {
                validate_identifier(node_id.as_str(), &format!("{path}.nodeId"))
                    .map_err(UiProtocolError::InvalidView)?;
                if let Some(selected_id) = selected_id {
                    validate_identifier(selected_id, &format!("{path}.selectedId"))
                        .map_err(UiProtocolError::InvalidView)?;
                }
                Ok(())
            }
            Self::MenuSetSelection {
                node_id,
                selected_id,
            } => {
                validate_identifier(node_id.as_str(), &format!("{path}.nodeId"))
                    .map_err(UiProtocolError::InvalidView)?;
                if let Some(selected_id) = selected_id {
                    validate_identifier(selected_id, &format!("{path}.selectedId"))
                        .map_err(UiProtocolError::InvalidView)?;
                }
                Ok(())
            }
            Self::TreeSetFilter { filter_id, .. } => {
                validate_identifier(filter_id, &format!("{path}.filterId"))
                    .map_err(UiProtocolError::InvalidView)
            }
            Self::TreeSetLocation { node_id, .. } => {
                validate_identifier(node_id.as_str(), &format!("{path}.nodeId"))
                    .map_err(UiProtocolError::InvalidView)
            }
            Self::TreeSpliceChildren {
                node_id,
                parent_id,
                index,
                delete_count,
                ..
            } => {
                validate_identifier(node_id.as_str(), &format!("{path}.nodeId"))
                    .map_err(UiProtocolError::InvalidView)?;
                if let Some(parent_id) = parent_id {
                    validate_identifier(parent_id, &format!("{path}.parentId"))
                        .map_err(UiProtocolError::InvalidView)?;
                }
                if *index > MAX_SAFE_UI_INTEGER || *delete_count > MAX_SAFE_UI_INTEGER {
                    return Err(UiProtocolError::InvalidMessage(format!(
                        "{path} Tree splice exceeds the cross-platform safe integer {MAX_SAFE_UI_INTEGER}"
                    )));
                }
                Ok(())
            }
            Self::TreeSetChildState {
                node_id, item_id, ..
            }
            | Self::TreeSetExpanded {
                node_id, item_id, ..
            } => {
                validate_identifier(node_id.as_str(), &format!("{path}.nodeId"))
                    .map_err(UiProtocolError::InvalidView)?;
                validate_identifier(item_id, &format!("{path}.itemId"))
                    .map_err(UiProtocolError::InvalidView)
            }
            Self::MarkdownSetSelection { node_id, .. }
            | Self::MarkdownSetPresentation { node_id, .. }
            | Self::MarkdownSetDirty { node_id, .. }
            | Self::MarkdownSetReadOnly { node_id, .. }
            | Self::MarkdownSetTitle { node_id, .. }
            | Self::MarkdownSetPlaceholder { node_id, .. }
            | Self::MarkdownSetActions { node_id, .. }
            | Self::MarkdownSetMenus { node_id, .. } => {
                validate_identifier(node_id.as_str(), &format!("{path}.nodeId"))
                    .map_err(UiProtocolError::InvalidView)
            }
        }
    }
}

impl UiNode {
    /// Applies ordered operations and validates the resulting complete node.
    pub fn apply_delta_operations(
        &mut self,
        operations: &[UiDeltaOperation],
    ) -> Result<(), UiValidationError> {
        for (index, operation) in operations.iter().enumerate() {
            match operation {
                UiDeltaOperation::ReplaceRoot { root } => *self = root.clone(),
                UiDeltaOperation::MarkdownReplaceRange { node_id, edit } => {
                    let editor = self.markdown_editor_mut(node_id, index)?;
                    apply_text_edit(&mut editor.text, edit).map_err(|message| {
                        UiValidationError::new(format!("delta.operations[{index}].edit"), message)
                    })?;
                }
                UiDeltaOperation::MarkdownSetSelection { node_id, selection } => {
                    self.markdown_editor_mut(node_id, index)?.selection = *selection;
                }
                UiDeltaOperation::MarkdownSetPresentation {
                    node_id,
                    presentation,
                } => {
                    self.markdown_editor_mut(node_id, index)?.presentation = *presentation;
                }
                UiDeltaOperation::MarkdownSetDirty { node_id, dirty } => {
                    self.markdown_editor_mut(node_id, index)?.dirty = *dirty;
                }
                UiDeltaOperation::MarkdownSetReadOnly { node_id, read_only } => {
                    self.markdown_editor_mut(node_id, index)?.read_only = *read_only;
                }
                UiDeltaOperation::MarkdownSetTitle { node_id, title } => {
                    self.markdown_editor_mut(node_id, index)?.title = title.clone();
                }
                UiDeltaOperation::MarkdownSetPlaceholder {
                    node_id,
                    placeholder,
                } => {
                    self.markdown_editor_mut(node_id, index)?.placeholder = placeholder.clone();
                }
                UiDeltaOperation::MarkdownSetActions { node_id, actions } => {
                    self.markdown_editor_mut(node_id, index)?.actions = actions.clone();
                }
                UiDeltaOperation::MarkdownSetMenus {
                    node_id,
                    insert_menu,
                    context_menu,
                } => {
                    let editor = self.markdown_editor_mut(node_id, index)?;
                    editor.insert_menu = insert_menu.clone();
                    editor.context_menu = context_menu.clone();
                }
                UiDeltaOperation::MenuSetSelection {
                    node_id,
                    selected_id,
                } => {
                    let menu = self.menu_mut(node_id, index)?;
                    menu.selected_id = selected_id.clone();
                }
                UiDeltaOperation::MediaSetSource {
                    node_id,
                    source,
                    intrinsic,
                } => {
                    let media = self.media_mut(node_id, index)?;
                    media.source = source.clone();
                    media.intrinsic = *intrinsic;
                }
                UiDeltaOperation::SurfaceSetReference { node_id, reference } => {
                    self.surface_mut(node_id, index)?.reference = reference.clone();
                }
                UiDeltaOperation::ToggleSetValue { node_id, value } => {
                    self.page_mut(index)?
                        .set_toggle_value(node_id, *value)
                        .map_err(|error| component_delta_error(index, error))?;
                }
                UiDeltaOperation::CheckmarkSetValue { node_id, value } => {
                    self.page_mut(index)?
                        .set_checkmark_value(node_id, *value)
                        .map_err(|error| component_delta_error(index, error))?;
                }
                UiDeltaOperation::InputSetValue { node_id, value } => {
                    self.page_mut(index)?
                        .set_input_value(node_id, value.clone())
                        .map_err(|error| component_delta_error(index, error))?;
                }
                UiDeltaOperation::ListInsertItem {
                    list_id,
                    index: item_index,
                    item,
                } => {
                    let item_index = usize::try_from(*item_index).map_err(|_| {
                        UiValidationError::new(
                            format!("delta.operations[{index}].index"),
                            "List insertion index does not fit this renderer",
                        )
                    })?;
                    self.page_mut(index)?
                        .insert_list_item(list_id, item_index, item.clone())
                        .map_err(|error| component_delta_error(index, error))?;
                }
                UiDeltaOperation::ListRemoveItem { list_id, item_id } => {
                    self.page_mut(index)?
                        .remove_list_item(list_id, item_id)
                        .map_err(|error| component_delta_error(index, error))?;
                }
                UiDeltaOperation::ListSetSelection {
                    list_id,
                    selected_id,
                } => {
                    self.page_mut(index)?
                        .set_list_selection(list_id, selected_id.clone())
                        .map_err(|error| component_delta_error(index, error))?;
                }
                UiDeltaOperation::ContentSetSelection {
                    content_id,
                    selection,
                } => {
                    self.page_mut(index)?
                        .set_content_selection(content_id, selection.clone())
                        .map_err(|error| component_delta_error(index, error))?;
                }
                UiDeltaOperation::ContentSpliceLines {
                    content_id,
                    index: line_index,
                    delete_count,
                    lines,
                } => {
                    let line_index = usize::try_from(*line_index).map_err(|_| {
                        UiValidationError::new(
                            format!("delta.operations[{index}].index"),
                            "Content line index does not fit this renderer",
                        )
                    })?;
                    let delete_count = usize::try_from(*delete_count).map_err(|_| {
                        UiValidationError::new(
                            format!("delta.operations[{index}].deleteCount"),
                            "Content delete count does not fit this renderer",
                        )
                    })?;
                    self.page_mut(index)?
                        .splice_content_lines(content_id, line_index, delete_count, lines.clone())
                        .map_err(|error| component_delta_error(index, error))?;
                }
                UiDeltaOperation::TreeSetSelection {
                    node_id,
                    selected_id,
                } => {
                    self.tree_mut(node_id, index)?
                        .set_selection(selected_id.clone())
                        .map_err(|error| tree_delta_error(index, error))?;
                }
                UiDeltaOperation::TreeSetFilter { filter_id, value } => {
                    self.tree_mut_for_operation(index)?
                        .set_filter_value(filter_id, value.clone())
                        .map_err(|error| tree_delta_error(index, error))?;
                }
                UiDeltaOperation::TreeSetLocation { node_id, location } => {
                    self.tree_mut(node_id, index)?.location = location.clone();
                }
                UiDeltaOperation::TreeSpliceChildren {
                    node_id,
                    parent_id,
                    index: child_index,
                    delete_count,
                    items,
                } => {
                    let child_index = usize::try_from(*child_index).map_err(|_| {
                        UiValidationError::new(
                            format!("delta.operations[{index}].index"),
                            "Tree splice index does not fit this renderer",
                        )
                    })?;
                    let delete_count = usize::try_from(*delete_count).map_err(|_| {
                        UiValidationError::new(
                            format!("delta.operations[{index}].deleteCount"),
                            "Tree splice count does not fit this renderer",
                        )
                    })?;
                    self.tree_mut(node_id, index)?
                        .splice_children(
                            parent_id.as_deref(),
                            child_index,
                            delete_count,
                            items.clone(),
                        )
                        .map_err(|error| tree_delta_error(index, error))?;
                }
                UiDeltaOperation::TreeSetChildState {
                    node_id,
                    item_id,
                    child_state,
                } => {
                    self.tree_mut(node_id, index)?
                        .set_child_state(item_id, *child_state)
                        .map_err(|error| tree_delta_error(index, error))?;
                }
                UiDeltaOperation::TreeSetExpanded {
                    node_id,
                    item_id,
                    expanded,
                } => {
                    self.tree_mut(node_id, index)?
                        .set_expanded(item_id, *expanded)
                        .map_err(|error| tree_delta_error(index, error))?;
                }
            }
        }
        self.validate()
    }

    fn markdown_editor_mut(
        &mut self,
        expected_id: &NodeId,
        operation_index: usize,
    ) -> Result<&mut MarkdownEditorSpec, UiValidationError> {
        if &self.id != expected_id {
            return Err(UiValidationError::new(
                format!("delta.operations[{operation_index}].nodeId"),
                format!("node {expected_id:?} is not present"),
            ));
        }
        match &mut self.element {
            UiComponent::MarkdownEditor(editor) => Ok(editor),
            UiComponent::CanvasPage(_)
            | UiComponent::Media(_)
            | UiComponent::Menu(_)
            | UiComponent::Page(_)
            | UiComponent::Surface(_)
            | UiComponent::Tree(_) => Err(UiValidationError::new(
                format!("delta.operations[{operation_index}].nodeId"),
                "operation requires a Markdown editor",
            )),
        }
    }

    fn media_mut(
        &mut self,
        expected_id: &NodeId,
        operation_index: usize,
    ) -> Result<&mut MediaSpec, UiValidationError> {
        if &self.id != expected_id {
            return Err(UiValidationError::new(
                format!("delta.operations[{operation_index}].nodeId"),
                format!("node {expected_id:?} is not present"),
            ));
        }
        match &mut self.element {
            UiComponent::Media(media) => Ok(media),
            UiComponent::CanvasPage(_)
            | UiComponent::MarkdownEditor(_)
            | UiComponent::Menu(_)
            | UiComponent::Page(_)
            | UiComponent::Surface(_)
            | UiComponent::Tree(_) => Err(UiValidationError::new(
                format!("delta.operations[{operation_index}].nodeId"),
                "operation requires Media",
            )),
        }
    }

    fn surface_mut(
        &mut self,
        expected_id: &NodeId,
        operation_index: usize,
    ) -> Result<&mut SurfaceSpec, UiValidationError> {
        match &mut self.element {
            UiComponent::Surface(surface) if &self.id == expected_id => Ok(surface),
            UiComponent::CanvasPage(page) if page.surface.id == expected_id.as_str() => {
                Ok(&mut page.surface.surface)
            }
            UiComponent::CanvasPage(_)
            | UiComponent::MarkdownEditor(_)
            | UiComponent::Media(_)
            | UiComponent::Menu(_)
            | UiComponent::Page(_)
            | UiComponent::Surface(_)
            | UiComponent::Tree(_) => Err(UiValidationError::new(
                format!("delta.operations[{operation_index}].nodeId"),
                format!("Surface node {expected_id:?} is not present"),
            )),
        }
    }

    fn page_mut(&mut self, operation_index: usize) -> Result<&mut Page, UiValidationError> {
        match &mut self.element {
            UiComponent::Page(page) => Ok(page),
            UiComponent::CanvasPage(_)
            | UiComponent::MarkdownEditor(_)
            | UiComponent::Media(_)
            | UiComponent::Menu(_)
            | UiComponent::Surface(_)
            | UiComponent::Tree(_) => Err(UiValidationError::new(
                format!("delta.operations[{operation_index}]"),
                "operation requires Page",
            )),
        }
    }

    fn tree_mut(
        &mut self,
        expected_id: &NodeId,
        operation_index: usize,
    ) -> Result<&mut Tree, UiValidationError> {
        if &self.id != expected_id {
            return Err(UiValidationError::new(
                format!("delta.operations[{operation_index}].nodeId"),
                format!("Tree node {expected_id:?} is not present"),
            ));
        }
        self.tree_mut_for_operation(operation_index)
    }

    fn menu_mut(
        &mut self,
        expected_id: &NodeId,
        operation_index: usize,
    ) -> Result<&mut SemanticMenu, UiValidationError> {
        if &self.id != expected_id {
            return Err(UiValidationError::new(
                format!("delta.operations[{operation_index}].nodeId"),
                format!("Menu node {expected_id:?} is not present"),
            ));
        }
        match &mut self.element {
            UiComponent::Menu(menu) => Ok(menu),
            UiComponent::CanvasPage(_)
            | UiComponent::MarkdownEditor(_)
            | UiComponent::Media(_)
            | UiComponent::Page(_)
            | UiComponent::Surface(_)
            | UiComponent::Tree(_) => Err(UiValidationError::new(
                format!("delta.operations[{operation_index}].nodeId"),
                "operation requires Menu",
            )),
        }
    }

    fn tree_mut_for_operation(
        &mut self,
        operation_index: usize,
    ) -> Result<&mut Tree, UiValidationError> {
        match &mut self.element {
            UiComponent::Tree(tree) => Ok(tree),
            UiComponent::CanvasPage(_)
            | UiComponent::MarkdownEditor(_)
            | UiComponent::Media(_)
            | UiComponent::Menu(_)
            | UiComponent::Page(_)
            | UiComponent::Surface(_) => Err(UiValidationError::new(
                format!("delta.operations[{operation_index}]"),
                "operation requires Tree",
            )),
        }
    }
}

fn component_delta_error(
    operation_index: usize,
    error: crate::ComponentValidationError,
) -> UiValidationError {
    UiValidationError::new(
        format!("delta.operations[{operation_index}].{}", error.path),
        error.message,
    )
}

fn tree_delta_error(
    operation_index: usize,
    error: crate::TreeValidationError,
) -> UiValidationError {
    UiValidationError::new(
        format!("delta.operations[{operation_index}].{}", error.path),
        error.message,
    )
}

/// Contiguous server-to-renderer change between two immutable revisions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiDelta {
    pub protocol: String,
    pub protocol_version: u32,
    pub app_instance_id: AppInstanceId,
    pub client_id: ClientId,
    pub view_id: ViewId,
    pub base_revision: u64,
    pub revision: u64,
    pub operations: Vec<UiDeltaOperation>,
}

impl UiDelta {
    #[must_use]
    pub fn new(
        app_instance_id: impl Into<AppInstanceId>,
        client_id: impl Into<ClientId>,
        view_id: impl Into<ViewId>,
        base_revision: u64,
        revision: u64,
        operations: Vec<UiDeltaOperation>,
    ) -> Self {
        Self {
            protocol: UI_PROTOCOL_NAME.to_owned(),
            protocol_version: UI_PROTOCOL_VERSION,
            app_instance_id: app_instance_id.into(),
            client_id: client_id.into(),
            view_id: view_id.into(),
            base_revision,
            revision,
            operations,
        }
    }
}

/// Semantic interaction category, independent of keys and pointer geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UiEventKind {
    Activate,
    Select,
    Change,
    Submit,
    Cancel,
    Command,
}

/// Typed event data shared by Swift, web, and Rust.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum UiEventValue {
    #[default]
    None,
    Bool(bool),
    Index(u64),
    Integer(i64),
    Number(f64),
    Text(String),
    TextList(Vec<String>),
    TextEdit(TextEdit),
    TextSelection(TextSelection),
}

/// Renderer-local component action before the session envelope is applied.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAction {
    pub node_id: NodeId,
    pub action: ActionId,
    pub kind: UiEventKind,
    pub value: UiEventValue,
}

impl UiAction {
    #[must_use]
    pub fn new(
        node_id: impl Into<NodeId>,
        action: impl Into<ActionId>,
        kind: UiEventKind,
        value: UiEventValue,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            action: action.into(),
            kind,
            value,
        }
    }

    #[must_use]
    pub fn replace_range(node_id: impl Into<NodeId>, edit: TextEdit) -> Self {
        Self::new(
            node_id,
            MarkdownEditorActions::REPLACE_RANGE,
            UiEventKind::Change,
            UiEventValue::TextEdit(edit),
        )
    }

    #[must_use]
    pub fn set_selection(node_id: impl Into<NodeId>, selection: TextSelection) -> Self {
        Self::new(
            node_id,
            MarkdownEditorActions::SET_SELECTION,
            UiEventKind::Select,
            UiEventValue::TextSelection(selection),
        )
    }

    #[must_use]
    pub fn command(node_id: impl Into<NodeId>, action: impl Into<ActionId>) -> Self {
        Self::new(node_id, action, UiEventKind::Command, UiEventValue::None)
    }

    #[must_use]
    pub fn activate(node_id: impl Into<NodeId>, action: impl Into<ActionId>) -> Self {
        Self::new(node_id, action, UiEventKind::Activate, UiEventValue::None)
    }
}

/// One authenticated, idempotent renderer action against a snapshot revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiEvent {
    pub protocol: String,
    pub protocol_version: u32,
    pub app_instance_id: AppInstanceId,
    pub participant_id: ParticipantId,
    pub client_id: ClientId,
    pub renderer_id: RendererId,
    pub view_id: ViewId,
    pub event_id: EventId,
    pub base_revision: u64,
    #[serde(flatten)]
    pub action: UiAction,
}

impl UiEvent {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        app_instance_id: impl Into<AppInstanceId>,
        participant_id: impl Into<ParticipantId>,
        client_id: impl Into<ClientId>,
        renderer_id: impl Into<RendererId>,
        view_id: impl Into<ViewId>,
        event_id: impl Into<EventId>,
        base_revision: u64,
        action: UiAction,
    ) -> Self {
        Self {
            protocol: UI_PROTOCOL_NAME.to_owned(),
            protocol_version: UI_PROTOCOL_VERSION,
            app_instance_id: app_instance_id.into(),
            participant_id: participant_id.into(),
            client_id: client_id.into(),
            renderer_id: renderer_id.into(),
            view_id: view_id.into(),
            event_id: event_id.into(),
            base_revision,
            action,
        }
    }

    #[must_use]
    pub fn node_id(&self) -> &NodeId {
        &self.action.node_id
    }

    #[must_use]
    pub fn action_id(&self) -> &ActionId {
        &self.action.action
    }
}

/// Result returned for an event ID. Final results are replayed after reconnect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UiAckStatus {
    Pending,
    Applied,
    Rejected,
    Stale,
}

/// Idempotent event acknowledgement from the terminal App.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAck {
    pub protocol: String,
    pub protocol_version: u32,
    pub app_instance_id: AppInstanceId,
    pub client_id: ClientId,
    pub renderer_id: RendererId,
    pub view_id: ViewId,
    pub event_id: EventId,
    pub status: UiAckStatus,
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Visibility update for one attached renderer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiLifecycle {
    pub protocol: String,
    pub protocol_version: u32,
    pub app_instance_id: AppInstanceId,
    pub client_id: ClientId,
    pub renderer_id: RendererId,
    pub view_id: ViewId,
    pub state: UiRendererState,
}

impl UiLifecycle {
    #[must_use]
    pub fn new(
        app_instance_id: impl Into<AppInstanceId>,
        client_id: impl Into<ClientId>,
        renderer_id: impl Into<RendererId>,
        view_id: impl Into<ViewId>,
        state: UiRendererState,
    ) -> Self {
        Self {
            protocol: UI_PROTOCOL_NAME.to_owned(),
            protocol_version: UI_PROTOCOL_VERSION,
            app_instance_id: app_instance_id.into(),
            client_id: client_id.into(),
            renderer_id: renderer_id.into(),
            view_id: view_id.into(),
            state,
        }
    }
}

/// Renderer request for the latest complete projection after reconnect/resync.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiRequestSnapshot {
    pub protocol: String,
    pub protocol_version: u32,
    pub app_instance_id: AppInstanceId,
    pub client_id: ClientId,
    pub renderer_id: RendererId,
    pub view_id: ViewId,
}

/// One currently attached participant projection exposed as presence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPresenceMember {
    pub participant: UiParticipant,
    pub client_id: ClientId,
    pub renderer: UiRendererMetadata,
    pub state: UiRendererState,
}

/// Complete presence set for one logical view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPresence {
    pub protocol: String,
    pub protocol_version: u32,
    pub app_instance_id: AppInstanceId,
    pub view_id: ViewId,
    pub members: Vec<UiPresenceMember>,
}

/// Connection-level rejection that is not a component event result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiErrorMessage {
    pub protocol: String,
    pub protocol_version: u32,
    pub code: String,
    pub message: String,
}

impl UiErrorMessage {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            protocol: UI_PROTOCOL_NAME.to_owned(),
            protocol_version: UI_PROTOCOL_VERSION,
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Every NDJSON frame has a visible camelCase `type` discriminator.
// Keep protocol frames as direct values: boxing only the snapshot variant would
// complicate every App reducer and fixture without changing the NDJSON wire size.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UiMessage {
    Attach(UiAttach),
    Attached(UiAttached),
    Snapshot(UiSnapshot),
    Delta(UiDelta),
    Event(UiEvent),
    Ack(UiAck),
    Lifecycle(UiLifecycle),
    RequestSnapshot(UiRequestSnapshot),
    Presence(UiPresence),
    Error(UiErrorMessage),
}

impl UiMessage {
    pub fn validate(&self) -> Result<(), UiProtocolError> {
        match self {
            Self::Attach(message) => validate_protocol_name(&message.protocol)?,
            Self::Attached(message) => {
                validate_header(&message.protocol, message.protocol_version)?;
            }
            Self::Snapshot(message) => {
                validate_header(&message.protocol, message.protocol_version)?;
            }
            Self::Delta(message) => {
                validate_header(&message.protocol, message.protocol_version)?;
            }
            Self::Event(message) => {
                validate_header(&message.protocol, message.protocol_version)?;
            }
            Self::Ack(message) => {
                validate_header(&message.protocol, message.protocol_version)?;
            }
            Self::Lifecycle(message) => {
                validate_header(&message.protocol, message.protocol_version)?;
            }
            Self::RequestSnapshot(message) => {
                validate_header(&message.protocol, message.protocol_version)?;
            }
            Self::Presence(message) => {
                validate_header(&message.protocol, message.protocol_version)?;
            }
            Self::Error(message) => {
                validate_header(&message.protocol, message.protocol_version)?;
            }
        }

        match self {
            Self::Attach(message) => validate_attach(message),
            Self::Attached(message) => {
                validate_protocol_range(
                    message.min_protocol_version,
                    message.max_protocol_version,
                    "attached",
                )?;
                if message.protocol_version < message.min_protocol_version
                    || message.protocol_version > message.max_protocol_version
                {
                    return Err(UiProtocolError::InvalidMessage(
                        "attached protocolVersion is outside the advertised server range"
                            .to_owned(),
                    ));
                }
                validate_app(&message.app)?;
                validate_session_route(
                    &message.app_instance_id,
                    &message.client_id,
                    &message.renderer_id,
                    &message.view_id,
                )?;
                validate_identifier(message.participant_id.as_str(), "attached.participantId")
                    .map_err(UiProtocolError::InvalidView)?;
                if let Some(revision) = message.current_revision {
                    validate_revision(revision)?;
                }
                Ok(())
            }
            Self::Snapshot(message) => {
                validate_identifier(message.app_instance_id.as_str(), "snapshot.appInstanceId")
                    .map_err(UiProtocolError::InvalidView)?;
                validate_identifier(message.client_id.as_str(), "snapshot.clientId")
                    .map_err(UiProtocolError::InvalidView)?;
                validate_identifier(message.view_id.as_str(), "snapshot.viewId")
                    .map_err(UiProtocolError::InvalidView)?;
                validate_revision(message.revision)?;
                message
                    .root
                    .validate()
                    .map_err(UiProtocolError::InvalidView)
            }
            Self::Delta(message) => validate_delta(message),
            Self::Event(message) => validate_event(message),
            Self::Ack(message) => {
                validate_session_route(
                    &message.app_instance_id,
                    &message.client_id,
                    &message.renderer_id,
                    &message.view_id,
                )?;
                validate_identifier(message.event_id.as_str(), "ack.eventId")
                    .map_err(UiProtocolError::InvalidView)?;
                validate_revision(message.revision)
            }
            Self::Lifecycle(message) => validate_session_route(
                &message.app_instance_id,
                &message.client_id,
                &message.renderer_id,
                &message.view_id,
            ),
            Self::RequestSnapshot(message) => validate_session_route(
                &message.app_instance_id,
                &message.client_id,
                &message.renderer_id,
                &message.view_id,
            ),
            Self::Presence(message) => {
                validate_identifier(message.app_instance_id.as_str(), "presence.appInstanceId")
                    .map_err(UiProtocolError::InvalidView)?;
                validate_identifier(message.view_id.as_str(), "presence.viewId")
                    .map_err(UiProtocolError::InvalidView)?;
                for (index, member) in message.members.iter().enumerate() {
                    validate_participant(
                        &member.participant,
                        &format!("presence.members[{index}]"),
                    )?;
                    validate_identifier(
                        member.client_id.as_str(),
                        &format!("presence.members[{index}].clientId"),
                    )
                    .map_err(UiProtocolError::InvalidView)?;
                    validate_renderer(
                        &member.renderer,
                        &format!("presence.members[{index}].renderer"),
                    )?;
                }
                Ok(())
            }
            Self::Error(message) => {
                validate_identifier(&message.code, "error.code")
                    .map_err(UiProtocolError::InvalidView)?;
                if message.message.trim().is_empty() {
                    Err(UiProtocolError::InvalidMessage(
                        "error message must not be empty".to_owned(),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

macro_rules! impl_message_from {
    ($value:ty, $variant:ident) => {
        impl From<$value> for UiMessage {
            fn from(value: $value) -> Self {
                Self::$variant(value)
            }
        }
    };
}

impl_message_from!(UiAttach, Attach);
impl_message_from!(UiAttached, Attached);
impl_message_from!(UiSnapshot, Snapshot);
impl_message_from!(UiDelta, Delta);
impl_message_from!(UiEvent, Event);
impl_message_from!(UiAck, Ack);
impl_message_from!(UiLifecycle, Lifecycle);
impl_message_from!(UiRequestSnapshot, RequestSnapshot);
impl_message_from!(UiPresence, Presence);
impl_message_from!(UiErrorMessage, Error);

/// A precise component validation failure with its semantic path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiValidationError {
    pub path: String,
    pub message: String,
}

impl UiValidationError {
    #[must_use]
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for UiValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for UiValidationError {}

/// Errors raised while decoding, validating, or writing protocol frames.
#[derive(Debug)]
pub enum UiProtocolError {
    Io(io::Error),
    Json(serde_json::Error),
    EmptyFrame,
    FrameTooLarge {
        max_bytes: usize,
    },
    InvalidView(UiValidationError),
    InvalidMessage(String),
    UnexpectedProtocol {
        expected: &'static str,
        received: String,
    },
    UnsupportedVersion {
        minimum: u32,
        maximum: u32,
        received: u32,
    },
}

impl fmt::Display for UiProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "UI protocol I/O error: {error}"),
            Self::Json(error) => write!(formatter, "invalid UI protocol JSON: {error}"),
            Self::EmptyFrame => formatter.write_str("UI protocol frame is empty"),
            Self::FrameTooLarge { max_bytes } => {
                write!(
                    formatter,
                    "UI protocol frame exceeds the {max_bytes}-byte limit"
                )
            }
            Self::InvalidView(error) => write!(formatter, "invalid App Kit view: {error}"),
            Self::InvalidMessage(message) => {
                write!(formatter, "invalid UI protocol message: {message}")
            }
            Self::UnexpectedProtocol { expected, received } => write!(
                formatter,
                "unexpected UI protocol {received:?}; expected {expected:?}"
            ),
            Self::UnsupportedVersion {
                minimum,
                maximum,
                received,
            } if minimum == maximum => write!(
                formatter,
                "unsupported UI protocol version {received}; supported version is {minimum}"
            ),
            Self::UnsupportedVersion {
                minimum,
                maximum,
                received,
            } => write!(
                formatter,
                "unsupported UI protocol version {received}; supported range is {minimum}..={maximum}"
            ),
        }
    }
}

impl std::error::Error for UiProtocolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::InvalidView(error) => Some(error),
            Self::EmptyFrame
            | Self::FrameTooLarge { .. }
            | Self::InvalidMessage(_)
            | Self::UnexpectedProtocol { .. }
            | Self::UnsupportedVersion { .. } => None,
        }
    }
}

impl From<io::Error> for UiProtocolError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for UiProtocolError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

/// Read and validate one newline-delimited message. `None` is a clean EOF.
pub fn read_ui_message<R: BufRead>(reader: &mut R) -> Result<Option<UiMessage>, UiProtocolError> {
    let mut frame = Vec::new();
    let mut limited = reader.take(MAX_UI_FRAME_BYTES as u64 + 2);
    if limited.read_until(b'\n', &mut frame)? == 0 {
        return Ok(None);
    }
    if frame.ends_with(b"\n") {
        frame.pop();
        if frame.ends_with(b"\r") {
            frame.pop();
        }
    }
    decode_ui_frame(&frame).map(Some)
}

/// Decode and validate one JSON payload without its NDJSON newline.
pub fn decode_ui_frame(frame: &[u8]) -> Result<UiMessage, UiProtocolError> {
    if frame.len() > MAX_UI_FRAME_BYTES {
        return Err(UiProtocolError::FrameTooLarge {
            max_bytes: MAX_UI_FRAME_BYTES,
        });
    }
    if frame.iter().all(u8::is_ascii_whitespace) {
        return Err(UiProtocolError::EmptyFrame);
    }

    let header: MessageHeader = serde_json::from_slice(frame)?;
    validate_protocol_name(&header.protocol)?;
    if header.message_type != "attach" {
        let version = header.protocol_version.ok_or_else(|| {
            UiProtocolError::InvalidMessage(
                "non-attach messages require protocolVersion".to_owned(),
            )
        })?;
        validate_header(&header.protocol, version)?;
    }
    let message: UiMessage = serde_json::from_slice(frame)?;
    message.validate()?;
    Ok(message)
}

/// Encode and validate one compact NDJSON frame, including its newline.
pub fn encode_ui_frame(message: &UiMessage) -> Result<Vec<u8>, UiProtocolError> {
    message.validate()?;
    let mut frame = serde_json::to_vec(message)?;
    if frame.len() > MAX_UI_FRAME_BYTES {
        return Err(UiProtocolError::FrameTooLarge {
            max_bytes: MAX_UI_FRAME_BYTES,
        });
    }
    frame.push(b'\n');
    Ok(frame)
}

/// Write one compact JSON frame and flush it for interactive side-channel use.
pub fn write_ui_message<W: Write>(
    writer: &mut W,
    message: &UiMessage,
) -> Result<(), UiProtocolError> {
    writer.write_all(&encode_ui_frame(message)?)?;
    writer.flush()?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageHeader {
    #[serde(rename = "type")]
    message_type: String,
    protocol: String,
    #[serde(default)]
    protocol_version: Option<u32>,
}

fn validate_protocol_name(protocol: &str) -> Result<(), UiProtocolError> {
    if protocol != UI_PROTOCOL_NAME {
        return Err(UiProtocolError::UnexpectedProtocol {
            expected: UI_PROTOCOL_NAME,
            received: protocol.to_owned(),
        });
    }
    Ok(())
}

fn validate_header(protocol: &str, version: u32) -> Result<(), UiProtocolError> {
    validate_protocol_name(protocol)?;
    if !(UI_PROTOCOL_MIN_VERSION..=UI_PROTOCOL_MAX_VERSION).contains(&version) {
        return Err(UiProtocolError::UnsupportedVersion {
            minimum: UI_PROTOCOL_MIN_VERSION,
            maximum: UI_PROTOCOL_MAX_VERSION,
            received: version,
        });
    }
    Ok(())
}

fn validate_protocol_range(
    minimum: u32,
    maximum: u32,
    context: &str,
) -> Result<(), UiProtocolError> {
    if minimum == 0 {
        return Err(UiProtocolError::InvalidMessage(format!(
            "{context} minProtocolVersion must be at least 1"
        )));
    }
    if minimum > maximum {
        return Err(UiProtocolError::InvalidMessage(format!(
            "{context} minProtocolVersion must not exceed maxProtocolVersion"
        )));
    }
    Ok(())
}

/// Selects the highest protocol version shared with this App Kit build.
#[must_use]
pub const fn negotiate_ui_protocol_version(minimum: u32, maximum: u32) -> Option<u32> {
    if minimum == 0 || minimum > maximum {
        return None;
    }
    let shared_minimum = if minimum > UI_PROTOCOL_MIN_VERSION {
        minimum
    } else {
        UI_PROTOCOL_MIN_VERSION
    };
    let shared_maximum = if maximum < UI_PROTOCOL_MAX_VERSION {
        maximum
    } else {
        UI_PROTOCOL_MAX_VERSION
    };
    if shared_minimum <= shared_maximum {
        Some(shared_maximum)
    } else {
        None
    }
}

fn validate_revision(revision: u64) -> Result<(), UiProtocolError> {
    if revision > MAX_SAFE_UI_INTEGER {
        Err(UiProtocolError::InvalidMessage(format!(
            "revision exceeds the cross-platform safe integer {MAX_SAFE_UI_INTEGER}"
        )))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_identifier_for_protocol(
    value: &str,
    path: &str,
) -> Result<(), UiValidationError> {
    if value.is_empty() || value.len() > 256 {
        return Err(UiValidationError::new(
            path,
            "identifier must contain 1..=256 bytes",
        ));
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
    }) {
        return Err(UiValidationError::new(
            path,
            "identifier contains unsupported characters",
        ));
    }
    Ok(())
}

fn validate_identifier(value: &str, path: &str) -> Result<(), UiValidationError> {
    validate_identifier_for_protocol(value, path)
}

fn validate_app(app: &AppMetadata) -> Result<(), UiProtocolError> {
    for (field, value) in [
        ("app.id", app.id.as_str()),
        ("app.name", app.name.as_str()),
        ("app.version", app.version.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(UiProtocolError::InvalidMessage(format!(
                "{field} must not be empty"
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_participant_for_protocol(
    participant: &UiParticipant,
    path: &str,
) -> Result<(), UiProtocolError> {
    validate_identifier(participant.id.as_str(), &format!("{path}.id"))
        .map_err(UiProtocolError::InvalidView)?;
    if let Some(source_session_id) = &participant.source_session_id {
        validate_identifier(source_session_id, &format!("{path}.sourceSessionId"))
            .map_err(UiProtocolError::InvalidView)?;
    }
    if participant
        .display_name
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(UiProtocolError::InvalidMessage(format!(
            "{path}.displayName must not be empty"
        )));
    }
    for (index, grant) in participant.grants.iter().enumerate() {
        if grant.as_str() != UiGrant::ALL {
            validate_identifier(grant.as_str(), &format!("{path}.grants[{index}]"))
                .map_err(UiProtocolError::InvalidView)?;
        }
    }
    Ok(())
}

fn validate_participant(participant: &UiParticipant, path: &str) -> Result<(), UiProtocolError> {
    validate_participant_for_protocol(participant, path)
}

fn validate_renderer(renderer: &UiRendererMetadata, path: &str) -> Result<(), UiProtocolError> {
    validate_identifier(renderer.id.as_str(), &format!("{path}.id"))
        .map_err(UiProtocolError::InvalidView)?;
    validate_identifier(&renderer.kind, &format!("{path}.kind"))
        .map_err(UiProtocolError::InvalidView)?;
    for (index, capability) in renderer.capabilities.iter().enumerate() {
        validate_identifier(capability, &format!("{path}.capabilities[{index}]"))
            .map_err(UiProtocolError::InvalidView)?;
    }
    Ok(())
}

fn validate_attach(message: &UiAttach) -> Result<(), UiProtocolError> {
    validate_protocol_range(
        message.min_protocol_version,
        message.max_protocol_version,
        "attach",
    )?;
    if message.participant_token.is_empty() || message.participant_token.len() > 16 * 1024 {
        return Err(UiProtocolError::InvalidMessage(
            "attach participantToken must contain 1..=16384 bytes".to_owned(),
        ));
    }
    validate_identifier(message.client_id.as_str(), "attach.clientId")
        .map_err(UiProtocolError::InvalidView)?;
    validate_renderer(&message.renderer, "attach.renderer")?;
    validate_identifier(message.view_id.as_str(), "attach.viewId")
        .map_err(UiProtocolError::InvalidView)?;
    if let Some(app_instance_id) = &message.expected_app_instance_id {
        validate_identifier(app_instance_id.as_str(), "attach.expectedAppInstanceId")
            .map_err(UiProtocolError::InvalidView)?;
    }
    if let Some(revision) = message.last_seen_revision {
        validate_revision(revision)?;
    }
    Ok(())
}

fn validate_delta(message: &UiDelta) -> Result<(), UiProtocolError> {
    validate_identifier(message.app_instance_id.as_str(), "delta.appInstanceId")
        .map_err(UiProtocolError::InvalidView)?;
    validate_identifier(message.client_id.as_str(), "delta.clientId")
        .map_err(UiProtocolError::InvalidView)?;
    validate_identifier(message.view_id.as_str(), "delta.viewId")
        .map_err(UiProtocolError::InvalidView)?;
    validate_revision(message.base_revision)?;
    validate_revision(message.revision)?;
    if message.revision <= message.base_revision {
        return Err(UiProtocolError::InvalidMessage(
            "delta revision must be greater than baseRevision".to_owned(),
        ));
    }
    if message.operations.is_empty() || message.operations.len() > 4096 {
        return Err(UiProtocolError::InvalidMessage(
            "delta operations must contain 1..=4096 entries".to_owned(),
        ));
    }
    for (index, operation) in message.operations.iter().enumerate() {
        operation.validate(&format!("delta.operations[{index}]"))?;
    }
    Ok(())
}

fn validate_session_route(
    app_instance_id: &AppInstanceId,
    client_id: &ClientId,
    renderer_id: &RendererId,
    view_id: &ViewId,
) -> Result<(), UiProtocolError> {
    for (path, value) in [
        ("appInstanceId", app_instance_id.as_str()),
        ("clientId", client_id.as_str()),
        ("rendererId", renderer_id.as_str()),
        ("viewId", view_id.as_str()),
    ] {
        validate_identifier(value, path).map_err(UiProtocolError::InvalidView)?;
    }
    Ok(())
}

fn validate_event(message: &UiEvent) -> Result<(), UiProtocolError> {
    validate_session_route(
        &message.app_instance_id,
        &message.client_id,
        &message.renderer_id,
        &message.view_id,
    )?;
    validate_identifier(message.participant_id.as_str(), "event.participantId")
        .map_err(UiProtocolError::InvalidView)?;
    validate_identifier(message.event_id.as_str(), "event.eventId")
        .map_err(UiProtocolError::InvalidView)?;
    validate_identifier(message.action.node_id.as_str(), "event.nodeId")
        .map_err(UiProtocolError::InvalidView)?;
    validate_identifier(message.action.action.as_str(), "event.action")
        .map_err(UiProtocolError::InvalidView)?;
    validate_revision(message.base_revision)?;
    validate_event_value(&message.action.value)
}

fn validate_action_set(
    actions: &MarkdownEditorActions,
    read_only: bool,
    path: &str,
) -> Result<(), UiValidationError> {
    for (name, action) in [
        ("replaceRange", actions.replace_range.as_ref()),
        ("setSelection", actions.set_selection.as_ref()),
        ("save", actions.save.as_ref()),
        ("undo", actions.undo.as_ref()),
        ("redo", actions.redo.as_ref()),
        ("setPresentation", actions.set_presentation.as_ref()),
        ("openMenu", actions.open_menu.as_ref()),
    ] {
        if let Some(action) = action {
            validate_identifier(action.as_str(), &format!("{path}.actions.{name}"))?;
        }
    }
    if read_only
        && (actions.replace_range.is_some()
            || actions.undo.is_some()
            || actions.redo.is_some()
            || actions.open_menu.is_some())
    {
        return Err(UiValidationError::new(
            format!("{path}.actions"),
            "read-only Markdown editors cannot declare editing actions",
        ));
    }
    Ok(())
}

fn validate_position(text: &str, position: TextPosition) -> Result<(), String> {
    let Some(line) = text.split('\n').nth(position.line as usize) else {
        return Err(format!("line {} is outside the document", position.line));
    };
    let target = position.utf16_column as usize;
    let mut current = 0usize;
    if target == 0 {
        return Ok(());
    }
    for character in line.chars() {
        current += character.len_utf16();
        if current == target {
            return Ok(());
        }
        if current > target {
            return Err(format!(
                "UTF-16 column {} splits a surrogate pair",
                position.utf16_column
            ));
        }
    }
    Err(format!(
        "UTF-16 column {} is outside line {}",
        position.utf16_column, position.line
    ))
}

fn text_position_byte_offset(text: &str, position: TextPosition) -> Result<usize, String> {
    let mut line_start = 0usize;
    for _ in 0..position.line {
        let Some(relative_newline) = text[line_start..].find('\n') else {
            return Err(format!("line {} is outside the document", position.line));
        };
        line_start += relative_newline + 1;
    }
    let line_end = text[line_start..]
        .find('\n')
        .map_or(text.len(), |offset| line_start + offset);
    let line = &text[line_start..line_end];
    let target = usize::try_from(position.utf16_column)
        .map_err(|_| "UTF-16 column does not fit this platform".to_owned())?;
    if target == 0 {
        return Ok(line_start);
    }
    let mut utf16_column = 0usize;
    for (byte_offset, character) in line.char_indices() {
        utf16_column += character.len_utf16();
        if utf16_column == target {
            return Ok(line_start + byte_offset + character.len_utf8());
        }
        if utf16_column > target {
            return Err(format!(
                "UTF-16 column {} splits a surrogate pair",
                position.utf16_column
            ));
        }
    }
    Err(format!(
        "UTF-16 column {} is outside line {}",
        position.utf16_column, position.line
    ))
}

fn apply_text_edit(text: &mut String, edit: &TextEdit) -> Result<(), String> {
    if edit.range.start > edit.range.end {
        return Err("text edit range is reversed".to_owned());
    }
    let start = text_position_byte_offset(text, edit.range.start)?;
    let end = text_position_byte_offset(text, edit.range.end)?;
    text.replace_range(start..end, &edit.text);
    Ok(())
}

fn validate_event_value(value: &UiEventValue) -> Result<(), UiProtocolError> {
    match value {
        UiEventValue::Index(value) if *value > MAX_SAFE_UI_INTEGER => {
            Err(UiProtocolError::InvalidMessage(format!(
                "event index exceeds the cross-platform safe integer {MAX_SAFE_UI_INTEGER}"
            )))
        }
        UiEventValue::Integer(value)
            if *value < -(MAX_SAFE_UI_INTEGER as i64) || *value > MAX_SAFE_UI_INTEGER as i64 =>
        {
            Err(UiProtocolError::InvalidMessage(format!(
                "event integer exceeds the cross-platform safe range ±{MAX_SAFE_UI_INTEGER}"
            )))
        }
        UiEventValue::Number(value) if !value.is_finite() => Err(UiProtocolError::InvalidMessage(
            "event number must be finite".to_owned(),
        )),
        UiEventValue::TextEdit(edit) if edit.range.start > edit.range.end => Err(
            UiProtocolError::InvalidMessage("text edit range is reversed".to_owned()),
        ),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{Input, List, ListItem, ListItemSlot, Toggle};

    use super::*;

    fn route() -> (AppInstanceId, ParticipantId, ClientId, RendererId, ViewId) {
        (
            "app-123".into(),
            "person-1".into(),
            "client-1".into(),
            "renderer-1".into(),
            "main".into(),
        )
    }

    fn markdown_snapshot() -> UiSnapshot {
        let selection = TextSelection::caret(TextPosition::new(1, 2));
        UiSnapshot::new(
            "app-123",
            "client-1",
            "main",
            7,
            UiNode::markdown_editor(
                "editor",
                MarkdownEditorSpec::new("# Hello\n🙂 world", selection)
                    .dirty(true)
                    .title("README.md"),
            ),
        )
    }

    fn todo_snapshot() -> UiSnapshot {
        let item = ListItem::new("todo-1", "First")
            .trailing(ListItemSlot::toggle(Toggle::new(
                "todo-1-toggle",
                "Completed",
                false,
                "set-done",
            )))
            .delete_action("delete-todo");
        UiSnapshot::new(
            "app-123",
            "client-1",
            "main",
            1,
            UiNode::page(
                "todo-page",
                Page::new("Todos", List::new("todos", vec![item])).input(
                    Input::new("new-todo", "New todo")
                        .placeholder("What needs doing?")
                        .submit_action("add-todo"),
                ),
            ),
        )
    }

    #[test]
    fn attach_redacts_the_participant_token_and_round_trips() {
        let attach = UiAttach::new(
            "scoped-participant-secret",
            "client-1",
            UiRendererMetadata::new("renderer-1", "web"),
            "main",
        )
        .state(UiRendererState::component());
        assert!(!format!("{attach:?}").contains("scoped-participant-secret"));
        let message = UiMessage::Attach(attach);
        let frame = encode_ui_frame(&message).unwrap();
        let encoded = std::str::from_utf8(&frame).unwrap();
        assert!(encoded.contains(r#""minProtocolVersion":1"#));
        assert!(encoded.contains(r#""maxProtocolVersion":1"#));
        assert!(!encoded.contains(r#""protocolVersion""#));
        assert_eq!(decode_ui_frame(&frame[..frame.len() - 1]).unwrap(), message);
    }

    #[test]
    fn markdown_projection_diff_syncs_unicode_text_and_multiline_selection() {
        let previous = UiNode::markdown_editor(
            "editor",
            MarkdownEditorSpec::new(
                "alpha 🙂\nbeta",
                TextSelection::caret(TextPosition::new(0, 0)),
            ),
        );
        let next = UiNode::markdown_editor(
            "editor",
            MarkdownEditorSpec::new(
                "alpha brave 🙂\nbeta!",
                TextSelection {
                    anchor: TextPosition::new(0, 6),
                    head: TextPosition::new(1, 5),
                },
            )
            .dirty(true),
        );
        let operations = markdown_delta_operations(&previous, &next);
        assert!(matches!(
            operations.first(),
            Some(UiDeltaOperation::MarkdownReplaceRange { .. })
        ));
        assert!(operations.iter().any(|operation| matches!(
            operation,
            UiDeltaOperation::MarkdownSetSelection { selection, .. }
                if selection.anchor.line == 0 && selection.head.line == 1
        )));

        let mut applied = previous;
        applied.apply_delta_operations(&operations).unwrap();
        assert_eq!(applied, next);
    }

    #[test]
    fn attach_ranges_negotiate_without_requiring_strict_version_equality() {
        assert_eq!(negotiate_ui_protocol_version(1, 3), Some(1));
        assert_eq!(negotiate_ui_protocol_version(2, 3), None);
        assert_eq!(negotiate_ui_protocol_version(3, 2), None);

        let attach = UiAttach::new(
            "scoped-participant-secret",
            "client-1",
            UiRendererMetadata::new("renderer-1", "web"),
            "main",
        )
        .protocol_versions(2, 3);
        let message = UiMessage::Attach(attach);
        let frame = encode_ui_frame(&message).unwrap();
        assert_eq!(decode_ui_frame(&frame[..frame.len() - 1]).unwrap(), message);
    }

    #[test]
    fn recognized_messages_ignore_unknown_fields_but_unknown_kinds_fail() {
        let mut value = serde_json::to_value(UiMessage::Snapshot(markdown_snapshot())).unwrap();
        {
            let message = value.as_object_mut().unwrap();
            message.insert(
                "futureEnvelopeField".to_owned(),
                serde_json::json!({ "v": 2 }),
            );
            let root = message.get_mut("root").unwrap().as_object_mut().unwrap();
            root.insert("futureComponentField".to_owned(), serde_json::json!(true));
            root.get_mut("selection")
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(
                    "futureSelectionField".to_owned(),
                    serde_json::json!("ignored"),
                );
        }

        assert!(decode_ui_frame(&serde_json::to_vec(&value).unwrap()).is_ok());

        value
            .get_mut("root")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("type".to_owned(), serde_json::json!("futureEditor"));
        assert!(matches!(
            decode_ui_frame(&serde_json::to_vec(&value).unwrap()),
            Err(UiProtocolError::Json(_))
        ));
    }

    #[test]
    fn markdown_snapshot_round_trips_with_projection_identity() {
        let message = UiMessage::Snapshot(markdown_snapshot());
        let encoded = serde_json::to_string(&message).unwrap();
        assert!(encoded.contains(r#""type":"markdownEditor""#));
        assert!(encoded.contains(r#""appInstanceId":"app-123""#));
        assert!(encoded.contains(r#""utf16Column":2"#));
        assert_eq!(decode_ui_frame(encoded.as_bytes()).unwrap(), message);
    }

    #[test]
    fn page_deltas_update_only_the_closed_component_slots() {
        let second = ListItem::new("todo-2", "Second").trailing(ListItemSlot::toggle(Toggle::new(
            "todo-2-toggle",
            "Completed",
            false,
            "set-done",
        )));
        let delta = UiDelta::new(
            "app-123",
            "client-1",
            "main",
            1,
            2,
            vec![
                UiDeltaOperation::input_set_value("new-todo", "draft"),
                UiDeltaOperation::toggle_set_value("todo-1-toggle", true),
                UiDeltaOperation::list_insert_item("todos", 1, second),
                UiDeltaOperation::list_remove_item("todos", "todo-1"),
                UiDeltaOperation::list_set_selection("todos", Some("todo-2".to_owned())),
            ],
        );
        let updated = todo_snapshot().applying(&delta).unwrap();
        let UiComponent::Page(page) = updated.root.element else {
            panic!("Page operations must preserve the component kind");
        };
        assert_eq!(page.input_spec().unwrap().value, "draft");
        assert_eq!(page.list().items.len(), 1);
        assert_eq!(page.list().items[0].id, "todo-2");
        assert_eq!(page.list().selected_id.as_deref(), Some("todo-2"));
    }

    #[test]
    fn text_edit_events_carry_multi_user_and_idempotency_context() {
        let (app, participant, client, renderer, view) = route();
        let edit = TextEdit::new(
            TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 2)),
            "hello",
        );
        let event = UiEvent::new(
            app,
            participant,
            client,
            renderer,
            view,
            "event-1",
            7,
            UiAction::replace_range("editor", edit.clone()),
        );
        assert_eq!(event.event_id.as_str(), "event-1");
        assert_eq!(event.action.value, UiEventValue::TextEdit(edit));
        UiMessage::Event(event).validate().unwrap();
    }

    #[test]
    fn validation_rejects_surrogate_splits_and_read_only_edit_actions() {
        let split = UiSnapshot::new(
            "app-123",
            "client-1",
            "main",
            1,
            UiNode::markdown_editor(
                "editor",
                MarkdownEditorSpec::new("🙂", TextSelection::caret(TextPosition::new(0, 1))),
            ),
        );
        assert!(matches!(
            UiMessage::Snapshot(split).validate(),
            Err(UiProtocolError::InvalidView(_))
        ));

        let mut editor =
            MarkdownEditorSpec::new("safe", TextSelection::caret(TextPosition::new(0, 0)));
        editor.read_only = true;
        assert!(matches!(
            UiMessage::Snapshot(UiSnapshot::new(
                "app-123",
                "client-1",
                "main",
                1,
                UiNode::markdown_editor("editor", editor),
            ))
            .validate(),
            Err(UiProtocolError::InvalidView(_))
        ));
    }

    #[test]
    fn framing_rejects_other_versions_and_oversized_safe_integers() {
        let unsupported = br#"{"type":"error","protocol":"unpeel.ui","protocolVersion":2,"code":"test","message":"test"}"#;
        assert!(matches!(
            decode_ui_frame(unsupported),
            Err(UiProtocolError::UnsupportedVersion { received: 2, .. })
        ));

        let mut oversized = markdown_snapshot();
        oversized.revision = MAX_SAFE_UI_INTEGER + 1;
        assert!(matches!(
            write_ui_message(&mut Vec::new(), &UiMessage::Snapshot(oversized)),
            Err(UiProtocolError::InvalidMessage(_))
        ));

        let frame = encode_ui_frame(&UiMessage::Snapshot(markdown_snapshot())).unwrap();
        let mut reader = Cursor::new(frame);
        assert!(read_ui_message(&mut reader).unwrap().is_some());
        assert!(read_ui_message(&mut reader).unwrap().is_none());
    }
}
