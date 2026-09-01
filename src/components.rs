//! Closed, standalone-first components for data and document Apps.
//!
//! The same owned values render through Ratatui and, when `ui-bridge` is
//! enabled, serialize into the semantic component channel. Containment stays
//! deliberately slot-based: a [`List`] owns only [`ListItem`] values, and a
//! row slot accepts only the controls enumerated by [`ListItemSlot`].

#![cfg_attr(not(feature = "ui-bridge"), allow(dead_code))]

use std::collections::HashSet;
use std::fmt;

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

use crate::{
    InputField, KitTheme, ListPageBehavior, ListState, SELECTABLE_LEFT_PADDING, SelectableRow,
    VerticalScrollbar,
};

/// Renderer capability for the v1 Page container.
pub const PAGE_COMPONENT_CAPABILITY: &str = "page";
/// Renderer capability for the v1 List container.
pub const LIST_COMPONENT_CAPABILITY: &str = "list";
/// Renderer capability for the v1 ListItem row.
pub const LIST_ITEM_COMPONENT_CAPABILITY: &str = "listItem";
/// Renderer capability for ListItem subtitle/trailing-value metadata.
pub const LIST_ITEM_METADATA_CAPABILITY: &str = "listItemMetadata";
/// Renderer capability for an activatable ListItem row.
pub const LIST_ITEM_ACTIVATE_CAPABILITY: &str = "listItemActivate";
/// Renderer capability for status/badge/busy ListItem presentation.
pub const LIST_ITEM_PRESENTATION_CAPABILITY: &str = "listItemPresentation";
/// Renderer capability for authoritative selection and shared list navigation.
pub const LIST_SELECTION_CAPABILITY: &str = "listSelection";
/// Renderer capability for static leading status symbols.
pub const STATUS_SYMBOL_COMPONENT_CAPABILITY: &str = "statusSymbol";
/// Renderer capability for compact ListItem badges.
pub const BADGE_COMPONENT_CAPABILITY: &str = "badge";
/// Renderer capability for the v1 Toggle control.
pub const TOGGLE_COMPONENT_CAPABILITY: &str = "toggle";
/// Renderer capability for the v1 Input control.
pub const INPUT_COMPONENT_CAPABILITY: &str = "input";
/// Renderer capability for the v1 Button control.
pub const BUTTON_COMPONENT_CAPABILITY: &str = "button";
/// Renderer capability for Page-level back navigation.
pub const PAGE_BACK_CAPABILITY: &str = "pageBack";

const MAX_ITEMS: usize = 100_000;
const MAX_SHORT_TEXT_BYTES: usize = 4 * 1024;
const MAX_LABEL_BYTES: usize = 16 * 1024;
const MAX_INPUT_BYTES: usize = 1024 * 1024;

/// Visual intent for a semantic [`Button`]. The renderer chooses its native
/// platform treatment; these are not arbitrary style tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ButtonRole {
    #[default]
    Default,
    Primary,
    Destructive,
}

/// One semantic action control accepted by named component slots.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Button {
    pub id: String,
    pub label: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "is_default_button_role")]
    pub role: ButtonRole,
}

impl Button {
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            action: action.into(),
            role: ButtonRole::Default,
        }
    }

    #[must_use]
    pub const fn role(mut self, role: ButtonRole) -> Self {
        self.role = role;
        self
    }

    pub(crate) fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        validate_identifier(&self.id, &format!("{path}.id"))?;
        validate_text(&self.label, MAX_SHORT_TEXT_BYTES, &format!("{path}.label"))?;
        validate_identifier(&self.action, &format!("{path}.action"))
    }
}

const fn is_default_button_role(role: &ButtonRole) -> bool {
    matches!(role, ButtonRole::Default)
}

/// Boolean control embeddable in an explicitly declared row slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Toggle {
    pub id: String,
    pub label: String,
    pub value: bool,
    pub set_value: String,
}

impl Toggle {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        value: bool,
        set_value: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            set_value: set_value.into(),
        }
    }

    /// Compact terminal marker used inside a row.
    #[must_use]
    pub const fn marker(&self) -> &'static str {
        if self.value { "[x]" } else { "[ ]" }
    }

    pub(crate) fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        validate_identifier(&self.id, &format!("{path}.id"))?;
        validate_text(&self.label, MAX_SHORT_TEXT_BYTES, &format!("{path}.label"))?;
        validate_identifier(&self.set_value, &format!("{path}.setValue"))
    }
}

/// Semantic foreground treatment shared by terminal, native, and web rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListItemTone {
    #[default]
    Default,
    Muted,
    Accent,
    Info,
    Success,
    Warning,
    Danger,
}

/// Semantic label weight; arbitrary font/style values stay out of the wire vocabulary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListItemEmphasis {
    #[default]
    Regular,
    Strong,
}

/// Compact leading state such as `M`, `A`, `D`, or an issue number/state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusSymbol {
    pub symbol: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "is_default_list_item_tone")]
    pub tone: ListItemTone,
    #[serde(default, skip_serializing_if = "is_default_list_item_emphasis")]
    pub emphasis: ListItemEmphasis,
    #[serde(default)]
    pub preserve_tone_when_selected: bool,
}

impl StatusSymbol {
    #[must_use]
    pub fn new(symbol: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            label: label.into(),
            tone: ListItemTone::Default,
            emphasis: ListItemEmphasis::Regular,
            preserve_tone_when_selected: false,
        }
    }

    #[must_use]
    pub const fn tone(mut self, tone: ListItemTone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub const fn emphasis(mut self, emphasis: ListItemEmphasis) -> Self {
        self.emphasis = emphasis;
        self
    }

    #[must_use]
    pub const fn preserve_tone_when_selected(mut self, preserve: bool) -> Self {
        self.preserve_tone_when_selected = preserve;
        self
    }

    fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        validate_text(
            &self.symbol,
            MAX_SHORT_TEXT_BYTES,
            &format!("{path}.symbol"),
        )?;
        if self.symbol.is_empty() || self.symbol.contains(['\n', '\r']) {
            return Err(ComponentValidationError::new(
                format!("{path}.symbol"),
                "status symbol must be a non-empty single line",
            ));
        }
        validate_text(&self.label, MAX_SHORT_TEXT_BYTES, &format!("{path}.label"))
    }
}

/// Compact inline metadata shown beside a row label.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Badge {
    pub text: String,
    #[serde(default, skip_serializing_if = "is_default_list_item_tone")]
    pub tone: ListItemTone,
}

impl Badge {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: ListItemTone::Muted,
        }
    }

    #[must_use]
    pub const fn tone(mut self, tone: ListItemTone) -> Self {
        self.tone = tone;
        self
    }

    fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        validate_text(&self.text, MAX_SHORT_TEXT_BYTES, &format!("{path}.text"))?;
        if self.text.contains(['\n', '\r']) {
            return Err(ComponentValidationError::new(
                format!("{path}.text"),
                "badge text must be a single line",
            ));
        }
        Ok(())
    }
}

const fn is_default_list_item_tone(tone: &ListItemTone) -> bool {
    matches!(tone, ListItemTone::Default)
}

const fn is_default_list_item_emphasis(emphasis: &ListItemEmphasis) -> bool {
    matches!(emphasis, ListItemEmphasis::Regular)
}

const fn default_value_tone() -> ListItemTone {
    ListItemTone::Muted
}

const fn is_default_value_tone(tone: &ListItemTone) -> bool {
    matches!(tone, ListItemTone::Muted)
}

/// Controls currently allowed in a ListItem's named slots.
///
/// This enum grows deliberately only when a control has defined terminal,
/// native, web, and agent semantics. Arbitrary child nodes are never accepted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ListItemSlot {
    Toggle(Toggle),
    Status(StatusSymbol),
    Badge(Badge),
}

impl ListItemSlot {
    #[must_use]
    pub const fn toggle(toggle: Toggle) -> Self {
        Self::Toggle(toggle)
    }

    #[must_use]
    pub const fn status(status: StatusSymbol) -> Self {
        Self::Status(status)
    }

    #[must_use]
    pub const fn badge(badge: Badge) -> Self {
        Self::Badge(badge)
    }

    #[must_use]
    pub const fn id(&self) -> Option<&str> {
        match self {
            Self::Toggle(toggle) => Some(toggle.id.as_str()),
            Self::Status(_) | Self::Badge(_) => None,
        }
    }

    pub(crate) fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        match self {
            Self::Toggle(toggle) => toggle.validate(path),
            Self::Status(status) => status.validate(path),
            Self::Badge(badge) => badge.validate(path),
        }
    }

    fn toggle_mut(&mut self, id: &str) -> Option<&mut Toggle> {
        match self {
            Self::Toggle(toggle) if toggle.id == id => Some(toggle),
            Self::Toggle(_) | Self::Status(_) | Self::Badge(_) => None,
        }
    }

    fn as_toggle(&self) -> Option<&Toggle> {
        match self {
            Self::Toggle(toggle) => Some(toggle),
            Self::Status(_) | Self::Badge(_) => None,
        }
    }
}

/// One keyed, semantically meaningful row in a [`List`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "is_default_list_item_tone")]
    pub label_tone: ListItemTone,
    #[serde(default, skip_serializing_if = "is_default_list_item_emphasis")]
    pub emphasis: ListItemEmphasis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(
        default = "default_value_tone",
        skip_serializing_if = "is_default_value_tone"
    )]
    pub value_tone: ListItemTone,
    /// Minimum complete row width at which the trailing value is retained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_min_width: Option<u16>,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub busy: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading: Option<ListItemSlot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trailing: Option<ListItemSlot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessory: Option<ListItemSlot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activate: Option<String>,
}

impl ListItem {
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            label_tone: ListItemTone::Default,
            emphasis: ListItemEmphasis::Regular,
            detail: None,
            value: None,
            value_tone: ListItemTone::Muted,
            value_min_width: None,
            done: false,
            busy: false,
            leading: None,
            trailing: None,
            accessory: None,
            delete: None,
            activate: None,
        }
    }

    /// Secondary row text, rendered beneath the label natively and inline in
    /// compact terminal lists.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Trailing read-only value such as a quota, date, or status.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    #[must_use]
    pub const fn label_tone(mut self, tone: ListItemTone) -> Self {
        self.label_tone = tone;
        self
    }

    #[must_use]
    pub const fn emphasis(mut self, emphasis: ListItemEmphasis) -> Self {
        self.emphasis = emphasis;
        self
    }

    #[must_use]
    pub const fn value_tone(mut self, tone: ListItemTone) -> Self {
        self.value_tone = tone;
        self
    }

    /// Drops the trailing value below this total terminal-row width.
    #[must_use]
    pub const fn value_min_width(mut self, columns: u16) -> Self {
        self.value_min_width = Some(columns);
        self
    }

    #[must_use]
    pub const fn done(mut self, done: bool) -> Self {
        self.done = done;
        self
    }

    #[must_use]
    pub const fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    #[must_use]
    pub fn leading(mut self, slot: ListItemSlot) -> Self {
        self.leading = Some(slot);
        self
    }

    #[must_use]
    pub fn trailing(mut self, slot: ListItemSlot) -> Self {
        self.trailing = Some(slot);
        self
    }

    #[must_use]
    pub fn accessory(mut self, slot: ListItemSlot) -> Self {
        self.accessory = Some(slot);
        self
    }

    #[must_use]
    pub fn delete_action(mut self, action: impl Into<String>) -> Self {
        self.delete = Some(action.into());
        self
    }

    /// Declares the idempotent action used to open or select this row.
    #[must_use]
    pub fn activate_action(mut self, action: impl Into<String>) -> Self {
        self.activate = Some(action.into());
        self
    }

    pub(crate) fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        validate_identifier(&self.id, &format!("{path}.id"))?;
        validate_text(&self.label, MAX_LABEL_BYTES, &format!("{path}.label"))?;
        validate_single_line(&self.label, &format!("{path}.label"))?;
        if let Some(detail) = &self.detail {
            validate_text(detail, MAX_LABEL_BYTES, &format!("{path}.detail"))?;
            validate_single_line(detail, &format!("{path}.detail"))?;
        }
        if let Some(value) = &self.value {
            validate_text(value, MAX_SHORT_TEXT_BYTES, &format!("{path}.value"))?;
            validate_single_line(value, &format!("{path}.value"))?;
        }
        for (name, slot) in [
            ("leading", self.leading.as_ref()),
            ("trailing", self.trailing.as_ref()),
            ("accessory", self.accessory.as_ref()),
        ] {
            if let Some(slot) = slot {
                slot.validate(&format!("{path}.{name}"))?;
            }
        }
        let toggles = self
            .slots()
            .filter_map(ListItemSlot::as_toggle)
            .collect::<Vec<_>>();
        if toggles.len() > 1 {
            return Err(ComponentValidationError::new(
                format!("{path}.slots"),
                "ListItem v1 accepts at most one completion Toggle",
            ));
        }
        if let Some(toggle) = toggles.first()
            && toggle.value != self.done
        {
            return Err(ComponentValidationError::new(
                format!("{path}.done"),
                "done must match the completion Toggle value",
            ));
        }
        if let Some(action) = &self.delete {
            validate_identifier(action, &format!("{path}.delete"))?;
        }
        if let Some(action) = &self.activate {
            validate_identifier(action, &format!("{path}.activate"))?;
        }
        Ok(())
    }

    fn slots(&self) -> impl Iterator<Item = &ListItemSlot> {
        [
            self.leading.as_ref(),
            self.trailing.as_ref(),
            self.accessory.as_ref(),
        ]
        .into_iter()
        .flatten()
    }

    fn slots_mut(&mut self) -> impl Iterator<Item = &mut ListItemSlot> {
        [
            self.leading.as_mut(),
            self.trailing.as_mut(),
            self.accessory.as_mut(),
        ]
        .into_iter()
        .flatten()
    }

    fn set_toggle_value(&mut self, id: &str, value: bool) -> bool {
        let mut found = false;
        for slot in self.slots_mut() {
            if let Some(toggle) = slot.toggle_mut(id) {
                toggle.value = value;
                found = true;
                break;
            }
        }
        if found {
            self.done = value;
        }
        found
    }
}

/// A keyed collection that contains only [`ListItem`] rows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub id: String,
    pub items: Vec<ListItem>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub empty_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub scroll_padding: u16,
    #[serde(
        default = "default_page_overlap",
        skip_serializing_if = "is_default_page_overlap"
    )]
    pub page_overlap: u16,
    #[serde(default, skip_serializing_if = "is_default_page_behavior")]
    pub page_behavior: ListPageBehavior,
    #[serde(default)]
    pub space_pages_down: bool,
}

impl List {
    #[must_use]
    pub fn new(id: impl Into<String>, items: Vec<ListItem>) -> Self {
        Self {
            id: id.into(),
            items,
            empty_message: String::new(),
            selected_id: None,
            select: None,
            scroll_padding: 0,
            page_overlap: default_page_overlap(),
            page_behavior: ListPageBehavior::Selection,
            space_pages_down: false,
        }
    }

    #[must_use]
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = message.into();
        self
    }

    #[must_use]
    pub fn selected(mut self, item_id: impl Into<String>, action: impl Into<String>) -> Self {
        self.selected_id = Some(item_id.into());
        self.select = Some(action.into());
        self
    }

    #[must_use]
    pub const fn scroll_padding(mut self, rows: u16) -> Self {
        self.scroll_padding = rows;
        self
    }

    #[must_use]
    pub const fn page_overlap(mut self, rows: u16) -> Self {
        self.page_overlap = rows;
        self
    }

    #[must_use]
    pub const fn page_behavior(mut self, behavior: ListPageBehavior) -> Self {
        self.page_behavior = behavior;
        self
    }

    #[must_use]
    pub const fn space_pages_down(mut self, enabled: bool) -> Self {
        self.space_pages_down = enabled;
        self
    }

    /// Renders the same single-line row language used by the sibling Apps.
    #[must_use]
    pub fn widget<'a>(&'a self, state: &'a mut ListState) -> ListWidget<'a> {
        ListWidget {
            list: self,
            state,
            theme: PageTheme::default(),
        }
    }

    fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        validate_identifier(&self.id, &format!("{path}.id"))?;
        if self.items.len() > MAX_ITEMS {
            return Err(ComponentValidationError::new(
                format!("{path}.items"),
                format!("List may contain at most {MAX_ITEMS} items"),
            ));
        }
        validate_text(
            &self.empty_message,
            MAX_SHORT_TEXT_BYTES,
            &format!("{path}.emptyMessage"),
        )?;
        if let Some(selected_id) = &self.selected_id
            && !self.items.iter().any(|item| &item.id == selected_id)
        {
            return Err(ComponentValidationError::new(
                format!("{path}.selectedId"),
                "selectedId must identify a ListItem in this List",
            ));
        }
        if let Some(select) = &self.select {
            validate_identifier(select, &format!("{path}.select"))?;
        }
        for (index, item) in self.items.iter().enumerate() {
            item.validate(&format!("{path}.items[{index}]"))?;
        }
        Ok(())
    }

    pub(crate) fn insert(
        &mut self,
        index: usize,
        item: ListItem,
    ) -> Result<(), ComponentValidationError> {
        if index > self.items.len() {
            return Err(ComponentValidationError::new(
                "delta.index",
                "List insertion index is outside the collection",
            ));
        }
        self.items.insert(index, item);
        Ok(())
    }

    pub(crate) fn remove(&mut self, item_id: &str) -> Result<(), ComponentValidationError> {
        let Some(index) = self.items.iter().position(|item| item.id == item_id) else {
            return Err(ComponentValidationError::new(
                "delta.itemId",
                format!("ListItem {item_id:?} is not present"),
            ));
        };
        self.items.remove(index);
        Ok(())
    }
}

const fn is_zero_u16(value: &u16) -> bool {
    *value == 0
}

const fn default_page_overlap() -> u16 {
    1
}

const fn is_default_page_overlap(value: &u16) -> bool {
    *value == default_page_overlap()
}

const fn is_default_page_behavior(value: &ListPageBehavior) -> bool {
    matches!(value, ListPageBehavior::Selection)
}

/// Single-line input placed in a Page's named header slot.
///
/// `setValue` is optional because submit-only inputs may keep an uncommitted
/// draft locally and send it with the submit event. This avoids a network
/// round trip for every keystroke while the Rust App remains authoritative for
/// committed state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub placeholder: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submit: Option<String>,
}

impl Input {
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: String::new(),
            placeholder: String::new(),
            set_value: None,
            submit: None,
        }
    }

    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    #[must_use]
    pub fn set_value_action(mut self, action: impl Into<String>) -> Self {
        self.set_value = Some(action.into());
        self
    }

    #[must_use]
    pub fn submit_action(mut self, action: impl Into<String>) -> Self {
        self.submit = Some(action.into());
        self
    }

    fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        validate_identifier(&self.id, &format!("{path}.id"))?;
        validate_text(&self.label, MAX_SHORT_TEXT_BYTES, &format!("{path}.label"))?;
        validate_text(&self.value, MAX_INPUT_BYTES, &format!("{path}.value"))?;
        validate_text(
            &self.placeholder,
            MAX_SHORT_TEXT_BYTES,
            &format!("{path}.placeholder"),
        )?;
        if let Some(action) = &self.set_value {
            validate_identifier(action, &format!("{path}.setValue"))?;
        }
        if let Some(action) = &self.submit {
            validate_identifier(action, &format!("{path}.submit"))?;
        }
        Ok(())
    }
}

/// Controls currently accepted by a Page's header region.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PageHeaderSlot {
    Input(Input),
}

impl PageHeaderSlot {
    #[must_use]
    pub const fn input(input: Input) -> Self {
        Self::Input(input)
    }

    #[must_use]
    pub const fn as_input(&self) -> &Input {
        match self {
            Self::Input(input) => input,
        }
    }

    fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        match self {
            Self::Input(input) => input.validate(path),
        }
    }
}

/// Containers currently accepted by a Page's body region.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PageBodySlot {
    List(List),
}

impl PageBodySlot {
    #[must_use]
    pub const fn list(list: List) -> Self {
        Self::List(list)
    }

    #[must_use]
    pub const fn as_list(&self) -> &List {
        match self {
            Self::List(list) => list,
        }
    }

    fn as_list_mut(&mut self) -> &mut List {
        match self {
            Self::List(list) => list,
        }
    }

    fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        match self {
            Self::List(list) => list.validate(path),
        }
    }
}

/// Top-level data/document container with named, constrained regions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<PageHeaderSlot>,
    pub body: PageBodySlot,
}

impl Page {
    #[must_use]
    pub fn new(title: impl Into<String>, list: List) -> Self {
        Self {
            title: title.into(),
            back: None,
            header: None,
            body: PageBodySlot::List(list),
        }
    }

    #[must_use]
    pub fn input(mut self, input: Input) -> Self {
        self.header = Some(PageHeaderSlot::Input(input));
        self
    }

    /// Declares a Page-level action that returns to the previous semantic view.
    #[must_use]
    pub fn back_action(mut self, action: impl Into<String>) -> Self {
        self.back = Some(action.into());
        self
    }

    #[must_use]
    pub const fn list(&self) -> &List {
        self.body.as_list()
    }

    #[must_use]
    pub fn input_spec(&self) -> Option<&Input> {
        self.header.as_ref().map(PageHeaderSlot::as_input)
    }

    /// Capabilities a renderer needs for this exact closed tree.
    #[must_use]
    pub fn required_capabilities(&self) -> Vec<&'static str> {
        let mut capabilities = vec![
            PAGE_COMPONENT_CAPABILITY,
            LIST_COMPONENT_CAPABILITY,
            LIST_ITEM_COMPONENT_CAPABILITY,
        ];
        if self.header.is_some() {
            capabilities.push(INPUT_COMPONENT_CAPABILITY);
        }
        if self.back.is_some() {
            capabilities.push(PAGE_BACK_CAPABILITY);
        }
        if self
            .list()
            .items
            .iter()
            .any(|item| item.detail.is_some() || item.value.is_some())
        {
            capabilities.push(LIST_ITEM_METADATA_CAPABILITY);
        }
        if self.list().items.iter().any(|item| item.activate.is_some()) {
            capabilities.push(LIST_ITEM_ACTIVATE_CAPABILITY);
        }
        if self.list().items.iter().any(|item| {
            item.slots()
                .any(|slot| matches!(slot, ListItemSlot::Toggle(_)))
        }) {
            capabilities.push(TOGGLE_COMPONENT_CAPABILITY);
        }
        if self.list().items.iter().any(|item| {
            item.busy
                || item.label_tone != ListItemTone::Default
                || item.value_tone != ListItemTone::Muted
                || item.emphasis != ListItemEmphasis::Regular
                || item.value_min_width.is_some()
                || item
                    .slots()
                    .any(|slot| matches!(slot, ListItemSlot::Status(_) | ListItemSlot::Badge(_)))
        }) {
            capabilities.push(LIST_ITEM_PRESENTATION_CAPABILITY);
        }
        if self.list().items.iter().any(|item| {
            item.slots()
                .any(|slot| matches!(slot, ListItemSlot::Status(_)))
        }) {
            capabilities.push(STATUS_SYMBOL_COMPONENT_CAPABILITY);
        }
        if self.list().items.iter().any(|item| {
            item.slots()
                .any(|slot| matches!(slot, ListItemSlot::Badge(_)))
        }) {
            capabilities.push(BADGE_COMPONENT_CAPABILITY);
        }
        let list = self.list();
        if list.selected_id.is_some()
            || list.select.is_some()
            || list.scroll_padding != 0
            || list.page_overlap != default_page_overlap()
            || list.page_behavior != ListPageBehavior::Selection
            || list.space_pages_down
        {
            capabilities.push(LIST_SELECTION_CAPABILITY);
        }
        capabilities
    }

    /// Validates the closed tree, including globally unique nested ids.
    pub fn validate(&self) -> Result<(), ComponentValidationError> {
        validate_text(&self.title, MAX_SHORT_TEXT_BYTES, "page.title")?;
        if let Some(back) = &self.back {
            validate_identifier(back, "page.back")?;
        }
        if let Some(header) = &self.header {
            header.validate("page.header")?;
        }
        self.body.validate("page.body")?;

        let mut ids = HashSet::new();
        if let Some(input) = self.input_spec() {
            register_unique(&mut ids, &input.id, "page.header.id")?;
        }
        register_unique(&mut ids, &self.list().id, "page.body.id")?;
        for (index, item) in self.list().items.iter().enumerate() {
            register_unique(&mut ids, &item.id, &format!("page.body.items[{index}].id"))?;
            for slot in item.slots() {
                if let Some(id) = slot.id() {
                    register_unique(&mut ids, id, &format!("page.body.items[{index}].slot.id"))?;
                }
            }
        }
        Ok(())
    }

    /// Uses App Kit's single-line List renderer and InputField named slots.
    #[must_use]
    pub fn widget<'a>(
        &'a self,
        input: &'a mut InputField,
        list_state: &'a mut ListState,
    ) -> PageWidget<'a> {
        PageWidget {
            page: self,
            input,
            list_state,
            theme: PageTheme::default(),
        }
    }

    /// Resolves the terminal rectangles used by the Page's named slots.
    ///
    /// Apps use this for target-aware mouse input without duplicating the
    /// component's layout math. The returned List rectangle maps one terminal
    /// row to one ListItem in v1.
    #[must_use]
    pub fn layout(&self, area: Rect) -> PageLayout {
        let constraints = if self.input_spec().is_some() {
            vec![
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ]
        } else {
            vec![Constraint::Length(2), Constraint::Min(0)]
        };
        let slots = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);
        if self.input_spec().is_some() {
            PageLayout {
                title: slots[0],
                input: Some(slots[1]),
                list: slots[3],
            }
        } else {
            PageLayout {
                title: slots[0],
                input: None,
                list: slots[1],
            }
        }
    }

    pub(crate) fn set_input_value(
        &mut self,
        input_id: &str,
        value: String,
    ) -> Result<(), ComponentValidationError> {
        let Some(PageHeaderSlot::Input(input)) = &mut self.header else {
            return Err(ComponentValidationError::new(
                "delta.nodeId",
                "Page has no Input header",
            ));
        };
        if input.id != input_id {
            return Err(ComponentValidationError::new(
                "delta.nodeId",
                format!("Input {input_id:?} is not present"),
            ));
        }
        input.value = value;
        Ok(())
    }

    pub(crate) fn set_toggle_value(
        &mut self,
        toggle_id: &str,
        value: bool,
    ) -> Result<(), ComponentValidationError> {
        for item in &mut self.body.as_list_mut().items {
            if item.set_toggle_value(toggle_id, value) {
                return Ok(());
            }
        }
        Err(ComponentValidationError::new(
            "delta.nodeId",
            format!("Toggle {toggle_id:?} is not present"),
        ))
    }

    pub(crate) fn insert_list_item(
        &mut self,
        list_id: &str,
        index: usize,
        item: ListItem,
    ) -> Result<(), ComponentValidationError> {
        let list = self.body.as_list_mut();
        if list.id != list_id {
            return Err(ComponentValidationError::new(
                "delta.listId",
                format!("List {list_id:?} is not present"),
            ));
        }
        list.insert(index, item)
    }

    pub(crate) fn set_list_selection(
        &mut self,
        list_id: &str,
        selected_id: Option<String>,
    ) -> Result<(), ComponentValidationError> {
        let list = self.body.as_list_mut();
        if list.id != list_id {
            return Err(ComponentValidationError::new(
                "delta.listId",
                format!("List {list_id:?} is not present"),
            ));
        }
        if let Some(selected_id) = &selected_id
            && !list.items.iter().any(|item| &item.id == selected_id)
        {
            return Err(ComponentValidationError::new(
                "delta.selectedId",
                format!("ListItem {selected_id:?} is not present"),
            ));
        }
        list.selected_id = selected_id;
        Ok(())
    }

    pub(crate) fn remove_list_item(
        &mut self,
        list_id: &str,
        item_id: &str,
    ) -> Result<(), ComponentValidationError> {
        let list = self.body.as_list_mut();
        if list.id != list_id {
            return Err(ComponentValidationError::new(
                "delta.listId",
                format!("List {list_id:?} is not present"),
            ));
        }
        list.remove(item_id)
    }
}

/// Borderless terminal presentation for [`Page`] and its current slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageTheme {
    pub style: Style,
    pub title: Style,
    pub item: Style,
    pub detail: Style,
    pub value: Style,
    pub accent: Style,
    pub info: Style,
    pub success: Style,
    pub warning: Style,
    pub danger: Style,
    pub done: Style,
    pub toggle: Style,
    pub badge: Style,
    pub busy: Style,
    pub delete: Style,
    pub empty: Style,
    pub selected: Style,
    pub selected_item: Style,
    pub selected_detail: Style,
    pub selected_value: Style,
    pub selected_badge: Style,
    pub navigation: Style,
    pub scrollbar_track: Style,
    pub scrollbar_thumb: Style,
    pub left_padding: u16,
}

/// Terminal hit-test geometry for a rendered [`Page`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PageLayout {
    pub title: Rect,
    pub input: Option<Rect>,
    pub list: Rect,
}

impl PageTheme {
    #[must_use]
    pub const fn for_theme(theme: KitTheme) -> Self {
        Self {
            style: Style::new(),
            title: Style::new().fg(theme.text).add_modifier(Modifier::BOLD),
            item: Style::new().fg(theme.text),
            detail: Style::new().fg(theme.muted),
            value: Style::new().fg(theme.muted),
            accent: Style::new().fg(theme.accent),
            info: Style::new().fg(match theme.scheme {
                crate::ColorScheme::Dark => ratatui::style::Color::LightBlue,
                crate::ColorScheme::Light => ratatui::style::Color::Blue,
            }),
            success: Style::new().fg(match theme.scheme {
                crate::ColorScheme::Dark => ratatui::style::Color::LightGreen,
                crate::ColorScheme::Light => ratatui::style::Color::Green,
            }),
            warning: Style::new().fg(match theme.scheme {
                crate::ColorScheme::Dark => ratatui::style::Color::LightYellow,
                crate::ColorScheme::Light => ratatui::style::Color::Yellow,
            }),
            danger: Style::new().fg(theme.danger),
            done: Style::new().fg(theme.muted),
            toggle: Style::new().fg(theme.accent),
            badge: Style::new().fg(theme.muted),
            busy: Style::new().fg(theme.accent),
            delete: Style::new().fg(theme.subtle),
            empty: Style::new().fg(theme.subtle),
            selected: theme.selected_row,
            selected_item: Style::new(),
            selected_detail: Style::new().add_modifier(Modifier::DIM),
            selected_value: Style::new(),
            selected_badge: Style::new().add_modifier(Modifier::DIM),
            navigation: Style::new().fg(theme.subtle),
            scrollbar_track: theme.scrollbar_track,
            scrollbar_thumb: theme.scrollbar_thumb,
            left_padding: SELECTABLE_LEFT_PADDING,
        }
    }

    #[must_use]
    pub fn detected() -> Self {
        Self::for_theme(KitTheme::detected())
    }

    fn tone(self, tone: ListItemTone) -> Style {
        match tone {
            ListItemTone::Default => self.item,
            ListItemTone::Muted => self.value,
            ListItemTone::Accent => self.accent,
            ListItemTone::Info => self.info,
            ListItemTone::Success => self.success,
            ListItemTone::Warning => self.warning,
            ListItemTone::Danger => self.danger,
        }
    }
}

impl Default for PageTheme {
    fn default() -> Self {
        Self::for_theme(KitTheme::dark())
    }
}

const LIST_SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Standalone single-line List renderer built from App Kit row primitives.
pub struct ListWidget<'a> {
    list: &'a List,
    state: &'a mut ListState,
    theme: PageTheme,
}

impl ListWidget<'_> {
    #[must_use]
    pub const fn theme(mut self, theme: PageTheme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for ListWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            self.state.prepare(area, self.list.items.len());
            return;
        }
        let item_count = self.list.items.len();
        self.state.set_navigation(
            usize::from(self.list.scroll_padding),
            usize::from(self.list.page_overlap),
            self.list.page_behavior,
        );
        if let Some(selected_id) = &self.list.selected_id {
            let selected = self
                .list
                .items
                .iter()
                .position(|item| &item.id == selected_id);
            self.state.select(selected, item_count);
        }
        let overflow = item_count > usize::from(area.height) && area.width > 1;
        let rows_area = Rect {
            width: area.width.saturating_sub(u16::from(overflow)),
            ..area
        };
        self.state.prepare(rows_area, item_count);

        if item_count == 0 {
            let content = SelectableRow::new(false, self.theme.selected)
                .inactive_style(self.theme.style)
                .paint(
                    Rect::new(rows_area.x, rows_area.y, rows_area.width, 1),
                    buffer,
                );
            Paragraph::new(self.list.empty_message.as_str())
                .style(self.theme.empty)
                .render(content, buffer);
        } else {
            for row in 0..rows_area.height {
                let index = self.state.offset().saturating_add(usize::from(row));
                let Some(item) = self.list.items.get(index) else {
                    break;
                };
                render_list_item(
                    item,
                    Rect::new(
                        rows_area.x,
                        rows_area.y.saturating_add(row),
                        rows_area.width,
                        1,
                    ),
                    self.state.selected() == Some(index),
                    self.state.spinner_frame(),
                    self.theme,
                    buffer,
                );
            }
        }
        if overflow {
            VerticalScrollbar::new(
                item_count,
                usize::from(rows_area.height),
                self.state.offset(),
            )
            .track_style(self.theme.scrollbar_track)
            .thumb_style(self.theme.scrollbar_thumb)
            .render(
                Rect::new(area.right().saturating_sub(1), area.y, 1, area.height),
                buffer,
            );
        }
    }
}

fn render_list_item(
    item: &ListItem,
    area: Rect,
    selected: bool,
    spinner_frame: usize,
    theme: PageTheme,
    buffer: &mut Buffer,
) {
    if area.is_empty() {
        return;
    }
    let content = SelectableRow::new(selected, theme.selected)
        .inactive_style(theme.style)
        .paint(area, buffer);
    if content.is_empty() {
        return;
    }

    let mut left = Vec::new();
    if item.busy {
        let style = if selected {
            theme.selected_item
        } else {
            theme.busy
        };
        left.push(Span::styled(
            format!(
                "{} ",
                LIST_SPINNER_FRAMES[spinner_frame % LIST_SPINNER_FRAMES.len()]
            ),
            style,
        ));
    }
    if let Some(slot) = &item.leading {
        append_leading_slot(&mut left, slot, selected, theme);
    }
    let mut label_style = if item.done {
        if selected {
            theme.selected_detail
        } else {
            theme.done
        }
        .add_modifier(Modifier::CROSSED_OUT)
    } else if selected {
        theme.selected_item
    } else {
        theme.tone(item.label_tone)
    };
    if item.emphasis == ListItemEmphasis::Strong {
        label_style = label_style.add_modifier(Modifier::BOLD);
    }
    left.push(Span::styled(item.label.clone(), label_style));
    if let Some(ListItemSlot::Badge(badge)) = &item.accessory {
        left.push(Span::raw(" "));
        left.push(Span::styled(
            badge.text.clone(),
            if selected {
                theme.selected_badge
            } else {
                theme.tone(badge.tone)
            },
        ));
    }
    if let Some(detail) = &item.detail {
        left.push(Span::styled(
            format!("  {detail}"),
            if selected {
                theme.selected_detail
            } else {
                theme.detail
            },
        ));
    }

    let mut suffix = Vec::new();
    if let Some(slot) = &item.trailing {
        append_trailing_slot(&mut suffix, slot, selected, theme);
    }
    if let Some(slot) = &item.accessory
        && !matches!(slot, ListItemSlot::Badge(_))
    {
        append_trailing_slot(&mut suffix, slot, selected, theme);
    }
    if item.delete.is_some() {
        suffix.push(Span::styled(
            "[d]",
            if selected {
                theme.selected_detail
            } else {
                theme.delete
            },
        ));
    }

    let suffix_width = Line::from(suffix.clone()).width();
    let value = item.value.as_deref().filter(|value| {
        let value_width = UnicodeWidthStr::width(*value);
        let default_min = value_width
            .saturating_add(suffix_width)
            .saturating_add(usize::from(SELECTABLE_LEFT_PADDING))
            .saturating_add(9);
        usize::from(area.width)
            >= usize::from(
                item.value_min_width
                    .unwrap_or_else(|| u16::try_from(default_min).unwrap_or(u16::MAX)),
            )
    });
    let value_style = value.map(|_| {
        if selected {
            theme.selected_value
        } else {
            theme.tone(item.value_tone)
        }
    });
    let mut right = Vec::new();
    if let Some(value) = value {
        right.push(Span::styled(
            value.to_owned(),
            value_style.unwrap_or_default(),
        ));
    }
    if !suffix.is_empty() {
        if !right.is_empty() {
            right.push(Span::raw("  "));
        }
        right.extend(suffix);
    }
    let right_width = Line::from(right.clone())
        .width()
        .min(usize::from(content.width));
    let right_columns = u16::try_from(right_width).unwrap_or(content.width);
    let gap = u16::from(right_columns > 0 && right_columns < content.width);
    let [label_area, value_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(right_columns.saturating_add(gap)),
    ])
    .areas(content);
    Paragraph::new(Line::from(left)).render(label_area, buffer);
    if right_columns > 0 {
        let mut paragraph = Paragraph::new(Line::from(right)).alignment(Alignment::Right);
        if let Some(style) = value_style {
            paragraph = paragraph.style(style);
        }
        paragraph.render(value_area, buffer);
    }
}

fn append_leading_slot(
    spans: &mut Vec<Span<'static>>,
    slot: &ListItemSlot,
    selected: bool,
    theme: PageTheme,
) {
    match slot {
        ListItemSlot::Toggle(toggle) => spans.push(Span::styled(
            format!("{} ", toggle.marker()),
            if selected {
                theme.selected_item
            } else {
                theme.toggle
            },
        )),
        ListItemSlot::Status(status) => {
            let mut style = if selected && !status.preserve_tone_when_selected {
                theme.selected_item
            } else {
                theme.tone(status.tone)
            };
            if status.emphasis == ListItemEmphasis::Strong {
                style = style.add_modifier(Modifier::BOLD);
            }
            spans.push(Span::styled(format!("{}  ", status.symbol), style));
        }
        ListItemSlot::Badge(badge) => spans.push(Span::styled(
            format!("{} ", badge.text),
            if selected {
                theme.selected_badge
            } else {
                theme.tone(badge.tone)
            },
        )),
    }
}

fn append_trailing_slot(
    spans: &mut Vec<Span<'static>>,
    slot: &ListItemSlot,
    selected: bool,
    theme: PageTheme,
) {
    match slot {
        ListItemSlot::Toggle(toggle) => spans.push(Span::styled(
            toggle.marker(),
            if selected {
                theme.selected_item
            } else {
                theme.toggle
            },
        )),
        ListItemSlot::Status(status) => spans.push(Span::styled(
            status.symbol.clone(),
            if selected && !status.preserve_tone_when_selected {
                theme.selected_item
            } else {
                theme.tone(status.tone)
            },
        )),
        ListItemSlot::Badge(badge) => spans.push(Span::styled(
            badge.text.clone(),
            if selected {
                theme.selected_badge
            } else {
                theme.tone(badge.tone)
            },
        )),
    }
}

/// Renderable Page view returned by [`Page::widget`].
pub struct PageWidget<'a> {
    page: &'a Page,
    input: &'a mut InputField,
    list_state: &'a mut ListState,
    theme: PageTheme,
}

impl PageWidget<'_> {
    #[must_use]
    pub const fn theme(mut self, theme: PageTheme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for PageWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        buffer.set_style(area, self.theme.style);
        let layout = self.page.layout(area);
        Paragraph::new(format!(
            "{}{}{}",
            " ".repeat(usize::from(self.theme.left_padding)),
            if self.page.back.is_some() {
                "‹  "
            } else {
                ""
            },
            self.page.title
        ))
        .style(self.theme.title)
        .render(layout.title, buffer);

        if let Some(input) = self.page.input_spec() {
            self.input.set_placeholder(input.placeholder.clone());
            self.input.set_prompt(format!("{}: ", input.label));
            self.input
                .widget()
                .render(layout.input.expect("Page input layout"), buffer);
        }
        self.page
            .list()
            .widget(self.list_state)
            .theme(self.theme)
            .render(layout.list, buffer);
    }
}

/// Validation failure for the closed component tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentValidationError {
    pub path: String,
    pub message: String,
}

impl ComponentValidationError {
    #[must_use]
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ComponentValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for ComponentValidationError {}

fn register_unique(
    ids: &mut HashSet<String>,
    id: &str,
    path: &str,
) -> Result<(), ComponentValidationError> {
    if !ids.insert(id.to_owned()) {
        return Err(ComponentValidationError::new(
            path,
            format!("duplicate component id {id:?}"),
        ));
    }
    Ok(())
}

pub(crate) fn validate_identifier(value: &str, path: &str) -> Result<(), ComponentValidationError> {
    if value.is_empty()
        || value.len() > 256
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
    {
        return Err(ComponentValidationError::new(
            path,
            "must be a portable identifier",
        ));
    }
    Ok(())
}

pub(crate) fn validate_text(
    value: &str,
    maximum: usize,
    path: &str,
) -> Result<(), ComponentValidationError> {
    if value.len() > maximum
        || value
            .chars()
            .any(|character| matches!(character, '\0' | '\r'))
    {
        return Err(ComponentValidationError::new(
            path,
            format!("must contain at most {maximum} bytes and no NUL/CR characters"),
        ));
    }
    Ok(())
}

fn validate_single_line(value: &str, path: &str) -> Result<(), ComponentValidationError> {
    if value.contains('\n') {
        return Err(ComponentValidationError::new(path, "must be a single line"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn todo_page() -> Page {
        Page::new(
            "Todos",
            List::new(
                "todos",
                vec![
                    ListItem::new("todo-1", "Run the standalone TUI")
                        .done(true)
                        .trailing(ListItemSlot::toggle(Toggle::new(
                            "todo-1-toggle",
                            "Completed",
                            true,
                            "set-done",
                        )))
                        .delete_action("delete-todo"),
                ],
            )
            .empty_message("No todos yet"),
        )
        .input(
            Input::new("new-todo", "New todo")
                .placeholder("What needs doing?")
                .submit_action("add-todo"),
        )
    }

    #[test]
    fn list_composition_is_closed_and_ids_are_unique() {
        let page = todo_page();
        page.validate().unwrap();
        assert_eq!(
            page.required_capabilities(),
            vec!["page", "list", "listItem", "input", "toggle"]
        );

        let mut duplicate = page;
        duplicate.body.as_list_mut().items[0]
            .trailing
            .as_mut()
            .unwrap()
            .toggle_mut("todo-1-toggle")
            .unwrap()
            .id = "new-todo".to_owned();
        assert!(duplicate.validate().is_err());
    }

    #[test]
    fn terminal_page_uses_shared_list_rows_and_input_field() {
        let page = todo_page();
        let mut input = InputField::new("");
        input.set_focused(true);
        let mut state = ListState::new(Some(0));
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(page.widget(&mut input, &mut state), frame.area());
            })
            .unwrap();
        let rendered = terminal.backend().buffer();
        let text = rendered
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("Todos"));
        assert!(text.contains("[x]"));
        assert!(text.contains("Run the standalone TUI"));
        let selected_y = 4;
        assert_eq!(
            rendered[(0, selected_y)].bg,
            ratatui::style::Color::Rgb(63, 63, 70)
        );
        assert_eq!(
            rendered[(49, selected_y)].bg,
            ratatui::style::Color::Rgb(63, 63, 70)
        );
        assert_eq!(rendered[(0, selected_y)].symbol(), " ");
        assert_eq!(rendered[(1, selected_y)].symbol(), " ");
        assert_eq!(rendered[(2, selected_y)].symbol(), "R");
    }

    #[test]
    fn terminal_list_preserves_full_row_selection_insets_and_overflow_scrollbar() {
        let list = List::new(
            "files",
            (0..5)
                .map(|index| ListItem::new(format!("file-{index}"), format!("file-{index}.rs")))
                .collect(),
        );
        let mut state = ListState::new(Some(1));
        let backend = TestBackend::new(24, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| frame.render_widget(list.widget(&mut state), frame.area()))
            .unwrap();

        let rendered = terminal.backend().buffer();
        let selected = KitTheme::dark().selected_row.bg.unwrap();
        assert_eq!(rendered[(0, 1)].bg, selected);
        assert_eq!(rendered[(21, 1)].bg, selected);
        assert_eq!(rendered[(22, 1)].bg, selected);
        assert_ne!(rendered[(23, 1)].bg, selected);
        assert_eq!(rendered[(0, 1)].symbol(), " ");
        assert_eq!(rendered[(1, 1)].symbol(), " ");
        assert_eq!(rendered[(2, 1)].symbol(), "f");
        assert_eq!(rendered[(22, 1)].symbol(), " ");
        assert_ne!(rendered[(23, 0)].symbol(), " ");
        assert_eq!(state.rows_area(), Rect::new(0, 0, 23, 3));
        assert_eq!(
            state.item_at(ratatui::layout::Position::new(22, 1), 5),
            Some(1)
        );
        assert_eq!(
            state.item_at(ratatui::layout::Position::new(23, 1), 5),
            None
        );
    }

    #[test]
    fn terminal_list_right_aligns_values_and_drops_them_when_narrow() {
        let list = List::new(
            "changes",
            vec![
                ListItem::new("src", "src/lib.rs")
                    .leading(ListItemSlot::status(
                        StatusSymbol::new("M", "Modified")
                            .tone(ListItemTone::Warning)
                            .emphasis(ListItemEmphasis::Strong),
                    ))
                    .value("modified")
                    .value_min_width(24),
            ],
        );
        let mut state = ListState::new(Some(0));
        let mut wide = Buffer::empty(Rect::new(0, 0, 30, 1));
        list.widget(&mut state)
            .render(Rect::new(0, 0, 30, 1), &mut wide);
        let wide_text = (0..30).map(|x| wide[(x, 0)].symbol()).collect::<String>();
        assert!(wide_text.starts_with("  M  src/lib.rs"));
        assert_eq!(&wide_text[21..29], "modified");
        assert_eq!(&wide_text[29..], " ");

        let mut narrow_state = ListState::new(Some(0));
        let mut narrow = Buffer::empty(Rect::new(0, 0, 20, 1));
        list.widget(&mut narrow_state)
            .render(Rect::new(0, 0, 20, 1), &mut narrow);
        let narrow_text = (0..20).map(|x| narrow[(x, 0)].symbol()).collect::<String>();
        assert!(narrow_text.contains("src/lib.rs"));
        assert!(!narrow_text.contains("modified"));
    }

    #[test]
    fn terminal_list_renders_badges_and_busy_rows_from_closed_slots() {
        let list = List::new(
            "issues",
            vec![
                ListItem::new("loading", "Loading issues")
                    .busy(true)
                    .accessory(ListItemSlot::badge(
                        Badge::new("open").tone(ListItemTone::Success),
                    )),
            ],
        );
        let mut state = ListState::new(Some(0));
        state.set_spinner_frame(0);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 32, 1));
        list.widget(&mut state)
            .render(Rect::new(0, 0, 32, 1), &mut buffer);
        let text = (0..32).map(|x| buffer[(x, 0)].symbol()).collect::<String>();
        assert!(text.starts_with("  ⠋ Loading issues open"));
    }

    #[test]
    fn page_layout_exposes_the_same_named_slots_used_for_rendering() {
        let page = todo_page();
        let layout = page.layout(Rect::new(3, 5, 50, 10));
        assert_eq!(layout.title, Rect::new(3, 5, 50, 2));
        assert_eq!(layout.input, Some(Rect::new(3, 7, 50, 1)));
        assert_eq!(layout.list, Rect::new(3, 9, 50, 6));
    }

    #[test]
    fn master_detail_fields_require_explicit_renderer_capabilities() {
        let page = Page::new(
            "Usage",
            List::new(
                "providers",
                vec![
                    ListItem::new("codex", "Codex")
                        .detail("Pro")
                        .value("7-day 3% used")
                        .activate_action("open-provider"),
                ],
            ),
        )
        .back_action("close-provider");
        page.validate().unwrap();
        assert_eq!(
            page.required_capabilities(),
            vec![
                "page",
                "list",
                "listItem",
                "pageBack",
                "listItemMetadata",
                "listItemActivate",
            ]
        );
    }

    #[test]
    fn component_mutations_preserve_done_toggle_invariant() {
        let mut page = todo_page();
        page.set_toggle_value("todo-1-toggle", false).unwrap();
        assert!(!page.list().items[0].done);
        let ListItemSlot::Toggle(toggle) = page.list().items[0].trailing.as_ref().unwrap() else {
            panic!("todo fixture has a Toggle")
        };
        assert!(!toggle.value);
    }
}
