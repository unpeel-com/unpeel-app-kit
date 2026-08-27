# unpeel-app-kit

Reusable, borderless [Ratatui](https://ratatui.rs/) components for
terminal-native Unpeel Apps. The crate is standalone-safe: Unpeel-specific
integration becomes an inert no-op or an explicit availability error when an
App runs outside an Unpeel-hosted session.

The kit currently provides:

| Component | Responsibility |
| --- | --- |
| `Explorer` | Flat current-directory navigation, filename filtering, selection, scrolling, hit-testing, and path drag sources |
| `PopupMenu` / `MenuItem` | Gray borderless context menu/dropdown with hover, keyboard selection, disabled items, and danger tones |
| `KitTheme` / `ColorScheme` | Shared dark/light defaults for selectable rows, menus, text, and scrollbars |
| `AgentBridge` | Async adjacent-agent discovery plus approval-free same-group path/text handoff |
| `DragSurface` | Frame-accurate semantic path regions for native drag-and-drop |
| `DraggablePath` / `DragSource<W>` | Small Ratatui wrappers for custom path drag sources |
| `VerticalScrollbar` | Shared proportional, capless scrollbar |
| `MarkdownTextArea` | Wrapped Markdown editing surface behind the `markdown-text-area` feature |

The crate owns reusable component behavior, not an App's event loop, key map,
commands, or surrounding chrome.

## Design conventions

### Selectable rows

Selectable list items use a full-width selection rectangle, with their label
inset by **two terminal cells** (`SELECTABLE_LEFT_PADDING`). Do not paint only
the text span: the gray selection background should continue through the
unused cells on both sides of the row. `Explorer`, `PopupMenu`, and the shared
theme defaults implement this convention.

Use `KitTheme::dark()` or `KitTheme::light()` when an App owns an appearance
setting. `KitTheme::detected()` and `ColorScheme::detect()` first honor
`UNPEEL_TUI_THEME=dark|light`, then the common `COLORFGBG` hint, and otherwise
choose dark. Both schemes leave the ordinary terminal background transparent;
only semantic surfaces such as selected rows and popup menus paint a
background.

```rust
use unpeel_app_kit::{ColorScheme, ExplorerTheme, KitTheme};

let scheme = ColorScheme::detect();
let palette = KitTheme::for_scheme(scheme);
let explorer_theme = ExplorerTheme::for_color_scheme(scheme);
```

## Add it to an App

The kit is currently a sibling development crate rather than a crates.io
package:

```toml
[dependencies]
unpeel-app-kit = { path = "../unpeel-app-kit" }
```

It targets Ratatui `0.30`. Enable the editor only in Apps that need it:

```toml
unpeel-app-kit = { path = "../unpeel-app-kit", features = ["markdown-text-area"] }
```

## Native path dragging

Path dragging does not capture terminal mouse input. Instead, `DragSurface`
publishes a short-lived semantic map between Ratatui terminal rectangles and
Host-local files or directories. Unpeel performs the native point-to-cell hit
test, starts a normal platform file drag, and pastes a shell-quoted path when
the destination is another Unpeel terminal.

### Ratatui usage

```rust
use ratatui::{layout::Rect, text::Line};
use unpeel_app_kit::{DragSurface, DraggablePath};

let mut paths = DragSurface::detect();

// Once per rendered frame:
paths.begin_frame();
terminal.draw(|frame| {
    frame.render_widget(
        DraggablePath::new(
            &mut paths,
            "/tmp/a folder",
            Line::from("a folder"),
        ),
        Rect::new(2, 4, 30, 1),
    );
})?;
paths.commit()?;

// From idle event-loop ticks so the map does not expire:
paths.heartbeat()?;
# Ok::<(), std::io::Error>(())
```

`DraggablePath` makes only the visible label draggable. `DragSource<W>` wraps
an arbitrary Ratatui widget and maps its full rectangle. `DragSurface::register`
is the lower-level primitive for custom or stateful components.

Call `commit` only after the terminal draw succeeds. Coordinates then describe
the frame the user can actually see. Maps are atomic, capped at 64 KiB, expire
at the receiver after five seconds, and are removed on clean shutdown. Paths
must be absolute and are revalidated by Unpeel immediately before dragging.

This contract is intentionally local-Host-only. A remote Host path is not a
Controller-local file URL and must never be advertised as one. The map carries
semantic paths rather than preformatted terminal text, leaving the receiving
surface free to paste a path today or handle an App-to-App path drop later.

## Explorer

`Explorer` is a flat, borderless current-directory browser for Unpeel Apps.
It follows the useful interaction model of
[`ratatui-explorer`](https://github.com/tatounee/ratatui-explorer)—a `../`
entry, directory suffixes, parent/child navigation, paging, hidden-file
toggling, a borderless current-folder filter, and full-row selection—while
integrating `DragSurface` directly.
The current-directory header and every visible file or folder row publish an
absolute Host-local path, so the receiving surface decides whether that path
becomes terminal text, a file URL, or a future App-specific drop.

```rust
use unpeel_app_kit::{DragSurface, Explorer, ExplorerInput};

let mut explorer = Explorer::new(".")?;
let mut drags = DragSurface::detect();

drags.begin_frame();
terminal.draw(|frame| {
    frame.render_widget(explorer.widget(&mut drags), frame.area());
})?;
drags.commit()?;

// Convert keys with whichever terminal backend the App already uses.
explorer.handle(ExplorerInput::Down)?;
explorer.handle(ExplorerInput::Open)?;
explorer.handle(ExplorerInput::FocusFilter)?;
explorer.handle(ExplorerInput::FilterCharacter('r'))?;
# Ok::<(), std::io::Error>(())
```

The component never enables mouse capture. A standalone App should leave the
pointer with the terminal emulator when it wants native path dragging.

`Explorer::handle` returns `ExplorerEvent`; directory navigation stays inside
the component, while file activation remains App-owned. `ExplorerTheme`
contains styles and spacing only. There is intentionally no `Block`, border,
or mandatory background, so Apps can compose it without inherited chrome.

## Popup menu

`PopupMenu<T>` is the shared context-menu/dropdown surface. It paints a flat
gray panel with one cell of outer breathing room, two cells before every
label, and a full-row hover/keyboard selection. The dark preset gets lighter
on selection; the light preset gets darker. It clamps to the terminal,
scrolls long menus, skips disabled entries during keyboard navigation, and
supports muted and danger tones without stock Ratatui borders.

```rust
use ratatui::layout::Position;
use unpeel_app_kit::{MenuItem, MenuTheme, PopupMenu};

let mut menu = PopupMenu::new(
    Position::new(mouse.column, mouse.row),
    [
        MenuItem::new("Send to agent", Action::Send),
        MenuItem::new("Unavailable", Action::Unavailable).disabled(),
        MenuItem::new("Delete", Action::Delete).danger(),
    ],
)
.with_theme(MenuTheme::detected());

menu.hover_at(Position::new(mouse.column, mouse.row));
menu.move_selection(1);
menu.render(frame);
```

## Send to agent

`AgentBridge` implements the shared Unpeel App handoff used by context menus.
Call `refresh()` when the App starts and again when a menu opens; it probes off
the UI thread and caches an honest target label. `send_text()` and
`send_path()` re-resolve at activation time, prefer an adjacent same-group
agent, and paste with `submit: false`, allowing the user to finish the prompt.
Outside Unpeel they return `AgentError`. `clipboard_sequence()` provides an
OSC 52 copy fallback.

```rust
use unpeel_app_kit::{AgentBridge, clipboard_sequence};

let agent = AgentBridge::new();
agent.refresh();
if agent.label().is_some() {
    let receiving_label = agent.send_path("/tmp/example")?;
} else {
    print!("{}", clipboard_sequence("/tmp/example"));
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Scrollbar

`VerticalScrollbar` is a stateless Ratatui widget shared by row-based views.
It accepts total content rows, visible viewport rows, and the requested top-row
offset; clamps the offset; and converts those values into Ratatui's expected
scroll-position count. The default is the capless Unpeel style (`│` track,
`┃` proportional thumb), with builder methods for per-App colors and symbols.

```rust
use ratatui::style::{Color, Style};
use unpeel_app_kit::VerticalScrollbar;

frame.render_widget(
    VerticalScrollbar::new(total_rows, viewport_rows, scroll_top)
        .track_style(Style::new().fg(Color::DarkGray))
        .thumb_style(Style::new().fg(Color::Gray)),
    scrollbar_area,
);
```

## Markdown text area

`MarkdownTextArea` wraps `tui-textarea-2` with the visual behavior shared by
Unpeel Markdown Apps: word-or-glyph wrapping, a continuation-aware line-number
gutter, native terminal cursor placement, wrapped mouse hit-testing, drag
auto-scroll, and the shared proportional scrollbar. Markdown commands and
syntax highlighting remain App-owned and can use `text_area_mut()` (or normal
`DerefMut` coercion) to reach the underlying editor.

Enable the `markdown-text-area` crate feature for this component; drag sources
and `VerticalScrollbar` stay lightweight for Apps that do not edit text.

```rust
use ratatui::layout::Position;
use ratatui::style::{Color, Style};
use unpeel_app_kit::{MarkdownTextArea, MarkdownTextAreaStyle};

let style = MarkdownTextAreaStyle {
    current_gutter: Style::new().fg(Color::Gray),
    gutter: Style::new().fg(Color::DarkGray),
    scrollbar_track: Style::new().fg(Color::DarkGray),
    scrollbar_thumb: Style::new().fg(Color::Gray),
    ..MarkdownTextAreaStyle::default()
};
let mut editor = MarkdownTextArea::new(["# Hello"], style);

editor.render(frame, editor_area, true);
if editor.contains(Position::new(mouse.column, mouse.row)) {
    let (line, column) = editor.hit_test(Position::new(mouse.column, mouse.row));
    // Move or extend the App's selection to (line, column).
}
```

## Development

```sh
cargo test --no-default-features
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

The Explorer and drag tests render into Ratatui's `Buffer`/`TestBackend` and
assert terminal-cell regions directly; they do not require Unpeel to be
running.
