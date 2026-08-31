# App Kit UI components

App Kit is an opinionated component library for terminal-powered Apps. Its
author-facing model is component-first—`MarkdownEditor`, then `Page`, `List`,
`ListItem`, `Input`, and other deliberate primitives—not a serialization of
terminal cells and not a portable clone of every Ratatui widget.

The Rust terminal process is the App. It owns the model, reducer, validation,
persistence, and commands. Ratatui, SwiftUI/AppKit, and DOM are renderers of
the same semantic component state.

## Runtime architecture

```text
SwiftUI/AppKit Host ─────────────────────────────┐
                                                │ trusted local unpeel.ui/1
remote browser ── authenticated WSS ─▶ workspace broker
                                                │ one Unix connection per renderer
                                                ▼
                                      terminal App process
                                      ├─ model + reducer
                                      ├─ UiBridge (App-owned socket)
                                      └─ App Kit/Ratatui ─▶ PTY
```

The session owner gives the App an absolute `UNPEEL_UI_SOCKET` path and a
random `UNPEEL_UI_TOKEN`. `UiBridge::detect` binds that Unix socket; it is
inert when the variables are absent, so the same executable remains a complete
standalone TUI. A trusted native Host or workspace broker connects and
authenticates with the token. JSON never shares stdin or stdout with ANSI
terminal bytes.

`UiBridge::detect` must run during single-threaded App startup, before the App
spawns workers or children. It reads and removes both variables from the
process environment before returning—even if endpoint detection fails—so an
App-spawned command cannot inherit the bearer token or socket route. The socket
is mode `0600`, and App Kit also compares the connecting process's effective
user ID where the operating system exposes Unix peer credentials. Platforms
without a supported peer-credential API retain the filesystem permission
check. The token still represents the trusted broker and can attest arbitrary
participant identities and grants, so it must remain server-only session
metadata.

Connections must authenticate with `attach` within five seconds. A slow
renderer has an independent bounded output queue, and each stable client has a
bounded event/ack replay ledger. Exceeding either quota disconnects only that
renderer; the App receives `Detached`, while `publish()` and `poll()` continue
serving the other clients. Final acknowledgement records expire after their
replay window, and one client's quota rotation never evicts another client's
records.

This direction is important: the replaceable GUI and workspace server attach
to the long-running terminal App. Restarting a Swift view, browser tab, or
broker does not move ownership of the App state into that renderer.

### Hiding the terminal without stopping the App

The component UI does not need to be painted over a live terminal pane. The
Host can hide or replace the PTY view while keeping its terminal process alive.
Each renderer reports whether the component UI and terminal are visible.
`UiBridge::should_render_terminal()` becomes false when visible component
renderers no longer need the terminal, allowing the App to stop calling
`Terminal::draw` and avoid unnecessary Ratatui/PTY work.

The App must continue polling `UiBridge` and its normal I/O/timers. If a native
renderer disconnects, or any attached view needs the terminal again,
`should_render_terminal()` returns true and Ratatui drawing resumes. The PTY is
therefore a durable process container and fallback renderer, not a constantly
repainted hidden canvas.

## Durable sessions and restarts

The App creates one `appInstanceId` for its process lifetime. A renderer keeps
stable `clientId`, `rendererId`, `viewId`, and client-generated `eventId`
values across reconnects:

1. The renderer attaches with its expected App instance and last revision.
2. The App answers with `attached`, then the current full snapshot.
3. Unacknowledged events can be resent with the same event IDs.
4. The App deduplicates `(clientId, eventId)` and replays the final ack.
5. If the App instance changed, clients discard pending events and rebuild
   from the new snapshot.

This survives wrapper and workspace-server restarts as long as the terminal
App process remains alive. If that process itself exits, in-memory state is
gone; an App that must survive terminal-process restarts still needs normal
disk or database persistence.

For broker restart recovery, the session supervisor must retain the socket
path and random token in server-only session metadata for the lifetime of the
terminal process. Neither value belongs in workspace HTML, browser storage, or
browser WebSocket messages.

### Protocol compatibility

The first `attach` frame carries inclusive `minProtocolVersion` and
`maxProtocolVersion` values rather than claiming one exact version. The App
selects the highest shared version. `attached` returns that selected
`protocolVersion` together with the App's own min/max range, and every later
frame on that connection must use the selected version. A connection with no
overlap is rejected without affecting other renderers.

All Rust, Swift, Web, and JSON Schema implementations follow the same forward
compatibility rule: ignore unknown fields inside a recognized message,
component, or typed value. Unknown discriminators—message type, component
type, event kind, or value type—remain errors because old code cannot safely
guess their behavior.

## Multi-user workspaces

`UiBridge` accepts many simultaneous attachments. Every attachment contains:

- an opaque, broker-attested participant ID and grants;
- a stable client ID for one user/device;
- a replaceable renderer ID and renderer capabilities;
- a logical view ID; and
- current component/terminal visibility.

`publish(view, revision, root)` sends shared state to everyone in a view.
`publish_to(client, view, revision, root)` can overlay participant-specific
state such as focus or selection without leaking it into another client's
projection. Presence lists the connected participants and renderer states.

The App enforces grants again at the Unix boundary: `view`, `interact`, `edit`,
`command`, `admin`, or `*`. This is defense in depth; workspace membership and
App-session access must also be checked by the workspace server.

### Browser trust boundary

Browser code uses `unpeel.workspace.ui/1`, not the trusted local attach frame.
Its messages deliberately contain no `authToken`, participant identity, or
grant list. The workspace server must:

1. authenticate the HTTPS/WebSocket session, validate its `Origin`, and
   authorize `appSessionId`;
2. derive the participant and grants from server-side workspace membership;
3. validate and namespace client/renderer IDs to that authenticated account;
4. open one authenticated local `unpeel.ui/1` attachment per renderer;
5. stamp that participant ID onto translated App events;
6. apply frame, rate, and connection limits; and
7. forward only `attached`, `snapshot`, `ack`, filtered `presence`, and `error`
   frames back to the browser.

The broker must strip grants and private identity fields from browser presence;
`WorkspaceUiSession` also removes grants defensively before its presence
callback. The broker must never forward `UNPEEL_UI_TOKEN`, accept a
browser-supplied participant claim, or treat a browser `clientId` as globally
trusted.

The browser transport is implemented by `WorkspaceUiSession` in `web/`. It
reconnects with bounded backoff, resumes a known App instance, retains stable
event IDs until final acknowledgement, requests resync after stale events, and
clears optimistic/pending state when a new terminal App instance appears.

## Component contract

Every cross-renderer component has:

1. an owned, serializable Rust specification;
2. stable node and action identifiers;
3. a Ratatui-backed terminal implementation;
4. a Swift renderer with native controls and accessibility;
5. a web renderer with native DOM behavior and accessibility; and
6. validation plus one shared fixture corpus across implementations.

Renderers preserve semantics and interaction, not terminal-cell geometry.
Platform-native presentation is expected: for example, `List` can map to
`ratatui::widgets::List`, SwiftUI `List`, and an accessible web list while
retaining the same item identities and selection actions.

Raw Ratatui widgets remain supported in the terminal. They do not acquire
native meaning automatically, because a painted cell buffer cannot recover
labels, selection, validation, accessibility, or intent.

## MarkdownEditor v1

The first component reuses `tui-textarea-2` rather than introducing another
text engine. Its complete snapshot contains the Markdown document, selection,
presentation (`source`, `preview`, or `split`), dirty/read-only state, title,
placeholder, and declared actions.

Renderer-to-App actions are:

- `replace-range` with a half-open `TextEdit`;
- `set-selection` with oriented anchor/head positions;
- `save`, `undo`, and `redo` commands; and
- `set-presentation` with `source`, `preview`, or `split`.

Positions use zero-based lines and UTF-16 columns. Cocoa and JavaScript use
UTF-16 natively; the Ratatui adapter validates scalar boundaries and converts
them to `tui-textarea`'s character-wise cursor. The shared tests include emoji
to ensure edits never split a surrogate pair.

The native renderer uses `NSTextView` inside `NSViewRepresentable`, with
SwiftUI chrome and preview. The web renderer uses a native `<textarea>`, emits
minimal Unicode-safe range edits, and accepts an optional Markdown rendering
callback for preview HTML.

### Wiring the first vertical slice

On the Rust side, build the projection from the same `MarkdownEditor` that
Ratatui renders. Drain `UiBridge` during the App's bounded idle tick, pass
`Action` events to `handle_ui_event`, acknowledge the outcome, increment the
App revision for model changes, and publish the next `ui_node`. The reducer
also receives `Attached`, `Detached`, and `Lifecycle` events when it needs to
maintain participant-specific state.

The web side connects the renderer and session directly:

```ts
let renderer: MarkdownEditorRenderer;
const session = new WorkspaceUiSession({
  url: workspaceUiUrl,
  appSessionId,
  clientId,
  rendererId,
  viewId: "main",
  onSnapshot: (snapshot) => renderer.render(snapshot),
});
renderer = new MarkdownEditorRenderer(container, (action) => session.send(action));
session.start();
```

The trusted native Host uses the same split: `MarkdownEditorView` emits a
`UIAction`, and `UIUnixSessionClient.send` adds session identity and revision.
Its message callback should move snapshots onto the main actor before updating
SwiftUI state. A remote Swift client should use the authenticated workspace
transport rather than receiving the local Unix token.

## Revision and collaboration semantics

An event names the snapshot revision on which the interaction occurred. The
App accepts current-revision events, deduplicates their stable event IDs, and
publishes a new immutable revision after applying model changes. Stale events
receive a `stale` acknowledgement plus a fresh snapshot; future events are
rejected.

This gives v1 deterministic reconnect and safe serialized editing. It does not
silently pretend that two concurrent text edits commute. A truly simultaneous
Google-Docs-style Markdown editor should add rebasing, OT, or a CRDT inside the
authoritative Rust model, then project each participant's cursor separately.
The transport identities and targeted projections are ready for that later
component version.

## Protocol artifacts

- `protocol/unpeel-ui-v1.schema.json` — trusted broker-to-App wire contract;
- `protocol/unpeel-ui-v1.ndjson` — shared Rust, Swift, and web fixtures;
- `protocol/unpeel-workspace-ui-v1.schema.json` — untrusted browser-to-server
  messages; and
- `protocol/unpeel-workspace-ui-v1.ndjson` — browser boundary fixtures.

## Next components

The next useful vocabulary is intentionally conventional:

| Component | Ratatui foundation | Native/web meaning |
| --- | --- | --- |
| `Page` | layout + block | top-level content and safe-area/chrome contract |
| `List` | `ratatui::widgets::List` | native list with selection and reorder |
| `ListItem` | Ratatui `ListItem` | keyed row, label, detail, icon, actions |
| `Input` | existing `InputField` | native single-line input and validation |
| `Menu` | existing `PopupMenu` | native menu with disabled/danger roles |
| `Explorer` | existing `Explorer` | hierarchical file navigation and drops |

Each should be added only with all three renderer interpretations and shared
fixtures. That keeps App Kit opinionated and prevents its public API from
becoming an unbounded remote widget toolkit.
