//! Canonical standalone-first App Kit example.
//!
//! Run it in any terminal with `cargo run --example todo`. With the default
//! `ui-bridge` feature, the exact same binary also publishes its Page tree when
//! an Unpeel Host injects a UI endpoint. Build with `--no-default-features` to
//! remove every socket/protocol path while keeping the complete TUI.

use std::error::Error;
use std::fs;
#[cfg(unix)]
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};
use unpeel_app_kit::{
    Input, InputField, InputFieldAction, KitTheme, List, ListItem, ListItemSlot, ListKeymap,
    ListNavigationOutcome, ListState, Page, PageTheme, Toggle,
};

#[cfg(feature = "ui-bridge")]
use unpeel_app_kit::{
    AppMetadata, UiBridge, UiBridgeEvent, UiDeltaOperation, UiEventKind, UiEventOutcome,
    UiEventValue, UiNode,
};

const STATE_FORMAT: &str = "unpeel.app-kit.example.todo";
const STATE_FORMAT_VERSION: u32 = 1;
const STATE_SCHEMA_VERSION: u32 = 1;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const DEFAULT_STATE_FILE: &str = ".unpeel-todo.json";
#[cfg(feature = "ui-bridge")]
const VIEW_ID: &str = "main";
#[cfg(feature = "ui-bridge")]
const ROOT_ID: &str = "todo-page";
const LIST_ID: &str = "todos";
const INPUT_ID: &str = "new-todo";
const ADD_ACTION: &str = "add-todo";
const SET_DONE_ACTION: &str = "set-done";
const DELETE_ACTION: &str = "delete-todo";
const SELECT_ACTION: &str = "select-todo";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Todo {
    id: u64,
    label: String,
    done: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TodoState {
    format: String,
    format_version: u32,
    state_schema_version: u32,
    revision: u64,
    next_id: u64,
    todos: Vec<Todo>,
}

impl Default for TodoState {
    fn default() -> Self {
        Self {
            format: STATE_FORMAT.to_owned(),
            format_version: STATE_FORMAT_VERSION,
            state_schema_version: STATE_SCHEMA_VERSION,
            revision: 1,
            next_id: 4,
            todos: vec![
                Todo {
                    id: 1,
                    label: "Run the standalone TUI".to_owned(),
                    done: true,
                },
                Todo {
                    id: 2,
                    label: "Attach SwiftUI or web".to_owned(),
                    done: false,
                },
                Todo {
                    id: 3,
                    label: "Invite an agent with edit grant".to_owned(),
                    done: false,
                },
            ],
        }
    }
}

#[derive(Clone, Debug)]
enum Intent {
    Toggle { id: u64, value: bool },
    Add { label: String },
    Delete { id: u64 },
}

#[cfg_attr(not(feature = "ui-bridge"), allow(dead_code))]
#[derive(Clone, Debug)]
enum ModelChange {
    Toggle { id: u64, value: bool },
    Insert { index: usize, todo: Todo },
    Remove { id: u64 },
}

#[cfg(feature = "ui-bridge")]
impl ModelChange {
    fn ui_delta_operations(&self, selected_id: Option<String>) -> Vec<UiDeltaOperation> {
        let model = match self {
            Self::Toggle { id, value } => {
                UiDeltaOperation::toggle_set_value(toggle_id(*id), *value)
            }
            Self::Insert { index, todo } => {
                UiDeltaOperation::list_insert_item(LIST_ID, *index as u64, component_item(todo))
            }
            Self::Remove { id } => UiDeltaOperation::list_remove_item(LIST_ID, item_id(*id)),
        };
        vec![
            model,
            UiDeltaOperation::list_set_selection(LIST_ID, selected_id),
        ]
    }
}

struct TodoApp {
    state: TodoState,
    state_path: PathBuf,
    input: InputField,
    list_state: ListState,
    input_focused: bool,
    list_area: Rect,
    status: String,
}

impl TodoApp {
    fn load(state_path: PathBuf) -> Result<Self, Box<dyn Error>> {
        let state = match fs::read(&state_path) {
            Ok(bytes) => {
                let state: TodoState = serde_json::from_slice(&bytes)?;
                validate_saved_state(&state)?;
                state
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => TodoState::default(),
            Err(error) => return Err(error.into()),
        };
        let selected = (!state.todos.is_empty()).then_some(0);
        let mut input = InputField::new("What needs doing?");
        input.set_focused(true);
        Ok(Self {
            state,
            state_path,
            input,
            list_state: ListState::new(selected),
            input_focused: true,
            list_area: Rect::default(),
            status: String::new(),
        })
    }

    fn page(&self) -> Page {
        let mut list = List::new(
            LIST_ID,
            self.state.todos.iter().map(component_item).collect(),
        )
        .empty_message("No todos yet");
        if let Some(todo) = self.selected_todo() {
            list = list.selected(item_id(todo.id), SELECT_ACTION);
        }
        Page::new("Todos", list).input(
            Input::new(INPUT_ID, "New todo")
                .placeholder("What needs doing?")
                .submit_action(ADD_ACTION),
        )
    }

    fn commit(&mut self, intent: Intent) -> Result<ModelChange, String> {
        let mut next = self.state.clone();
        let change = match intent {
            Intent::Toggle { id, value } => {
                let todo = next
                    .todos
                    .iter_mut()
                    .find(|todo| todo.id == id)
                    .ok_or_else(|| format!("Todo {id} no longer exists"))?;
                todo.done = value;
                ModelChange::Toggle { id, value }
            }
            Intent::Add { label } => {
                let label = label.trim();
                if label.is_empty() {
                    return Err("Enter a todo first".to_owned());
                }
                if label.len() > 16 * 1024 || label.contains(['\0', '\r', '\n']) {
                    return Err("Todo labels must be one line and at most 16 KiB".to_owned());
                }
                let todo = Todo {
                    id: next.next_id,
                    label: label.to_owned(),
                    done: false,
                };
                next.next_id = next
                    .next_id
                    .checked_add(1)
                    .ok_or_else(|| "Todo id space is exhausted".to_owned())?;
                let index = next.todos.len();
                next.todos.push(todo.clone());
                ModelChange::Insert { index, todo }
            }
            Intent::Delete { id } => {
                let index = next
                    .todos
                    .iter()
                    .position(|todo| todo.id == id)
                    .ok_or_else(|| format!("Todo {id} no longer exists"))?;
                next.todos.remove(index);
                ModelChange::Remove { id }
            }
        };
        next.revision = next
            .revision
            .checked_add(1)
            .filter(|revision| *revision <= MAX_SAFE_INTEGER)
            .ok_or_else(|| "Revision space is exhausted".to_owned())?;
        save_state(&self.state_path, &next).map_err(|error| error.to_string())?;
        self.state = next;
        self.clamp_selection();
        self.status = format!("Saved {}", self.state_path.display());
        Ok(change)
    }

    fn selected_todo(&self) -> Option<&Todo> {
        self.list_state
            .selected()
            .and_then(|index| self.state.todos.get(index))
    }

    fn select_relative(&mut self, offset: isize) {
        let len = self.state.todos.len();
        if len == 0 {
            self.list_state.select(None, 0);
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as isize;
        let selected = (current + offset).clamp(0, len.saturating_sub(1) as isize) as usize;
        self.list_state.select(Some(selected), len);
    }

    fn clamp_selection(&mut self) {
        let len = self.state.todos.len();
        let selected = if len == 0 {
            None
        } else {
            Some(self.list_state.selected().unwrap_or(0).min(len - 1))
        };
        self.list_state.select(selected, len);
    }

    fn focus_input(&mut self, focused: bool) {
        self.input_focused = focused;
        self.input.set_focused(focused);
    }
}

fn terminal_mouse_intent(app: &mut TodoApp, mouse: MouseEvent) -> Option<Intent> {
    let position = Position::new(mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if app.input.area().contains(position) {
                app.focus_input(true);
                app.input
                    .mouse_down(position, mouse.modifiers.contains(KeyModifiers::SHIFT));
                return None;
            }
            if !app.list_area.contains(position) {
                return None;
            }
            let index = app
                .list_state
                .offset()
                .saturating_add(usize::from(position.y.saturating_sub(app.list_area.y)));
            let todo = app.state.todos.get(index)?;
            let intent = Intent::Toggle {
                id: todo.id,
                value: !todo.done,
            };
            app.focus_input(false);
            app.list_state.select(Some(index), app.state.todos.len());
            Some(intent)
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            app.input.mouse_drag(position);
            None
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.input.mouse_up();
            None
        }
        MouseEventKind::ScrollUp if app.list_area.contains(position) => {
            app.focus_input(false);
            app.select_relative(-1);
            None
        }
        MouseEventKind::ScrollDown if app.list_area.contains(position) => {
            app.focus_input(false);
            app.select_relative(1);
            None
        }
        _ => None,
    }
}

fn item_id(id: u64) -> String {
    format!("todo-{id}")
}

fn toggle_id(id: u64) -> String {
    format!("todo-{id}-toggle")
}

fn component_item(todo: &Todo) -> ListItem {
    ListItem::new(item_id(todo.id), todo.label.clone())
        .done(todo.done)
        .trailing(ListItemSlot::toggle(Toggle::new(
            toggle_id(todo.id),
            "Completed",
            todo.done,
            SET_DONE_ACTION,
        )))
        .delete_action(DELETE_ACTION)
}

fn validate_saved_state(state: &TodoState) -> Result<(), Box<dyn Error>> {
    if state.format != STATE_FORMAT
        || state.format_version != STATE_FORMAT_VERSION
        || state.state_schema_version != STATE_SCHEMA_VERSION
        || state.revision == 0
        || state.revision > MAX_SAFE_INTEGER
    {
        return Err("unsupported Todo save format; migrate or remove the state file".into());
    }
    let page = Page::new(
        "Todos",
        List::new(LIST_ID, state.todos.iter().map(component_item).collect()),
    );
    page.validate()?;
    if state.todos.iter().any(|todo| todo.id >= state.next_id) {
        return Err("Todo save has an invalid nextId".into());
    }
    Ok(())
}

fn save_state(path: &Path, state: &TodoState) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("todo.json");
    let mut temporary = tempfile::Builder::new()
        .prefix(&format!(".{name}."))
        .suffix(".tmp")
        .tempfile_in(parent)?;
    serde_json::to_writer_pretty(temporary.as_file_mut(), state)?;
    temporary.write_all(b"\n")?;
    temporary.as_file_mut().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn state_path() -> io::Result<PathBuf> {
    match std::env::var_os("UNPEEL_TODO_PATH") {
        Some(path) if !path.is_empty() => Ok(PathBuf::from(path)),
        _ => Ok(std::env::current_dir()?.join(DEFAULT_STATE_FILE)),
    }
}

fn terminal_intent(app: &mut TodoApp, key: KeyEvent) -> Option<Result<Intent, String>> {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Err("quit".to_owned()));
    }
    if key.code == KeyCode::Esc {
        return Some(Err("quit".to_owned()));
    }
    if key.code == KeyCode::Tab {
        app.focus_input(!app.input_focused);
        return None;
    }

    if app.input_focused {
        match key.code {
            KeyCode::Enter => {
                return Some(Ok(Intent::Add {
                    label: app.input.text().to_owned(),
                }));
            }
            KeyCode::Up => app.focus_input(false),
            KeyCode::Backspace => {
                app.input.handle(InputFieldAction::Backspace);
            }
            KeyCode::Delete => {
                app.input.handle(InputFieldAction::Delete);
            }
            KeyCode::Left => {
                app.input.handle(InputFieldAction::Left {
                    extend: key.modifiers.contains(KeyModifiers::SHIFT),
                    word: key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT),
                });
            }
            KeyCode::Right => {
                app.input.handle(InputFieldAction::Right {
                    extend: key.modifiers.contains(KeyModifiers::SHIFT),
                    word: key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT),
                });
            }
            KeyCode::Home => {
                app.input.handle(InputFieldAction::Home {
                    extend: key.modifiers.contains(KeyModifiers::SHIFT),
                });
            }
            KeyCode::End => {
                app.input.handle(InputFieldAction::End {
                    extend: key.modifiers.contains(KeyModifiers::SHIFT),
                });
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.input.handle(InputFieldAction::SelectAll);
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                app.input.handle(InputFieldAction::Insert(character));
            }
            _ => {}
        }
        return None;
    }

    if key.code == KeyCode::Char(' ') {
        return app.selected_todo().map(|todo| {
            Ok(Intent::Toggle {
                id: todo.id,
                value: !todo.done,
            })
        });
    }
    match key.code {
        KeyCode::Delete | KeyCode::Char('d') => app
            .selected_todo()
            .map(|todo| Ok(Intent::Delete { id: todo.id })),
        KeyCode::Char('i') => {
            app.focus_input(true);
            None
        }
        _ => match ListKeymap::new()
            .action_for_key(&key)
            .map(|action| app.list_state.navigate(action, app.state.todos.len()))
        {
            Some(ListNavigationOutcome::Back) => Some(Err("quit".to_owned())),
            Some(ListNavigationOutcome::Activate(index)) => {
                app.state.todos.get(index).map(|todo| {
                    Ok(Intent::Toggle {
                        id: todo.id,
                        value: !todo.done,
                    })
                })
            }
            Some(
                ListNavigationOutcome::None
                | ListNavigationOutcome::SelectionChanged(_)
                | ListNavigationOutcome::Scrolled(_),
            )
            | None => None,
        },
    }
}

#[cfg(feature = "ui-bridge")]
enum SemanticIntent {
    Model(Intent),
    Select(usize),
}

#[cfg(feature = "ui-bridge")]
fn semantic_intent(
    event: &unpeel_app_kit::UiEvent,
    app: &TodoApp,
) -> Result<SemanticIntent, String> {
    let node = event.action.node_id.as_str();
    let action = event.action.action.as_str();
    match (node, action, event.action.kind, &event.action.value) {
        (INPUT_ID, ADD_ACTION, UiEventKind::Submit, UiEventValue::Text(label)) => {
            Ok(SemanticIntent::Model(Intent::Add {
                label: label.clone(),
            }))
        }
        (LIST_ID, SELECT_ACTION, UiEventKind::Change, UiEventValue::Text(item)) => {
            let index = app
                .state
                .todos
                .iter()
                .position(|todo| item_id(todo.id) == *item)
                .ok_or_else(|| "Selected Todo no longer exists".to_owned())?;
            Ok(SemanticIntent::Select(index))
        }
        (_, SET_DONE_ACTION, UiEventKind::Change, UiEventValue::Bool(value)) => {
            let id = node
                .strip_prefix("todo-")
                .and_then(|node| node.strip_suffix("-toggle"))
                .and_then(|id| id.parse().ok())
                .ok_or_else(|| "Toggle target is not a Todo row".to_owned())?;
            if toggle_id(id) != node {
                return Err("Toggle target is not a canonical Todo id".to_owned());
            }
            Ok(SemanticIntent::Model(Intent::Toggle { id, value: *value }))
        }
        (_, DELETE_ACTION, UiEventKind::Change, UiEventValue::None) => {
            let id = node
                .strip_prefix("todo-")
                .and_then(|id| id.parse().ok())
                .ok_or_else(|| "Delete target is not a Todo row".to_owned())?;
            if item_id(id) != node {
                return Err("Delete target is not a canonical Todo id".to_owned());
            }
            Ok(SemanticIntent::Model(Intent::Delete { id }))
        }
        _ => Err("Action is not declared by the Todo Page".to_owned()),
    }
}

#[cfg(feature = "ui-bridge")]
fn drain_bridge(app: &mut TodoApp, bridge: &mut UiBridge) -> Result<(), Box<dyn Error>> {
    while let Some(message) = bridge.poll()? {
        match message {
            UiBridgeEvent::Attached {
                participant,
                client_id,
                ..
            } if std::env::var_os("UNPEEL_KITCHEN_SINK").is_some() => {
                let mut page = app.page();
                page.title = format!(
                    "Todos · {}",
                    participant
                        .display_name
                        .as_deref()
                        .unwrap_or(participant.id.as_str())
                );
                bridge.publish_to(
                    client_id,
                    VIEW_ID,
                    app.state.revision,
                    UiNode::page(ROOT_ID, page),
                )?;
            }
            UiBridgeEvent::Action { event, .. } => {
                let result: Result<Option<ModelChange>, String> =
                    if event.base_revision == app.state.revision {
                        semantic_intent(&event, app).and_then(|intent| match intent {
                            SemanticIntent::Model(intent) => app.commit(intent).map(Some),
                            SemanticIntent::Select(index) => {
                                app.list_state.select(Some(index), app.state.todos.len());
                                Ok(None)
                            }
                        })
                    } else {
                        Err(format!(
                            "Todo changed from revision {} to {}; retry the action",
                            event.base_revision, app.state.revision
                        ))
                    };
                let outcome = match result {
                    Ok(Some(change)) => {
                        let base = event.base_revision;
                        bridge.publish_delta(
                            VIEW_ID,
                            base,
                            app.state.revision,
                            change.ui_delta_operations(
                                app.selected_todo().map(|todo| item_id(todo.id)),
                            ),
                        )?;
                        UiEventOutcome::Applied
                    }
                    Ok(None) => UiEventOutcome::Applied,
                    Err(message) => UiEventOutcome::Rejected(message),
                };
                bridge.acknowledge(&event, outcome, app.state.revision)?;
            }
            UiBridgeEvent::Attached { .. }
            | UiBridgeEvent::Detached { .. }
            | UiBridgeEvent::Lifecycle { .. } => {
                // Presence and terminal visibility are maintained by UiBridge.
            }
        }
    }
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut app = TodoApp::load(state_path()?)?;
    save_state(&app.state_path, &app.state)?;

    #[cfg(feature = "ui-bridge")]
    let mut bridge = {
        let mut bridge = UiBridge::detect(
            AppMetadata::new("dev.unpeel.app-kit.todo", "Todo", env!("CARGO_PKG_VERSION"))
                .description("Canonical standalone and hosted App Kit example"),
        )?;
        bridge.publish(
            VIEW_ID,
            app.state.revision,
            UiNode::page(ROOT_ID, app.page()),
        )?;
        bridge
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = (|| -> Result<(), Box<dyn Error>> {
        loop {
            #[cfg(feature = "ui-bridge")]
            drain_bridge(&mut app, &mut bridge)?;

            #[cfg(feature = "ui-bridge")]
            let should_draw = bridge.should_render_terminal();
            #[cfg(not(feature = "ui-bridge"))]
            let should_draw = true;

            if should_draw {
                let page = app.page();
                let theme = PageTheme::for_theme(KitTheme::detected());
                terminal.draw(|frame| {
                    let areas = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(1)])
                        .split(frame.area());
                    app.list_area = page.layout(areas[0]).list;
                    frame.render_widget(
                        page.widget(&mut app.input, &mut app.list_state)
                            .theme(theme),
                        areas[0],
                    );
                    let help = if app.status.is_empty() {
                        "Tab focus · Enter/Space complete · d delete · Esc quit"
                    } else {
                        &app.status
                    };
                    frame.render_widget(
                        Paragraph::new(help).style(Style::new().fg(KitTheme::detected().subtle)),
                        areas[1],
                    );
                    if let Some(position) = app.input.cursor_position() {
                        frame.set_cursor_position(position);
                    }
                })?;
            }

            if !event::poll(Duration::from_millis(50))? {
                continue;
            }
            match event::read()? {
                Event::Key(key) => {
                    let Some(intent) = terminal_intent(&mut app, key) else {
                        continue;
                    };
                    match intent {
                        Err(message) if message == "quit" => break,
                        Err(message) => app.status = message,
                        Ok(intent) => {
                            #[cfg(feature = "ui-bridge")]
                            let base = app.state.revision;
                            match app.commit(intent) {
                                Ok(change) => {
                                    if matches!(change, ModelChange::Insert { .. }) {
                                        app.input.clear();
                                    }
                                    #[cfg(feature = "ui-bridge")]
                                    bridge.publish_delta(
                                        VIEW_ID,
                                        base,
                                        app.state.revision,
                                        change.ui_delta_operations(
                                            app.selected_todo().map(|todo| item_id(todo.id)),
                                        ),
                                    )?;
                                }
                                Err(message) => app.status = message,
                            }
                        }
                    }
                }
                Event::Paste(text) if app.input_focused => {
                    app.input.handle(InputFieldAction::InsertText(text));
                }
                Event::Mouse(mouse) => {
                    let Some(intent) = terminal_mouse_intent(&mut app, mouse) else {
                        continue;
                    };
                    #[cfg(feature = "ui-bridge")]
                    let base = app.state.revision;
                    match app.commit(intent) {
                        Ok(change) => {
                            #[cfg(not(feature = "ui-bridge"))]
                            let _ = change;
                            #[cfg(feature = "ui-bridge")]
                            bridge.publish_delta(
                                VIEW_ID,
                                base,
                                app.state.revision,
                                change.ui_delta_operations(
                                    app.selected_todo().map(|todo| item_id(todo.id)),
                                ),
                            )?;
                        }
                        Err(message) => app.status = message,
                    }
                }
                Event::Resize(_, _) => terminal.autoresize()?,
                _ => {}
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    result
}

fn main() {
    if let Err(error) = run() {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
        eprintln!("todo: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durable_reducer_restores_the_committed_model() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("todo.json");
        let mut app = TodoApp::load(path.clone()).unwrap();
        app.commit(Intent::Toggle { id: 2, value: true }).unwrap();
        app.commit(Intent::Add {
            label: "Persist me".to_owned(),
        })
        .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
        }

        let restored = TodoApp::load(path).unwrap();
        assert!(restored.state.todos[1].done);
        assert_eq!(restored.state.todos.last().unwrap().label, "Persist me");
        assert_eq!(restored.state.revision, 3);
    }

    #[test]
    fn terminal_row_click_targets_the_visible_todo_toggle() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = TodoApp::load(directory.path().join("todo.json")).unwrap();
        app.list_area = Rect::new(4, 8, 60, 10);
        let intent = terminal_mouse_intent(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 30,
                row: 9,
                modifiers: KeyModifiers::NONE,
            },
        )
        .unwrap();
        assert!(matches!(intent, Intent::Toggle { id: 2, value: true }));
        assert_eq!(app.list_state.selected(), Some(1));
        assert!(!app.input_focused);
    }

    #[cfg(feature = "ui-bridge")]
    #[test]
    fn shared_page_fixture_is_generated_from_the_canonical_todo_model() {
        let directory = tempfile::tempdir().unwrap();
        let app = TodoApp::load(directory.path().join("todo.json")).unwrap();
        let fixture = include_str!("../protocol/unpeel-ui-v1.ndjson")
            .lines()
            .nth(13)
            .unwrap();
        let message: unpeel_app_kit::UiMessage = serde_json::from_str(fixture).unwrap();
        let unpeel_app_kit::UiMessage::Snapshot(snapshot) = message else {
            panic!("Todo fixture must be a snapshot");
        };
        assert_eq!(snapshot.root, UiNode::page(ROOT_ID, app.page()));
    }
}
