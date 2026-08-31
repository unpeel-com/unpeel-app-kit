# App Kit UI components

App Kit is an opinionated component library for terminal-powered Apps. Its
author-facing model is component-first—`MarkdownEditor`, then `Page`, `List`,
`ListItem`, `Input`, and other deliberate primitives—not a serialization of
terminal cells and not a portable clone of every Ratatui widget.

The Rust terminal process is the App. It owns the model, reducer, validation,
persistence, and commands. Ratatui, SwiftUI/AppKit, and DOM are renderers of
the same semantic component state.

This channel is the decided presentation path in Unpeel's decision log,
`docs/MASTER PLAN.md` §10 **D16 — The App Kit semantic component channel is a
sanctioned opt-in presentation path** (2026-08-31). D16 amends D14 and narrows
the 2026-08-24 removal: the general semantic widget SDK remains absent from
Unpeel core, while this opinionated component channel lives entirely in
`unpeel-app-kit`.

Unpeel core has only two semantic-channel touchpoints: inject
`UNPEEL_UI_SOCKET` / `UNPEEL_UI_TOKEN` when it spawns an App Session, and broker
authorized participant attachments through the existing Host contract. The
component vocabulary, protocol model, reducers, persistence convention,
Ratatui implementations, and Swift/web wrappers remain in App Kit.

The matching **Presentation paths (2026-08-31)** Product Philosophy note in
Unpeel's `AGENTS.md` makes D14 and D16 the two sanctioned additive channels.
D16's invariants are load-bearing conformance requirements:

- **Terminal fallback is mandatory.** Every App is first a complete Ratatui
  TUI; semantic presentation cannot be required to launch, use, stream, or
  recover it.
- **The bridge is inert without Host injection.** With no injected socket, the
  same binary runs as a plain TUI and never creates a competing server.
- **Component UI is not IDE chrome.** App Kit must not introduce diff viewers,
  file trees, source-code editor panes, language tooling, or other code-centric
  framing into Unpeel.
- **D14 and D16 have distinct jobs.** D14 scenes serve GPU/canvas presentation;
  D16 components serve data/document Apps where people and agents need
  semantic operations.

Unsupported components and older Hosts keep using the PTY. These boundaries
are architectural invariants, not graceful-degradation suggestions.

## Runtime architecture

```text
paired Controller / browser / remote Mac / agent Session
                         │
                         │ unpeel.workspace.ui/1 inside existing /mobile
                         │ Direct · SSH · Link relay
                         ▼
existing Unpeel Host (native app or unpeel-serve driver)
├─ ControllerPrincipal / paired-device authentication
├─ authorization + scoped participant-token minting
├─ one local unpeel.ui/1 connection per participant renderer
└──────────────────────────────────────────────────────────┐
                                                           ▼
                                            terminal App process
                                            ├─ model + reducer
                                            ├─ UiBridge → ui.sock
                                            └─ App Kit/Ratatui → PTY
```

There is no standalone workspace server in this design. The broker is the
existing Unpeel Host: today that is the native app Host or the
`unpeel-serve`-backed headless driver. Remote-anywhere UI reuses the Host's
paired-device/`ControllerPrincipal` authentication and the same Direct, SSH,
and opaque Link-relay transports already carrying `/mobile`.

The Host injects the endpoint at its existing Session-spawn choke point next
to `UNPEEL_SESSION_ID` and `UNPEEL_SESSION_DIR`:

```text
UNPEEL_UI_SOCKET=~/.unpeel/app-sessions/<session-id>/ui.sock
UNPEEL_UI_TOKEN=<random per-App-session signing key, at least 32 bytes>
```

`UiBridge::detect` binds that Unix socket; it is inert when the variables are
absent, so the same executable remains a complete standalone TUI. JSON never
shares stdin or stdout with ANSI terminal bytes.

`UiBridge::detect` must run during single-threaded App startup, before the App
spawns workers or children. It reads and removes both variables from the
process environment before returning—even if endpoint detection fails—so an
App-spawned command cannot inherit the signing key or socket route. The socket
is mode `0600`, and App Kit also compares the connecting process's effective
user ID where the operating system exposes Unix peer credentials. Platforms
without a supported peer-credential API retain the filesystem permission
check.

`UNPEEL_UI_TOKEN` is never an attachment bearer credential. It is the HMAC key
shared only by the Host and authoritative App process. The Host derives a
short-lived `upui1` participant token whose signed claims bind:

- App Session id;
- participant id, `human` / `agent` / `service` kind, presentation metadata,
  and exact grants;
- client id, renderer id, and view id;
- issued/expiry timestamps and a token id.

The `attach` frame contains only that derived `participantToken`; it cannot
claim a different participant or add `admin`. A neighboring agent Session may
therefore attach to the same socket as a first-class participant with, for
example, `view + edit`, its `sourceSessionId`, and no command/admin grant. The
Host mints it after resolving the calling `UNPEEL_SESSION_ID`; it never hands
the agent the per-App-session signing key.

The Session supervisor retains that signing key in Host-private runtime state
for as long as the App process is alive, so a restarted native or headless
transport adapter can mint a fresh short-lived attachment token. A full Host
or machine reboot relaunches the App with a newly generated key; the durable
App model comes from `ui-state.json`, never from the credential.

Connections must authenticate with `attach` within five seconds. A slow
renderer has an independent bounded output queue, and each stable client has a
bounded event/ack replay ledger. Exceeding either quota disconnects only that
renderer; the App receives `Detached`, while `publish()` and `poll()` continue
serving the other clients. Final acknowledgement records expire after their
replay window, and one client's quota rotation never evicts another client's
records.

### Existing Host integration points

The two D16 Unpeel-core touchpoints map to a narrow additive slice at existing
choke points; native and remote transport adapters stay wrappers around that
same Host contract:

1. `crates/unpeel-core/src/session_host.rs` constructs the provider
   `CommandBuilder`; its shared launch integration already injects
   `UNPEEL_SESSION_ID` / `UNPEEL_SESSION_DIR`. Create `<session>/ui.sock`, mint
   the per-App-session key, and inject the two UI variables there only for an
   App-capable Session.
2. `crates/unpeel-core/src/controller_api.rs` remains the transport-neutral
   semantic router. UI attach/resume/action/lifecycle/resync operations enter
   after an adapter has supplied `ControllerPrincipal`; authorization and
   token scopes are decided there, not in Swift, HTTP, or Relay code.
3. Native `MobileRemoteServer.swift` and the `unpeel-serve` mobile driver
   expose the same additive `/mobile` capability and translate
   `unpeel.workspace.ui/1` ↔ local `unpeel.ui/1`. They reuse paired-device
   credentials, workspace selection, request replay, and existing rate limits.
4. Direct, SSH, and Link remain adapters around that one Host operation. Link
   relays opaque encrypted frames and never stores App UI or state.

The local socket is not advertised as a remote address. A remote Controller
names the ordinary Session id; the authenticated Host resolves the private
socket and opens it on the Controller's behalf.

This direction is important: the replaceable GUI and existing Host adapter
attach to the long-running terminal App. Restarting a Swift view, browser tab,
or transport adapter does not move ownership of the App state into a renderer.

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
2. The App answers with `attached`, then the current full snapshot (the safe
   reconnect baseline).
3. Unacknowledged events can be resent with the same event IDs.
4. The App deduplicates `(clientId, eventId)` and replays the final ack.
5. If the App instance changed, clients discard pending events and rebuild
   from the new snapshot.

This survives renderer and Host-adapter restarts while the terminal App keeps
running. Always-on Apps also use the App Kit persistence convention so the App
model survives a Host or machine reboot:

```text
~/.unpeel/app-sessions/<session-id>/
├─ manifest.json          existing Host launch record
├─ session.sock           existing PTY control socket
├─ ui.sock                ephemeral App-owned semantic endpoint
└─ ui-state.json          App-owned durable model envelope
```

`UiBridge::state_store()` exposes a `UiStateStore` for `ui-state.json`. Its
`save` operation writes a versioned `unpeel.app-kit.state` envelope to a
private temporary file, fsyncs it, atomically renames it, and fsyncs the
Session directory. The envelope records App id/version, the App-owned model
schema version, current UI revision, save time, and arbitrary App state. On
launch the App loads and migrates this envelope **before** publishing its first
snapshot. It creates a new `appInstanceId` but continues from the persisted
model revision, so renderers discard unsafe pending events and rebuild cleanly.

The complementary Host convention is small but essential: an App Session
marked always-on keeps its stable Session id and launch record, and the
existing Host supervisor relaunches that command after reboot with a fresh
signing key and the same Session directory. The Host does not interpret or
rewrite `ui-state.json`; App Kit does not invent a second process supervisor.
Removing the Session removes its state under the existing Session lifecycle.

An App should save after durable model commits (normally debounced for rapid
edits). Shared Room content continues to belong in Host RoomStore/RoomFS;
`ui-state.json` is for the always-on App process's recoverable model and local
metadata, not a competing multi-user database.

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

`UiBridge` accepts many simultaneous human, agent, and service attachments.
Every attachment contains:

- one Host-signed participant token (identity, kind, grants, and route claims);
- a stable client ID for one user/device;
- a replaceable renderer ID and renderer capabilities;
- a logical view ID; and
- current component/terminal visibility.

`publish(view, revision, root)` sends shared state to everyone in a view.
`publish_to(client, view, revision, root)` can overlay participant-specific
state such as focus or selection without leaking it into another client's
projection. Presence lists the connected participants and renderer states.

The App enforces signed grants again at the Unix boundary: `view`, `interact`, `edit`,
`command`, `admin`, or `*`. This is defense in depth; workspace membership and
App-session access are authorized first by the existing Host. Agent presence
uses the same participant list and event path, so UI can show which agent is
viewing, selecting, or editing instead of treating automation as invisible
owner activity.

### Browser trust boundary

Browser and remote Controller code use `unpeel.workspace.ui/1`, not the local
attach frame. This is an additive message family inside the existing `/mobile`
contract, not a new listener, account system, workspace daemon, or Relay data
model. Its messages deliberately contain no signing key, participant token,
participant identity, or grant list. The native or headless Host must:

1. authenticate through the existing paired-device/Controller transport,
   validate browser `Origin` where applicable, and authorize `appSessionId`;
2. derive the participant and grants from `ControllerPrincipal` (or the
   neighboring agent Session principal);
3. validate and namespace client/renderer IDs to that authenticated account;
4. mint one route-bound participant token and open one local `unpeel.ui/1`
   attachment per renderer;
5. stamp that participant ID onto translated App events;
6. apply frame, rate, and connection limits; and
7. forward only `attached`, `snapshot`, `ack`, filtered `presence`, and `error`
   frames back to the browser.

The Host adapter must strip grants and private identity fields from browser
presence. An agent's public profile can retain its opaque id, `agent` kind,
display name, and color, while its signed `sourceSessionId` remains local to
the Host and App. `WorkspaceUiSession` also removes grants and
`sourceSessionId` defensively before its presence callback. It must never
forward `UNPEEL_UI_TOKEN` or `participantToken`, accept a browser-supplied
participant claim, or treat a browser `clientId` as globally trusted.

The browser transport is implemented by `WorkspaceUiSession` in `web/`. It
connects to a Host-provided `/mobile` UI transport URL, reconnects with bounded
backoff, resumes a known App instance, retains stable event IDs until final
acknowledgement, applies contiguous deltas, requests resync after a gap/stale
event, and clears optimistic/pending state when a new App instance appears.

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
App revision for model changes, persist durable state, and call
`publish_delta` for ordinary edits (`publish` remains the snapshot/fallback
path). The reducer
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
transport rather than receiving any local participant token or signing key.

## Revision and collaboration semantics

An event names the snapshot revision on which the interaction occurred. The
App accepts current-revision events, deduplicates their stable event IDs, and
publishes a new immutable revision after applying model changes. Stale events
receive a `stale` acknowledgement plus a fresh snapshot; future events are
rejected.

Server-to-client deltas are part of the foundation, not a post-DataGrid
optimization. `UiDelta` carries `baseRevision`, the next `revision`, and
ordered component operations. Markdown currently supports range replacement,
selection, presentation, dirty/read-only/title/placeholder/action updates, and
`replaceRoot` as an escape hatch. Swift and web clients apply deltas to their
last complete snapshot and expose the resulting complete state to renderers.

A renderer advertises `serverDelta`. `UiBridge` sends operations only when its
last queued projection for that renderer is the exact base revision and came
from the correct shared/targeted projection. A missing revision, personalized
base, unsupported capability, reconnect, or application failure triggers a
full snapshot. This invariant is what lets later DataGrid operations update a
cell range or splice rows without pushing an Excel-sized snapshot over WAN.

This gives v1 deterministic reconnect and safe serialized editing. It does not
silently pretend that two concurrent text edits commute. A truly simultaneous
Google-Docs-style Markdown editor should add rebasing, OT, or a CRDT inside the
authoritative Rust model, then project each participant's cursor separately.
The transport identities and targeted projections are ready for that later
component version.

## Protocol artifacts

- `protocol/unpeel-ui-v1.schema.json` — scoped local participant/App wire;
- `protocol/unpeel-ui-v1.ndjson` — shared Rust, Swift, and web fixtures;
- `protocol/unpeel-ui-participant-token-v1.schema.json` — decoded signed claim
  contract used by native and headless Hosts;
- `protocol/unpeel-workspace-ui-v1.schema.json` — untrusted renderer messages
  inside `/mobile`;
- `protocol/unpeel-workspace-ui-v1.ndjson` — Controller boundary fixtures; and
- `protocol/unpeel-app-kit-state-v1.schema.json` — reboot persistence envelope.

## Next components

The next useful vocabulary is intentionally conventional:

| Component | Ratatui foundation | Native/web meaning |
| --- | --- | --- |
| `Page` | layout + block | top-level content and safe-area/chrome contract |
| `Tabs` / `TabItem` | `ratatui::widgets::Tabs` | native tab selection and accessibility |
| `List` | `ratatui::widgets::List` | native list with selection and reorder |
| `ListItem` | Ratatui `ListItem` | keyed row, label, detail, icon, actions |
| `Input` | existing `InputField` | native single-line input and validation |
| `Menu` | existing `PopupMenu` | native menu with disabled/danger roles |
| `Explorer` | existing `Explorer` | hierarchical file navigation and drops |
| `DataGrid` | table + virtual viewport | virtualized sheet with range/cell deltas |

Each should be added only with all three renderer interpretations and shared
fixtures. That keeps App Kit opinionated and prevents its public API from
becoming an unbounded remote widget toolkit.
