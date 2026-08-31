//! Standalone Markdown editor and hosted semantic-component demo.

use std::error::Error;
use std::fs;
#[cfg(unix)]
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Position};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};
use tui_textarea::{Input as TextInput, Key as TextKey};
use unpeel_app_kit::{
    AppMetadata, MarkdownEditor, MarkdownEditorConfig, MarkdownEditorEvent,
    MarkdownEditorInteraction, MarkdownEditorStyle, MarkdownPresentation, UiBridge, UiBridgeEvent,
    UiDeltaOperation, UiEventOutcome, UiNode,
};

const STATE_FORMAT: &str = "unpeel.app-kit.example.markdown";
const STATE_FORMAT_VERSION: u32 = 1;
const DEFAULT_STATE_FILE: &str = ".unpeel-markdown.json";
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const VIEW_ID: &str = "main";
const EDITOR_ID: &str = "markdown-editor";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarkdownState {
    format: String,
    format_version: u32,
    revision: u64,
    text: String,
    presentation: MarkdownPresentation,
}

impl Default for MarkdownState {
    fn default() -> Self {
        Self {
            format: STATE_FORMAT.to_owned(),
            format_version: STATE_FORMAT_VERSION,
            revision: 1,
            text: concat!(
                "# App Kit Markdown\n\n",
                "This document is edited by the **same process** in Ratatui and SwiftUI.\n\n",
                "- Switch the kitchen sink between terminal, native, and split views.\n",
                "- Disconnect and reconnect to exercise revision resume.\n",
                "- Let a scoped agent append a line through the semantic channel.\n",
            )
            .to_owned(),
            presentation: MarkdownPresentation::Source,
        }
    }
}

struct MarkdownApp {
    state: MarkdownState,
    state_path: PathBuf,
    editor: MarkdownEditor<'static>,
    interaction: MarkdownEditorInteraction,
    status: String,
}

impl MarkdownApp {
    fn load(state_path: PathBuf) -> Result<Self, Box<dyn Error>> {
        let state = match fs::read(&state_path) {
            Ok(bytes) => {
                let state: MarkdownState = serde_json::from_slice(&bytes)?;
                validate_saved_state(&state)?;
                state
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => MarkdownState::default(),
            Err(error) => return Err(error.into()),
        };
        let mut editor = MarkdownEditor::new(
            document_lines(&state.text),
            MarkdownEditorStyle {
                cursor_line: Style::new().bg(Color::Rgb(27, 31, 38)),
                cursor: Style::new().fg(Color::White),
                selection: Style::new().bg(Color::Rgb(47, 78, 120)),
                gutter: Style::new().fg(Color::DarkGray),
                current_gutter: Style::new().fg(Color::Cyan),
                scrollbar_track: Style::new().fg(Color::Rgb(40, 44, 52)),
                scrollbar_thumb: Style::new().fg(Color::Rgb(100, 108, 122)),
            },
        );
        editor
            .text_area_mut()
            .set_placeholder_text("Write Markdown… Type / on an empty line for blocks.");
        Ok(Self {
            state,
            state_path,
            editor,
            interaction: MarkdownEditorInteraction::new(),
            status: String::new(),
        })
    }

    fn config(&self) -> MarkdownEditorConfig {
        MarkdownEditorConfig::new(EDITOR_ID)
            .title("Kitchen Sink.md")
            .presentation(self.state.presentation)
    }

    fn node(&self) -> UiNode {
        self.editor.ui_node(&self.config())
    }

    fn personalized_node(&self, participant_name: &str) -> UiNode {
        self.editor.ui_node(
            &MarkdownEditorConfig::new(EDITOR_ID)
                .title(format!("Kitchen Sink.md · {participant_name}"))
                .presentation(self.state.presentation),
        )
    }

    fn commit_projection_change(&mut self) -> Result<(u64, u64), Box<dyn Error>> {
        let base = self.state.revision;
        self.state.revision = base
            .checked_add(1)
            .filter(|revision| *revision <= MAX_SAFE_INTEGER)
            .ok_or("revision space is exhausted")?;
        self.state.text = markdown_document(self.editor.lines());
        save_state(&self.state_path, &self.state)?;
        self.status = format!("Saved revision {}", self.state.revision);
        Ok((base, self.state.revision))
    }

    fn save_without_revision(&mut self) -> Result<(), Box<dyn Error>> {
        self.state.text = markdown_document(self.editor.lines());
        save_state(&self.state_path, &self.state)?;
        self.status = format!("Saved {}", self.state_path.display());
        Ok(())
    }
}

fn document_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.split('\n').map(ToOwned::to_owned).collect();
    if text.ends_with('\n') {
        lines.pop();
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn markdown_document(lines: &[String]) -> String {
    lines.join("\n")
}

fn validate_saved_state(state: &MarkdownState) -> Result<(), Box<dyn Error>> {
    if state.format != STATE_FORMAT
        || state.format_version != STATE_FORMAT_VERSION
        || state.revision == 0
        || state.revision > MAX_SAFE_INTEGER
    {
        return Err("unsupported Markdown save format; migrate or remove the state file".into());
    }
    Ok(())
}

fn save_state(path: &Path, state: &MarkdownState) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("markdown.json");
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
    match std::env::var_os("UNPEEL_MARKDOWN_PATH") {
        Some(path) if !path.is_empty() => Ok(PathBuf::from(path)),
        _ => Ok(std::env::current_dir()?.join(DEFAULT_STATE_FILE)),
    }
}

fn text_input(key: KeyEvent) -> TextInput {
    let mapped = match key.code {
        KeyCode::Char(character) => TextKey::Char(character),
        KeyCode::F(number) => TextKey::F(number),
        KeyCode::Backspace => TextKey::Backspace,
        KeyCode::Enter => TextKey::Enter,
        KeyCode::Left => TextKey::Left,
        KeyCode::Right => TextKey::Right,
        KeyCode::Up => TextKey::Up,
        KeyCode::Down => TextKey::Down,
        KeyCode::Tab | KeyCode::BackTab => TextKey::Tab,
        KeyCode::Delete => TextKey::Delete,
        KeyCode::Home => TextKey::Home,
        KeyCode::End => TextKey::End,
        KeyCode::PageUp => TextKey::PageUp,
        KeyCode::PageDown => TextKey::PageDown,
        KeyCode::Esc => TextKey::Esc,
        _ => TextKey::Null,
    };
    TextInput {
        key: mapped,
        ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
        alt: key.modifiers.contains(KeyModifiers::ALT),
        shift: key.modifiers.contains(KeyModifiers::SHIFT),
    }
}

fn publish_root(
    app: &MarkdownApp,
    bridge: &mut UiBridge,
    base_revision: u64,
) -> Result<(), Box<dyn Error>> {
    bridge.publish_delta(
        VIEW_ID,
        base_revision,
        app.state.revision,
        vec![UiDeltaOperation::ReplaceRoot { root: app.node() }],
    )?;
    Ok(())
}

fn drain_bridge(app: &mut MarkdownApp, bridge: &mut UiBridge) -> Result<(), Box<dyn Error>> {
    while let Some(message) = bridge.poll()? {
        match message {
            UiBridgeEvent::Attached {
                participant,
                client_id,
                ..
            } if std::env::var_os("UNPEEL_KITCHEN_SINK").is_some() => {
                let name = participant
                    .display_name
                    .as_deref()
                    .unwrap_or(participant.id.as_str());
                bridge.publish_to(
                    client_id,
                    VIEW_ID,
                    app.state.revision,
                    app.personalized_node(name),
                )?;
            }
            UiBridgeEvent::Action { event, .. } => {
                let config = app.config();
                let outcome = match app
                    .editor
                    .handle_ui_event(app.state.revision, &config, &event)
                {
                    Ok(Some(MarkdownEditorEvent::TextChanged { changed: true }))
                    | Ok(Some(MarkdownEditorEvent::Undo { changed: true }))
                    | Ok(Some(MarkdownEditorEvent::Redo { changed: true })) => {
                        let (base, _) = app.commit_projection_change()?;
                        publish_root(app, bridge, base)?;
                        UiEventOutcome::Applied
                    }
                    Ok(Some(MarkdownEditorEvent::PresentationRequested(presentation))) => {
                        app.state.presentation = presentation;
                        let (base, _) = app.commit_projection_change()?;
                        publish_root(app, bridge, base)?;
                        UiEventOutcome::Applied
                    }
                    Ok(Some(MarkdownEditorEvent::SaveRequested)) => {
                        app.save_without_revision()?;
                        UiEventOutcome::Applied
                    }
                    Ok(Some(MarkdownEditorEvent::TextChanged { changed: false }))
                    | Ok(Some(MarkdownEditorEvent::Undo { changed: false }))
                    | Ok(Some(MarkdownEditorEvent::Redo { changed: false }))
                    | Ok(Some(MarkdownEditorEvent::SelectionChanged)) => UiEventOutcome::Applied,
                    Ok(None) => {
                        UiEventOutcome::Rejected("Action targets a different component".to_owned())
                    }
                    Err(error) => UiEventOutcome::Rejected(error.to_string()),
                };
                bridge.acknowledge(&event, outcome, app.state.revision)?;
            }
            UiBridgeEvent::Attached { .. }
            | UiBridgeEvent::Detached { .. }
            | UiBridgeEvent::Lifecycle { .. } => {}
        }
    }
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut app = MarkdownApp::load(state_path()?)?;
    app.save_without_revision()?;
    let mut bridge = UiBridge::detect(
        AppMetadata::new(
            "dev.unpeel.app-kit.markdown",
            "Markdown",
            env!("CARGO_PKG_VERSION"),
        )
        .description("Standalone Ratatui and hosted SwiftUI Markdown editor"),
    )?;
    bridge.publish(VIEW_ID, app.state.revision, app.node())?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = (|| -> Result<(), Box<dyn Error>> {
        loop {
            drain_bridge(&mut app, &mut bridge)?;
            if bridge.should_render_terminal() {
                terminal.draw(|frame| {
                    let areas = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(1),
                            Constraint::Min(1),
                            Constraint::Length(1),
                        ])
                        .split(frame.area());
                    frame.render_widget(
                        Paragraph::new(" App Kit Markdown").style(Style::new().fg(Color::Cyan)),
                        areas[0],
                    );
                    app.editor.render(frame, areas[1], true);
                    app.interaction.render_overlay(&app.editor, frame);
                    let controls =
                        "/ insert · drag/double/triple-click select · Ctrl-S save · Esc quit";
                    let help = if app.status.is_empty() {
                        controls.to_owned()
                    } else {
                        format!("{} · {controls}", app.status)
                    };
                    frame.render_widget(
                        Paragraph::new(help).style(Style::new().fg(Color::DarkGray)),
                        areas[2],
                    );
                })?;
            }

            if !event::poll(Duration::from_millis(50))? {
                continue;
            }
            match event::read()? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        break;
                    }
                    if key.code == KeyCode::Esc && !app.interaction.is_insert_menu_open() {
                        break;
                    }
                    if key.code == KeyCode::Char('s')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        app.save_without_revision()?;
                        continue;
                    }
                    let outcome = app
                        .interaction
                        .handle_input(&mut app.editor, text_input(key));
                    if outcome.text_changed() {
                        let (base, _) = app.commit_projection_change()?;
                        publish_root(&app, &mut bridge, base)?;
                    }
                }
                Event::Paste(text) => {
                    let outcome = app.interaction.handle_paste(&mut app.editor, &text);
                    if outcome.text_changed() {
                        let (base, _) = app.commit_projection_change()?;
                        publish_root(&app, &mut bridge, base)?;
                    }
                }
                Event::Mouse(mouse) => {
                    let position = Position::new(mouse.column, mouse.row);
                    let shift = mouse.modifiers.contains(KeyModifiers::SHIFT);
                    let outcome = match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            app.interaction
                                .pointer_down(&mut app.editor, position, shift)
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            app.interaction.pointer_drag(&mut app.editor, position)
                        }
                        MouseEventKind::Moved => {
                            app.interaction.pointer_move(&mut app.editor, position)
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            app.interaction.pointer_up(&mut app.editor)
                        }
                        MouseEventKind::ScrollDown => {
                            app.interaction
                                .pointer_scroll(&mut app.editor, position, 2, shift)
                        }
                        MouseEventKind::ScrollUp => {
                            app.interaction
                                .pointer_scroll(&mut app.editor, position, -2, shift)
                        }
                        _ => Default::default(),
                    };
                    if outcome.text_changed() {
                        let (base, _) = app.commit_projection_change()?;
                        publish_root(&app, &mut bridge, base)?;
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
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    result
}

fn main() {
    if let Err(error) = run() {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            DisableBracketedPaste,
            DisableMouseCapture,
            LeaveAlternateScreen
        );
        eprintln!("markdown: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durable_document_restores_after_a_committed_edit() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("markdown.json");
        let mut app = MarkdownApp::load(path.clone()).unwrap();
        app.editor.text_area_mut().insert_str("persisted ");
        app.commit_projection_change().unwrap();

        let restored = MarkdownApp::load(path).unwrap();
        assert!(markdown_document(restored.editor.lines()).starts_with("persisted "));
        assert_eq!(restored.state.revision, 2);
    }
}
