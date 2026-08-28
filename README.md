# unpeel-app-kit

Reusable, borderless [Ratatui](https://ratatui.rs/) components for
terminal-native Unpeel Apps. The crate is standalone-safe: Unpeel-specific
integration becomes an inert no-op or an explicit availability error when an
App runs outside an Unpeel-hosted session.

The kit currently provides:

| Component | Responsibility |
| --- | --- |
| `Explorer` | Flat current-directory navigation, filename filtering, selection, scrolling, hit-testing, and path drag sources |
| `InputField` | Borderless single-line editing with a native cursor, keyboard/mouse selection, word movement, and horizontal scrolling |
| `PopupMenu` / `MenuItem` | Gray borderless context menu/dropdown with hover, keyboard selection, disabled items, and danger tones |
| `KitTheme` / `ColorScheme` | Shared dark/light defaults for selectable rows, menus, text, and scrollbars |
| `DoubleClickTracker` | Target-aware double-click detection shared by mouse-driven Apps |
| `KeyboardEnhancementGuard` | Scoped unambiguous Escape delivery on capable terminals |
| `AgentBridge` | Async adjacent-agent discovery plus approval-free same-group path/text handoff |
| `EditorBridge` | Open a file or folder with Unpeel's preferred editor, with a standalone platform fallback |
| `DragSurface` | Frame-accurate semantic path regions for native drag-and-drop |
| `DraggablePath` / `DragSource<W>` | Small Ratatui wrappers for custom path drag sources |
| `display_path_from_root` | Render paths project-relative without weakening absolute semantic path operations |
| `VerticalScrollbar` | Shared proportional, capless scrollbar |
| `MarkdownTextArea` | Wrapped Markdown editing surface behind the `markdown-text-area` feature |

The crate owns reusable component behavior, not an App's event loop, key map,
commands, or surrounding chrome.

## Explorer filter focus

Explorer Apps should map every unmodified printable character to
`ExplorerInput::FilterCharacter`, even while the file list has focus. The
component focuses the filter before inserting that first character, so users
can type immediately without pressing `/` first. Paste through
`insert_filter_text` has the same behavior.

The filter remains directly mouse-accessible through `filter_mouse_down`,
which focuses it and places its native text cursor. For keyboard-only access,
the shared convention is that Up from the first list row sends `FocusFilter`,
while Down or Tab from the filter sends `BlurFilter` back to the list.

## Design conventions

### Selectable rows

Selectable list items use a full-width selection rectangle, with their label
inset by **two terminal cells** (`SELECTABLE_LEFT_PADDING`). Do not paint only
the text span: the gray selection background should continue through the
unused cells on both sides of the row. `Explorer` and the shared theme defaults
implement this convention. Popup menus are the deliberate exception: their
labels and gray selection rows are edge-aligned with no default cell padding.

Pinned navigation actions such as `← Back` may keep a full-width hit target,
but they are not selected list items: render their ordinary row background
transparent.

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

### Double-click activation

Use a logical item identity rather than terminal coordinates so a redraw
between presses cannot activate another row:

```rust
use unpeel_app_kit::DoubleClickTracker;

let mut clicks = DoubleClickTracker::new();
if clicks.click(item_id) {
    open_item(item_id);
}
```

The default interval is 500 ms. A different target replaces the pending
click, and a completed double click resets the sequence.

### Reliable Escape

Create a `KeyboardEnhancementGuard` after entering the terminal and keep it
alive for the event loop. It requests explicit Escape encoding from terminals
that support the progressive keyboard protocol, then restores the preceding
mode on drop:

```rust
use unpeel_app_kit::KeyboardEnhancementGuard;

let _keyboard = KeyboardEnhancementGuard::enter()?;
```

## Install in an App

The kit is currently a Git repository rather than a crates.io package. Check
it out once beside the Apps that use it, then add the local dependency:

```sh
mkdir -p ~/Dev && cd ~/Dev
git clone https://github.com/unpeel-com/unpeel-app-kit.git
cd your-ratatui-app
cargo add unpeel-app-kit --path ../unpeel-app-kit
```

The resulting dependency is:

```toml
[dependencies]
unpeel-app-kit = { path = "../unpeel-app-kit" }
```

It targets Ratatui `0.30`. Enable the Markdown text area only in Apps that
need it:

```toml
unpeel-app-kit = { path = "../unpeel-app-kit", features = ["markdown-text-area"] }
```

## Native path dragging

Path dragging does not capture terminal mouse input. Instead, `DragSurface`
publishes a short-lived semantic map between Ratatui terminal rectangles and
Host-local files or directories. Unpeel performs the native point-to-cell hit
test, starts a normal platform file drag, and pastes a shell-quoted path when
the destination is another Unpeel terminal. Pasted text is relative to that
Session's project root when possible, uses `~/…` elsewhere under the user's
home, and stays absolute only outside both roots.

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
entry below the root, directory suffixes, parent/child navigation, paging,
hidden-file toggling, a borderless current-folder filter, and full-row
selection—while integrating `DragSurface` directly.
The current-directory header and every visible file or folder row publish an
absolute Host-local path, so the receiving surface decides whether that path
becomes terminal text, a file URL, or a future App-specific drop.

```rust
use unpeel_app_kit::{DragSurface, Explorer, ExplorerInput};

// Project-facing Apps should keep navigation inside their launch directory.
let mut explorer = Explorer::scoped(".")?;
// File-opening Apps can retain folders but admit only relevant file types.
explorer.set_file_extensions(["md"])?;
// Search-style launchers can also hide folders with no matching descendants.
explorer.set_prune_unmatched_directories(true)?;
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

`Explorer::scoped` canonicalizes its initial directory and makes that path a
hard boundary: the root omits `../`, parent actions are no-ops there, and a
symlink cannot navigate outside it. `Explorer::new` remains available for an
explicitly unbounded filesystem browser. Passing a file to either constructor
opens its parent; for `scoped`, that parent becomes the boundary.
`set_file_extensions` / `with_file_extensions` provide a case-insensitive
file-type policy for open dialogs; directories always remain visible for
navigation by default. `set_prune_unmatched_directories(true)` changes that
policy for focused browsers: a directory remains visible only when a
non-hidden matching file exists somewhere below it. Toggling hidden files
recomputes those results, and recursive discovery never follows directory
symlinks.

The filter uses the shared `InputField`. Apps can map the extended
`FilterLeft`, `FilterRight`, `FilterHome`, `FilterEnd`, `FilterDelete`, and
`FilterSelectAll` actions, forward paste through `insert_filter_text`, and
forward mouse presses/drags through the `filter_mouse_*` methods. After
rendering, apply `filter_cursor_position()` to the Ratatui frame when present.

The component never enables mouse capture. A standalone App should leave the
pointer with the terminal emulator when it wants native path dragging.

`Explorer::handle` returns `ExplorerEvent`; directory navigation stays inside
the component, while file activation remains App-owned. `ExplorerTheme`
contains styles and spacing only. There is intentionally no `Block`, border,
or mandatory background, so Apps can compose it without inherited chrome.

## Project paths and preferred editor

Keep filesystem operations and drag maps absolute, then shorten only visible
labels with `display_path_from_root`. It returns `.` for the project root,
uses a repository-relative path for descendants, and leaves paths outside the
root absolute instead of inventing misleading `..` segments.

`EditorBridge::open` is the shared action for an **Open in editor** menu item.
Inside a local Unpeel App Session it asks the owning Unpeel instance to use
the editor selected in Settings. When the same App runs standalone—or against
an older host without that endpoint—it uses the platform's ordinary file
opener. Files and folders must exist; relative inputs are resolved against the
App's current directory before they cross the bridge.

```rust
use unpeel_app_kit::{EditorBridge, display_path_from_root};

let label = display_path_from_root("/work/project/src/main.rs", "/work/project");
assert_eq!(label, "src/main.rs");
EditorBridge::open("/work/project/src/main.rs")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Input field

`InputField` is the shared single-line primitive for filters and lightweight
forms. It owns its Unicode-safe edit cursor and selection, replaces selected
text on insert/delete, supports character and word movement with optional
Shift extension, selects a logical word on double-click, supports drag and
Shift-click selection, and scrolls horizontally to keep the cursor visible.
Its gray selection style comes from the same dark/light `KitTheme` defaults as
selectable rows.

```rust
use ratatui::layout::Position;
use unpeel_app_kit::{InputField, InputFieldAction, InputFieldTheme};

let mut input = InputField::new("Filter files")
    .with_prompt("/ ")
    .with_theme(InputFieldTheme::detected());
input.set_focused(true);
input.handle(InputFieldAction::InsertText("readme".into()));
input.handle(InputFieldAction::Left {
    extend: true,
    word: true,
});

frame.render_widget(input.widget(), input_area);
if let Some(position) = input.cursor_position() {
    frame.set_cursor_position(position);
}

// Forward terminal mouse reporting when the App enables it.
input.mouse_down(Position::new(mouse.column, mouse.row), shift_held);
input.mouse_drag(Position::new(mouse.column, mouse.row));
input.mouse_up();
```

`InputField::handle` reports whether editing or selection state changed;
`text()` is the resulting value and `selected_text()` exposes the active
selection. The component does not choose a terminal backend, key bindings,
mouse-capture mode, clipboard policy, or form submission behavior.

## Popup menu

`PopupMenu<T>` is the shared context-menu/dropdown surface. It paints a flat
gray panel with no outer, left, or right cell padding by default and a full-row
hover/keyboard selection. The panel sizes to its longest label (plus a
scrollbar only when needed). The dark preset gets lighter on selection; the
light preset gets darker. It clamps to the terminal, scrolls long menus, skips
disabled entries during keyboard navigation, and supports muted and danger
tones without stock Ratatui borders.

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
