# unpeel-app-kit

An opinionated [Ratatui](https://ratatui.rs/) component library for building
normal standalone TUI Apps. Use the components in an ordinary terminal
process, ship the binary anywhere Ratatui runs, and keep ownership of the event
loop, commands, and model. Unpeel is not required at build time or runtime.

## Quickstart: Todo

The canonical demo is a normal Ratatui Todo App:

```sh
cargo run --example todo
```

Type a todo and press Enter. Click a row to toggle it; Tab moves between the
Input and List, Enter or Space toggles the selected row, `d` deletes it, and
Escape exits. The App saves
its versioned model atomically to `.unpeel-todo.json` in the current directory
(`UNPEEL_TODO_PATH` overrides the location), so an always-on process can be
stopped and restored without making a renderer the state owner.

The complete TUI also builds with every hosted module removed:

```sh
cargo run --example todo --no-default-features
```

With the default `ui-bridge` feature, that same binary detects a Host-injected
endpoint and additionally publishes its Page → List → ListItem tree. SwiftUI
renders native list rows and toggles through `PageView`; web does the same with
`PageRenderer`. With no injected endpoint, bridge detection is inert and the
App is simply the standalone TUI above.

The same focus engine also powers App Kit Lists and Explorer. Rows add a
closed, UITableViewCell-style role instead of arbitrary child widgets:
checkbox Toggle, navigation Disclosure, selection Checkmark, ordinary or
destructive command, or static information. Enter invokes the focused primary
role; Space invokes a Toggle and otherwise pages down; Escape routes through
the App-owned Page back action. Disclosure never makes the renderer the
router—the reducer publishes the next durable Page for terminal, SwiftUI, and
web together.

## Kitchen Sink mini-host (macOS)

The repository also includes a self-contained macOS 14+ SwiftUI test rig that
exercises the complete hosted loop without Unpeel installed:

```sh
swift/Examples/KitchenSink/run-app.sh
```

It builds and launches the Todo, Markdown, Media, and (when the sibling guest
artifact exists) Surface Planets plus Canvas + Controls examples in real libghostty PTYs rendered
through Metal, creates private per-session Unix
sockets and signing keys, and attaches the sibling `UnpeelAppKitUI` renderer
with Host-minted scoped tokens. A `WKWebView` loads the repository's actual
TypeScript/DOM renderers against the same live snapshot and action stream.
The launcher wraps the SwiftPM executable in a generated, ad-hoc signed `.app`
under `.build`, giving Launch Services a stable bundle identity and keeping
keyboard focus in the harness. Direct `swift run` remains supported and adopts
macOS's regular foreground activation policy itself.
Every session can switch among Terminal, Native, Web, and a three-way Split.
An expandable, live component-tree inspector shows node ids, slots, actions,
values, and the authoritative revision beside any presentation. Harness
controls cover child kill/restart and durable restore, renderer
disconnect/resume, a second agent participant with configurable grants,
presence and acknowledgements, participant-specific `publish_to` projections,
and a raw snapshot-versus-delta indicator.

The two Surface sessions exercise the composed canvas path as well. Canvas +
Controls puts the closed CanvasPage/Button toolbar over the same local GPU
scene in Ratatui, SwiftUI, and DOM. When the sibling
`UnpeelSurfaceKit` XCFramework and `web/pkg` artifacts exist, the mini-host
injects a private `UNPEEL_SURFACE_SOCKET`, retains the app's USRF resources and
latest scene, and fans those exact packets to a macOS CAMetalLayer presenter
and the WebGPU presenter. Terminal mode composites the same local Metal scene
behind transparent Ghostty cells. The hosted producer runs Surface in
`retained-only` mode, so it never creates a duplicate wgpu/Kitty projection.
Every view renders locally; the broker never rasterizes or transports frames.
Missing Surface artifacts simply remove the capability and preserve the
complete TUI fallback.

The harness is an independent SwiftPM executable package; it is not a library
dependency and its CI workflow is manual-only. See
[`swift/Examples/KitchenSink/README.md`](swift/Examples/KitchenSink/README.md)
for its mini-host boundaries and test flow.

Select the Markdown session to exercise the editor end to end. Both its PTY
and native view accept typing and text selection. In the terminal, drag to
select, double-click a word, or triple-click a line. Type `/` on an empty line
to open the closed block menu; keep typing to filter, use Up/Down and
Enter/Tab, and use Escape to remove the pending slash command. Backspace at a
heading, list, task, or quote marker converts that block back to plain text.
The same menu and Backspace rules are built into the AppKit and web renderers.
The terminal menu is a compact bordered shortcut/name/sample dropdown. The
native popover keeps the document as first responder, so Up/Down, Home/End,
Enter/Tab, and Escape navigate it without stealing typing. Terminal drag,
double-click word, triple-click line, and multi-line selections publish
selection deltas alongside Unicode-safe text deltas, keeping the hosted view
on the same authoritative range.

## Standalone TUI usage

The base components need no socket, protocol, account, environment variables,
or bridge plumbing:

```toml
[dependencies]
unpeel-app-kit = { path = "../unpeel-app-kit", default-features = false }
```

Enable the Ratatui Markdown editor independently when needed:

```toml
[dependencies.unpeel-app-kit]
path = "../unpeel-app-kit"
default-features = false
features = ["markdown-text-area"]
```

Static terminal images are independently opt-in as well:

```toml
[dependencies.unpeel-app-kit]
path = "../unpeel-app-kit"
default-features = false
features = ["media"]
```

`MediaPicker::from_query_stdio()` selects Kitty first when supported, then
iTerm2 or Sixel, and finally Unicode half-blocks. Call it after entering the
alternate screen and before starting terminal event reads. `Media::load`
renders local paths or bounded inline images; broker-resolved blobs use
`Media::from_resolved_bytes` so length and SHA-256 are checked before decode.

Dynamic GPU surfaces are a separate, default-off integration and therefore do
not add wgpu to normal or pure-TUI builds:

```toml
[dependencies.unpeel-app-kit]
path = "../unpeel-app-kit"
default-features = false
features = ["surface-embed"]
```

The planet example reuses the existing unpeel-surface guest and presenter:

```sh
cargo build --release --manifest-path ../unpeel-surface/Cargo.toml \
  -p surface-planets-example --target wasm32-unknown-unknown
cargo run --example surface_planets --no-default-features --features surface-embed
cargo run --example surface_canvas --no-default-features --features surface-embed
```

Use arrows or Space to move between planets, Home for the overview, and Escape
to quit in the bare Surface example. In the Canvas example, Tab or Left/Right
focuses the top Buttons, Enter/Space activates one, and every control is also
clickable. `UNPEEL_SURFACE_PLANETS_WASM` or `--guest PATH` can point at a guest
outside the conventional sibling checkout.

The `ui-bridge` feature is default-on for API compatibility. It only makes the
optional hosted types available; it opens no socket and starts no background
work unless the App explicitly calls `UiBridge::detect()`. A pure-TUI build can
disable default features as above, which removes the socket, authentication,
persistence-envelope, and `unpeel.ui/1` protocol modules from compilation.

The standalone component layer currently provides:

| Component | Responsibility |
| --- | --- |
| `Explorer` | Flat current-directory navigation, filename filtering, selection, scrolling, hit-testing, and path drag sources |
| `InputField` | Borderless single-line editing with a native cursor, keyboard/mouse selection, word movement, and horizontal scrolling |
| `Page` | Top-level standalone Ratatui presentation with constrained Input header/List body slots and one optional back action |
| `List` / `ListItem` | Borderless single-line rows built from `SelectableRow`/`VerticalScrollbar`, with stable selection, status/badge/busy presentation, collapsible trailing values, and named closed slots |
| `ListState` / `ListKeymap` | Clamped non-wrapping selection, scroll-to-reveal/paging, hit testing, and the shared arrow/j/k/Home/g/End/G/Page/Enter/Escape/q vocabulary |
| `SelectableRow` | Full-width gray selected/hovered row painter returning the standard two-cell-inset content rectangle |
| `Toggle` / `Input` | Owned component specifications used directly by the TUI and optionally serialized for native renderers |
| `Button` | Closed semantic action control with default/primary/destructive native intent rather than arbitrary styling |
| `CanvasPage` | Exactly one Surface slot plus a bounded fixed top Button toolbar, with Ratatui layout/hit boxes and no generic child tree |
| `PopupMenu` / `MenuItem` | Gray borderless context menu/dropdown with hover, keyboard selection, disabled items, and danger tones |
| `KitTheme` / `ThemeMonitor` | Shared dark/light defaults plus a live hosted project/workspace accent for selectable rows, menus, text, and scrollbars |
| `DoubleClickTracker` | Target-aware double-click detection shared by mouse-driven Apps |
| `KeyboardEnhancementGuard` | Scoped unambiguous Escape delivery on capable terminals |
| `AgentBridge` | Async adjacent-agent discovery plus approval-free same-group path/text handoff |
| `EditorBridge` | Open a file or folder with Unpeel's preferred editor, with a standalone platform fallback |
| `DragSurface` | Frame-accurate semantic path regions for native drag-and-drop |
| `DropTargetSurface` | Native file/folder hover and drop cells for caret previews and App-owned insertion |
| `DraggablePath` / `DragSource<W>` | Small Ratatui wrappers for custom path drag sources |
| `AppContext` | Standalone-safe detection of the current hosted workspace, base project, worktree, and opaque Unpeel user |
| `AppReporter` | Shared hosted status, activity, alerts, automatic title, and agent-readable App context |
| `Navigator<Route>` | Rendering-neutral root/detail view stack with consistent back semantics |
| `display_path_from_root` | Render paths project-relative without weakening absolute semantic path operations |
| `VerticalScrollbar` | Shared proportional, capless scrollbar |
| `MarkdownEditor` / `MarkdownTextArea` | Ratatui-backed Markdown editor behind the independent `markdown-text-area` feature |
| `MarkdownEditorInteraction` | Optional closed `/` block menu, Markdown-aware Enter/Backspace, and drag/word/line selection for `MarkdownTextArea` |
| `Media` | Static images behind the independent `media` feature, using Kitty/iTerm2/Sixel with a Unicode half-block fallback |
| `Surface` / `SurfaceView` | Optional `surface-embed` delegation to unpeel-surface's WASM guest, local wgpu renderer, mmap ring, and Kitty presenter; absent from default/pure-TUI builds |

The crate owns reusable component behavior and the standard flat-list keymap,
not an App's event loop, commands, or surrounding chrome. Hosted helpers elsewhere in the table are
optional calls and retain their documented standalone no-op, unavailable, or
platform-fallback behavior.

## Optional hosted UI bridge

With `ui-bridge` enabled, an App may additionally publish the same component
state to native SwiftUI/AppKit or web renderers when Unpeel hosts it. This is an
enhancement over the complete TUI, similar to an Ionic-style component
vocabulary with platform-specific renderers—not a second required runtime.

| Optional API | Responsibility |
| --- | --- |
| `UiBridge` / `unpeel.ui/1` | App-owned Unix endpoint for scoped human/agent participants, snapshots, deltas, actions, presence, and acknowledgements without touching the PTY |
| `UiStateStore` | Atomic `ui-state.json` save/restore envelope for always-on hosted Apps |
| Markdown bridge adapter | Adds `ui_node` and `handle_ui_event` to the Ratatui editor when `markdown-text-area` and `ui-bridge` are both enabled |
| Media semantic projection | Reference-only image state, cross-renderer sizing, accessibility text, and one optional activation action |
| Page semantic projection | Closed Page/List/ListItem/Toggle/Input trees, constrained master/detail activation/back actions, compact deltas, and native SwiftUI/DOM wrappers |
| Explorer semantic projection (planned) | A separate closed Explorer/Tree contract preserving hierarchy, filter focus, wrap/page navigation, and the synthetic parent action; it is not encoded as flat ListItems |
| Surface semantic projection | Opaque session/stream reference, sizing, background, and input policy only; Swift/web wrappers inject existing USRF local-GPU presenters and never consume frames |
| CanvasPage semantic projection | Closed Surface slot plus Button actions; scene/input stays on USRF while toolbar interaction stays on `unpeel.ui` |

The hosted vocabulary is deliberately small and opinionated rather than a
portable encoding of every possible Ratatui widget:

```text
Controller / browser / neighboring agent
         │ unpeel.workspace.ui/1 inside existing /mobile
         │ Direct · SSH · Link relay
         ▼
existing native or unpeel-serve Host
         │ scoped local unpeel.ui/1
         ▼
terminal-backed Rust App + UiBridge
├─ model + reducer + ui-state.json
├─ App Kit/Ratatui → the normal PTY
└─ semantic snapshots/deltas/actions → native or DOM UI
```

The Rust App remains authoritative for state, validation, persistence, and
commands. Native and web wrappers render component snapshots and return
stable actions; they never scrape ANSI output. Raw Ratatui remains the escape
hatch for App-specific terminal UI and simply has no native/web projection
until an App Kit component exists for it.

When a hosted App explicitly calls `UiBridge::detect()`, it binds
`~/.unpeel/app-sessions/<id>/ui.sock`; the existing Unpeel Host brokers it
through the already-authenticated `/mobile` transports, so there is no
standalone workspace server. Multiple human and agent participants attach
with Host-minted, route-bound scoped tokens. When the Host hides the PTY in
favor of component UI, `UiBridge::should_render_terminal()` lets the App
suspend Ratatui drawing without suspending its model or process.

On that hosted path, `UiBridge::detect()` consumes and scrubs the inherited
socket path and per-session signing key before child processes can inherit
them. The endpoint uses mode-`0600` Unix sockets plus same-user peer
credentials where supported, requires a bounded-time authenticated attach,
negotiates a min/max protocol range, and isolates slow or flooding renderers
with per-connection and per-client quotas. Delta-capable renderers receive only
contiguous server-to-client operations; any gap automatically falls back to a
snapshot.

This channel is sanctioned by Unpeel's D16 decision and lives entirely in App
Kit. Its standalone invariant is strict: every App must remain fully
functional through its TUI, and semantic rendering is only an optional
presentation path over that fallback.

`MarkdownEditor`, static `Media`, and the Todo-driven Page component family are
the first vertical slices. Media travels as a local path, a bounded 256 KiB
inline image, or a content-addressed blob reference—never as an unbounded JSON
payload. `Tabs` and later richer components such as `DataGrid` can join the
same closed, versioned vocabulary. Containment is slot-based:
`List` accepts only `ListItem` values, and row slots accept only explicitly
enumerated Toggle, status-symbol, and Badge values rather than arbitrary child
nodes. See [the component
architecture](docs/ui-components.md), the trusted [`unpeel.ui/1`
schema](protocol/unpeel-ui-v1.schema.json), and the separate
[browser-to-workspace schema](protocol/unpeel-workspace-ui-v1.schema.json).

The renderer packages live with the component definitions so the contract
cannot drift:

- `swift/` — `UnpeelAppKitUI`, including native Page/List/Toggle/Input,
  Markdown, and asynchronous `NSImage` Media views plus a reconnecting trusted
  Unix client;
- `web/` — `@unpeel/app-kit-ui`, including native DOM Page/List controls,
  Markdown, and accessible `<img>` Media renderers plus `WorkspaceUiSession`
  for the existing Host's `/mobile` extension; and
- `protocol/` — validated, forward-compatible schemas and shared fixtures
  consumed by Rust, Swift, and web tests.

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

Inside an Unpeel-hosted Session, `KitTheme::detected()` adopts the Host's
accent. The Host resolves it from the current project's folder color first,
then the workspace App color. `ThemeMonitor` polls the live Host value at a
bounded cadence, so an already-open App repaints when either setting changes;
`UNPEEL_APP_ACCENT=#RRGGBB` remains the launch/older-Host fallback. A standalone
App ignores it unless `UNPEEL_SESSION_ID` is also set. Use
`ExplorerTheme::for_theme(palette)` to carry the detected accent into folder
and parent rows; `for_color_scheme` intentionally remains a pure light/dark
constructor.

```rust
use unpeel_app_kit::{ExplorerTheme, ThemeMonitor};

let mut monitor = ThemeMonitor::detected();
let palette = monitor.theme();
let explorer_theme = ExplorerTheme::for_theme(palette);

// In the idle branch of the event loop:
if monitor.refresh() {
    explorer.set_theme(ExplorerTheme::for_theme(monitor.theme()));
}
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
cargo add unpeel-app-kit --path ../unpeel-app-kit --no-default-features
```

The resulting dependency is:

```toml
[dependencies]
unpeel-app-kit = { path = "../unpeel-app-kit", default-features = false }
```

It targets Ratatui `0.30`. Enable the Markdown text area only in Apps that
need it:

```toml
unpeel-app-kit = { path = "../unpeel-app-kit", default-features = false, features = ["markdown-text-area"] }
```

Or enable static terminal images independently:

```toml
unpeel-app-kit = { path = "../unpeel-app-kit", default-features = false, features = ["media"] }
```

An App opting into D16 hosted presentation can add `ui-bridge`, or use the
crate's default feature set.

## Native path dragging

Path dragging does not capture terminal mouse input. Instead, `DragSurface`
publishes a short-lived semantic map between Ratatui terminal rectangles and
Host-local files or directories. Unpeel performs the native point-to-cell hit
test, starts a normal platform file drag, and pastes a shell-quoted path when
the destination is another Unpeel terminal. Pasted text is relative to that
Session's project root when possible, uses `~/…` elsewhere under the user's
home, and stays absolute only outside both roots. Image paths are the exception:
they remain absolute so Claude, Codex, and other agents recognize them as local
attachments such as `[Image #1]`.

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

The current hosted vocabulary does not flatten Explorer rows into semantic
`ListItem`s. The planned Explorer/Tree projection keeps directory hierarchy,
the filter/tree focus loop, single-step selection wrapping, and the synthetic
parent entry as distinct schema semantics. Its first implementation step is an
internal, parity-tested rebuild of Ratatui Explorer rows and navigation on
`SelectableRow` and the shared navigation engine; outward page/wrap and filter
behavior will not change. Until the complete Rust + Swift + web slice lands,
Filetree and Markdown's picker remain on this fully functional Ratatui Explorer
and hosted renderers use the pane's terminal fallback. See
[the Explorer/Tree follow-up contract](docs/ui-components.md#explorertree-follow-up-contract).

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
the UI thread and caches an honest target label. `send_text()`,
`send_reference()`, and `send_path()` re-resolve at activation time and prefer
an adjacent same-group agent. `send_reference()` types an exact, unsubmitted
`path:line-range` token without a conversational sender envelope; the other
methods paste with `submit: false`, allowing the user to finish the prompt.
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

`AgentBridge::project_context()` exposes the asynchronously cached Host-owned
`cwd` and project id of that same target. Project-scoped Apps can call
`Explorer::set_navigation_root()` (or reset their own repository model) when
the agent moves into a worktree and when it moves back. An explicit CLI path
should normally disable that follow behavior.

## Current Unpeel context

`AppContext::detect()` is the shared way to discover the App's own execution
scope. It never parses `~/.unpeel` files: a hosted App asks its Host for typed
workspace, project, worktree, and Session-owner data; a normal terminal run is
simply `AppMode::Standalone`. The current project is always the logical base
project, while `current_root()` selects the active worktree path when one is
present.

```rust
use unpeel_app_kit::{AppContext, AppMode};

let mut context = AppContext::detect();
match context.mode() {
    AppMode::Standalone => { /* ordinary CLI behavior */ }
    AppMode::Hosted => {
        if let Some(root) = context.current_root() {
            // Scope project files to `root`.
        }
        let workspace = context.current_workspace().map(|value| value.name.as_str());
        let user = context.current_user().map(|value| value.id.as_str());
        let _ = (workspace, user);
    }
}

// Re-read live Host data after a workspace/context change.
if context.refresh() {
    // Rebind App state and redraw.
}
```

`current_user().id` is deliberately an opaque Host-scoped principal. It is
safe for attribution keys, but Apps must not treat it as an email, account id,
or display name. Account claims belong to the future consented identity API.
When a valid hosted Session exists but an older/unreachable Host cannot answer,
the mode remains `Hosted`, `host_available()` is false, and typed values may be
absent instead of silently pretending the App is standalone.

Use the narrowest value that matches the job:

| Need | API | Meaning |
| --- | --- | --- |
| Read files for this Session | `current_root()` | Active worktree when present, otherwise the base project |
| Group data across worktrees | `current_project()` | Logical base project and stable project id |
| Label the active checkout | `current_worktree()` | Worktree path and optional branch |
| Namespace workspace-local state | `current_workspace()` | Stable workspace id when registered, plus its display name |
| Attribute shared state | `current_user()` | Opaque Host principal only; never an email or display name |

An explicit CLI path should win over detected context. A standalone-first App
should then fall back to its normal CLI behavior—usually `current_dir()`—when
`current_root()` is absent. Refresh at a deliberate boundary such as an App
reload or background polling interval; do not issue a Host request every frame.

### Adoption in the official Ratatui Apps

Every official App that consumes App Kit constructs `AppContext`; each uses
only the fields relevant to its own behavior:

| App | Hosted behavior | Standalone fallback |
| --- | --- | --- |
| Filetree | Starts at `current_root()` and then follows the adjacent agent across worktrees | Process working directory |
| Diffs | Discovers Git from `current_root()` and then follows the adjacent agent across worktrees | Process working directory |
| Markdown | Uses `current_root()/docs` as the first-run notes-folder suggestion; explicit and remembered vaults still win | Working-directory `docs` folder |
| Usage | Resolves **Current project** from refreshed `current_root()`; worktree history is folded into the base repository | Process working directory |
| GitHub Issues | Discovers the repository and branch from `current_root()` | Process working directory |

Workspace and user values are intentionally not copied into every App's own
reporter payload: the Host already owns that scope, and an App should consume
those values only for a real workspace-local or attribution feature.

## Hosted App reporter

`AppReporter` is the one Rust implementation of Unpeel's documented file +
loopback-HTTP App contract. It is inert standalone and deduplicates/debounces
rapid context and status updates when hosted.

```rust
use unpeel_app_kit::AppReporter;

let mut unpeel = AppReporter::detect("com.example.notes");
unpeel.idle();
unpeel.set_status("editing notes.md");
unpeel.set_title("notes.md");
unpeel.set_context(&serde_json::json!({
    "file": "/notes/notes.md",
    "cursor_line": 12,
}));
unpeel.flush();
```

Use `busy()`, `idle()`, and `attention()` for lifecycle; `alert()` is an
informational Recent/notification item and does not imply attention. Context
is App-authored data surfaced to adjacent agents through Unpeel MCP—it is not
an instruction or permission grant.

## View navigation

`Navigator<Route>` is deliberately smaller than a UI framework: route values
hold App state, and a normal `match navigator.current()` renders them. Enter
pushes a detail route; Escape calls `back()`. `back() == false` means the root
is already visible, allowing the App to keep Escape non-destructive or apply
its own exit policy.

```rust
use unpeel_app_kit::Navigator;

enum Route { List, Detail(u64) }
let mut views = Navigator::new(Route::List);
views.push(Route::Detail(42));
assert!(views.back());
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
auto-scroll, and the shared proportional scrollbar. `MarkdownEditorInteraction`
adds the opinionated editing layer used by the example: character/word/line
selection, list continuation, marker-aware Backspace, and the closed `/`
insert menu (`Heading 1`–`6`, Text, Bulleted/Numbered list, To-do, Quote, Code,
and Divider). Its terminal dropdown is caret-anchored, compact, bordered, and
keyboard/pointer navigable. `markdown_delta_operations` synchronizes the
resulting Unicode-safe text edit and oriented selection without replacing the
whole document. App-specific commands and syntax highlighting remain App-owned
and can use `text_area_mut()` (or normal `DerefMut` coercion) to reach the
underlying editor.

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

## Native drop destinations

`DropTargetSurface` is the destination counterpart to `DragSurface`. Register
the freshly rendered editor rectangle, commit it after a successful frame,
and poll from the event loop. Unpeel sends repeated hover cells (including at
a stationary edge for auto-scroll), leave, and a final drop containing both
raw references and the Host's normally quoted/shortened insertion text.

```rust
use unpeel_app_kit::{DropTargetEvent, DropTargetSurface};

let mut drops = DropTargetSurface::detect();
drops.begin_frame();
terminal.draw(|frame| {
    editor.render(frame, editor_area, true);
    drops.register(editor_area);
})?;
drops.commit()?;

match drops.poll()? {
    Some(DropTargetEvent::Hover { position }) => {
        editor.position_drop_cursor(position);
    }
    Some(DropTargetEvent::Drop { position, text, .. }) => {
        editor.position_drop_cursor(position);
        editor.insert_str(text);
    }
    _ => {}
}
# Ok::<(), std::io::Error>(())
```

## Development

```sh
cargo test --no-default-features
cargo test --no-default-features --features markdown-text-area
cargo test --no-default-features --features media
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
(cd swift && swift test)
(cd web && bun install --frozen-lockfile && bun run check && bun test)
```

The Explorer and drag tests render into Ratatui's `Buffer`/`TestBackend` and
assert terminal-cell regions directly; they do not require Unpeel to be
running.
