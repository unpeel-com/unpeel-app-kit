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
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    List as RatatuiList, ListItem as RatatuiListItem, ListState, Paragraph, StatefulWidget, Widget,
};
use serde::{Deserialize, Serialize};

use crate::{InputField, KitTheme, SELECTABLE_LEFT_PADDING};

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
/// Renderer capability for the v1 Toggle control.
pub const TOGGLE_COMPONENT_CAPABILITY: &str = "toggle";
/// Renderer capability for the v1 Input control.
pub const INPUT_COMPONENT_CAPABILITY: &str = "input";
/// Renderer capability for Page-level back navigation.
pub const PAGE_BACK_CAPABILITY: &str = "pageBack";

const MAX_ITEMS: usize = 100_000;
const MAX_SHORT_TEXT_BYTES: usize = 4 * 1024;
const MAX_LABEL_BYTES: usize = 16 * 1024;
const MAX_INPUT_BYTES: usize = 1024 * 1024;

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

/// Controls currently allowed in a ListItem's named slots.
///
/// This enum grows deliberately as controls such as Badge or Menu become part
/// of the closed vocabulary. Arbitrary child nodes are never accepted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ListItemSlot {
    Toggle(Toggle),
}

impl ListItemSlot {
    #[must_use]
    pub const fn toggle(toggle: Toggle) -> Self {
        Self::Toggle(toggle)
    }

    #[must_use]
    pub const fn id(&self) -> &str {
        match self {
            Self::Toggle(toggle) => toggle.id.as_str(),
        }
    }

    pub(crate) fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        match self {
            Self::Toggle(toggle) => toggle.validate(path),
        }
    }

    fn toggle_mut(&mut self, id: &str) -> Option<&mut Toggle> {
        match self {
            Self::Toggle(toggle) if toggle.id == id => Some(toggle),
            Self::Toggle(_) => None,
        }
    }

    fn as_toggle(&self) -> &Toggle {
        match self {
            Self::Toggle(toggle) => toggle,
        }
    }
}

/// One keyed, semantically meaningful row in a [`List`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default)]
    pub done: bool,
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
            detail: None,
            value: None,
            done: false,
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
    pub const fn done(mut self, done: bool) -> Self {
        self.done = done;
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
        if let Some(detail) = &self.detail {
            validate_text(detail, MAX_LABEL_BYTES, &format!("{path}.detail"))?;
        }
        if let Some(value) = &self.value {
            validate_text(value, MAX_SHORT_TEXT_BYTES, &format!("{path}.value"))?;
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
            .map(ListItemSlot::as_toggle)
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

    fn ratatui(&self, theme: PageTheme) -> RatatuiListItem<'static> {
        let mut spans = vec![Span::raw(" ".repeat(usize::from(theme.left_padding)))];
        if let Some(ListItemSlot::Toggle(toggle)) = &self.leading {
            spans.push(Span::styled(format!("{} ", toggle.marker()), theme.toggle));
        }
        let label_style = if self.done {
            theme
                .done
                .add_modifier(Modifier::CROSSED_OUT)
                .remove_modifier(Modifier::BOLD)
        } else {
            theme.item
        };
        spans.push(Span::styled(self.label.clone(), label_style));
        if let Some(detail) = &self.detail {
            spans.push(Span::styled(format!("  {detail}"), theme.detail));
        }
        if let Some(value) = &self.value {
            spans.push(Span::styled(format!("  {value}"), theme.value));
        }
        for slot in [&self.trailing, &self.accessory].into_iter().flatten() {
            let toggle = slot.as_toggle();
            spans.push(Span::styled(format!("  {}", toggle.marker()), theme.toggle));
        }
        if self.delete.is_some() {
            spans.push(Span::styled("  [d]", theme.delete));
        }
        if self.activate.is_some() {
            spans.push(Span::styled("  ›", theme.navigation));
        }
        RatatuiListItem::new(Line::from(spans))
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
}

impl List {
    #[must_use]
    pub fn new(id: impl Into<String>, items: Vec<ListItem>) -> Self {
        Self {
            id: id.into(),
            items,
            empty_message: String::new(),
        }
    }

    #[must_use]
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = message.into();
        self
    }

    #[must_use]
    pub fn widget(&self, theme: PageTheme) -> RatatuiList<'static> {
        let rows = if self.items.is_empty() {
            vec![RatatuiListItem::new(Line::styled(
                format!(
                    "{}{}",
                    " ".repeat(usize::from(theme.left_padding)),
                    self.empty_message
                ),
                theme.empty,
            ))]
        } else {
            self.items.iter().map(|item| item.ratatui(theme)).collect()
        };
        RatatuiList::new(rows).highlight_style(theme.selected)
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
        if self
            .list()
            .items
            .iter()
            .any(|item| item.slots().next().is_some())
        {
            capabilities.push(TOGGLE_COMPONENT_CAPABILITY);
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
                register_unique(
                    &mut ids,
                    slot.id(),
                    &format!("page.body.items[{index}].slot.id"),
                )?;
            }
        }
        Ok(())
    }

    /// Uses Ratatui's List and App Kit's InputField to render the named slots.
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
    pub done: Style,
    pub toggle: Style,
    pub delete: Style,
    pub empty: Style,
    pub selected: Style,
    pub navigation: Style,
    pub left_padding: u16,
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
            done: Style::new().fg(theme.muted),
            toggle: Style::new().fg(theme.accent),
            delete: Style::new().fg(theme.subtle),
            empty: Style::new().fg(theme.subtle),
            selected: theme.selected_row,
            navigation: Style::new().fg(theme.subtle),
            left_padding: SELECTABLE_LEFT_PADDING,
        }
    }

    #[must_use]
    pub fn detected() -> Self {
        Self::for_theme(KitTheme::detected())
    }
}

impl Default for PageTheme {
    fn default() -> Self {
        Self::for_theme(KitTheme::dark())
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
        let constraints = if self.page.input_spec().is_some() {
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
        .render(slots[0], buffer);

        let list_area = if let Some(input) = self.page.input_spec() {
            self.input.set_placeholder(input.placeholder.clone());
            self.input.set_prompt(format!("{}: ", input.label));
            self.input.widget().render(slots[1], buffer);
            slots[3]
        } else {
            slots[1]
        };
        StatefulWidget::render(
            self.page.list().widget(self.theme),
            list_area,
            buffer,
            self.list_state,
        );
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

fn validate_identifier(value: &str, path: &str) -> Result<(), ComponentValidationError> {
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

fn validate_text(value: &str, maximum: usize, path: &str) -> Result<(), ComponentValidationError> {
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
    fn terminal_page_uses_ratatui_list_and_input_field() {
        let page = todo_page();
        let mut input = InputField::new("");
        input.set_focused(true);
        let mut state = ListState::default().with_selected(Some(0));
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
        let ListItemSlot::Toggle(toggle) = page.list().items[0].trailing.as_ref().unwrap();
        assert!(!toggle.value);
    }
}
