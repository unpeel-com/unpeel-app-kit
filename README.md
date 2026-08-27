# unpeel-tui-kit

Reusable, borderless [Ratatui](https://ratatui.rs/) components for
terminal-native Unpeel Apps. The crate is standalone-safe: Unpeel-specific
integration becomes an inert no-op when an App runs in Ghostty, another
terminal emulator, or a test backend.

The kit currently provides:

| Component | Responsibility |
| --- | --- |
| `Explorer` | Flat current-directory navigation, selection, theming, scrolling, hit-testing, and path drag sources |
| `DragSurface` | Frame-accurate semantic path regions for native drag-and-drop |
| `DraggablePath` / `DragSource<W>` | Small Ratatui wrappers for custom path drag sources |
| `VerticalScrollbar` | Shared proportional, capless scrollbar |
| `MarkdownTextArea` | Wrapped Markdown editing surface behind the `markdown-text-area` feature |

The crate owns reusable component behavior, not an App's event loop, key map,
commands, or surrounding chrome.

## Add it to an App

The kit is currently a sibling development crate rather than a crates.io
package:

```toml
[dependencies]
unpeel-tui-kit = { path = "../unpeel-tui-kit" }
```

It targets Ratatui `0.30`. Enable the editor only in Apps that need it:

```toml
unpeel-tui-kit = { path = "../unpeel-tui-kit", features = ["markdown-text-area"] }
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
use unpeel_tui_kit::{DragSurface, DraggablePath};

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
toggling, and full-row selection—while integrating `DragSurface` directly.
The current-directory header and every visible file or folder row publish an
absolute Host-local path, so the receiving surface decides whether that path
becomes terminal text, a file URL, or a future App-specific drop.

```rust
use unpeel_tui_kit::{DragSurface, Explorer, ExplorerInput};

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
# Ok::<(), std::io::Error>(())
```

The component never enables mouse capture. A standalone App should leave the
pointer with the terminal emulator when it wants native path dragging.

`Explorer::handle` returns `ExplorerEvent`; directory navigation stays inside
the component, while file activation remains App-owned. `ExplorerTheme`
contains styles and spacing only. There is intentionally no `Block`, border,
or mandatory background, so Apps can compose it without inherited chrome.

## Scrollbar

`VerticalScrollbar` is a stateless Ratatui widget shared by row-based views.
It accepts total content rows, visible viewport rows, and the requested top-row
offset; clamps the offset; and converts those values into Ratatui's expected
scroll-position count. The default is the capless Unpeel style (`│` track,
`┃` proportional thumb), with builder methods for per-App colors and symbols.

```rust
use ratatui::style::{Color, Style};
use unpeel_tui_kit::VerticalScrollbar;

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
use unpeel_tui_kit::{MarkdownTextArea, MarkdownTextAreaStyle};

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
