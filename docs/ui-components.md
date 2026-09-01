# App Kit components

App Kit is first an opinionated Ratatui library for normal standalone TUI
Apps. Build with its components, own the event loop and model, and run the same
binary in any terminal with no Unpeel process, account, socket, protocol, or
environment variables present. Component construction, input handling, and
Ratatui rendering require zero bridge plumbing.

The canonical proof is the standalone Todo App:

```sh
cargo run --example todo
cargo run --example todo --no-default-features
```

Both commands run the complete Ratatui UI in any terminal. The first build also
contains the optional hosted projection; without injected Host variables it is
equally inert. Todo constructs Page, List, ListItem, Toggle, and Input values
without passing a bridge into any component API.

On macOS, the independent Kitchen Sink package exercises that same binary,
the component examples, and the five sibling Apps (Usage, Diffs, GitHub
Issues, Markdown, and File Tree) through real libghostty PTYs, the native
SwiftUI renderer, and the actual TypeScript DOM renderers inside `WKWebView`,
without Unpeel installed:

```sh
swift run --package-path swift/Examples/KitchenSink
```

It is a test-only mini-host, not another workspace server. It reproduces the
two real Host touchpoints locally—spawn-time endpoint injection and scoped
participant-token minting—then exposes App restart, renderer resume, lifecycle
visibility, multi-participant grants/presence, targeted projections, acks, and
snapshot-versus-delta delivery as interactive harness controls. Its live
component-tree inspector exposes ids, constrained slots, actions, values, and
the current revision independently of the chosen presentation. The executable
is a separate SwiftPM package that depends on `UnpeelAppKitUI`; the renderer
library does not depend on the harness.

## Standalone component layer

A pure-TUI App can compile out the hosted presentation layer entirely:

```toml
[dependencies]
unpeel-app-kit = { path = "../unpeel-app-kit", default-features = false }
```

The Ratatui Markdown editor is independently selectable:

```toml
[dependencies.unpeel-app-kit]
path = "../unpeel-app-kit"
default-features = false
features = ["markdown-text-area"]
```

Static terminal images are independently selectable too:

```toml
[dependencies.unpeel-app-kit]
path = "../unpeel-app-kit"
default-features = false
features = ["media"]
```

| Cargo features | Compiled API |
| --- | --- |
| `default-features = false` | Core Ratatui components and standalone-safe helpers; no socket or UI protocol modules |
| `markdown-text-area` | Adds the Ratatui `MarkdownEditor` / `MarkdownTextArea` and optional `MarkdownEditorInteraction` controller |
| `media` | Adds static image decoding and Ratatui rendering through Kitty, iTerm2, Sixel, or Unicode half-blocks |
| `surface-embed` | Adds the optional unpeel-surface runtime plus Surface/CanvasPage Ratatui presentation; normal builds pull no wgpu |
| `ui-bridge` (default) | Adds the optional protocol, socket, scoped-token, and state-envelope APIs |
| `markdown-text-area` + `ui-bridge` | Also adds the Markdown semantic projection/event adapter |
| `media` + `ui-bridge` | Uses the same Media specification for the standalone TUI and hosted semantic projection |

Keeping `ui-bridge` default-on preserves the existing API, not a runtime
requirement. Merely compiling it starts no listener or worker and changes no
terminal behavior. An App that never calls `UiBridge::detect()` loses nothing;
an App built without the feature contains none of the bridge modules.

API ergonomics follow that boundary. Authors construct, render, and drive the
Ratatui component first. Bridge-only projections and events are additional
methods/types available under `ui-bridge`; they are never constructor
parameters or prerequisites for terminal input and rendering.

## Optional hosted presentation layer

When Unpeel hosts the App, it may opt into a component-first semantic model—
`MarkdownEditor`, static `Media`, Page, List, ListItem, Toggle, Input, Button,
CanvasPage, and other
deliberate primitives—not a serialization of terminal cells or a
portable clone of every Ratatui widget. The Rust terminal process remains the
App and owns the model, reducer, validation, persistence, and commands.
SwiftUI/AppKit and DOM become optional renderers of the same state alongside
Ratatui.

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

## Hosted runtime architecture

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
component, or typed value. Unknown message, event-kind, and value-kind
discriminators remain errors because old code cannot safely guess their
behavior.

Component kinds use a different rule now that Media is the second component.
Swift and web retain an unrecognized root long enough to switch the complete
pane to its terminal view; they do not reject or close the attachment and do
not guess at a partial native representation. The same pane-level fallback
applies when the renderer omits the required capability, negotiates an
incompatible component version, receives an unknown component discriminator,
or receives a local-path Media source in a browser. The renderer reports
`rendererVisible: false, terminalVisible: true` through the existing lifecycle
message while its authenticated attachment remains alive.

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
7. forward only `attached`, `snapshot`, `delta`, `ack`, filtered `presence`,
   and `error` frames back to the browser.

For Media, the Host must additionally prevent local `path` sources from
crossing this boundary. A browser projection receives a bounded `inline`
source or a `blob` reference. Blob bytes are fetched out of band through the
same authenticated App Session route after checking session grants, byte
length, and SHA-256; neither snapshots nor deltas contain the asset body.

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
App Kit's Ratatui `ListWidget`, SwiftUI `List`, and an accessible web list while
retaining the same item identities and selection actions.

Raw Ratatui widgets remain supported in the terminal. They do not acquire
native meaning automatically, because a painted cell buffer cannot recover
labels, selection, validation, accessibility, or intent.

### Closed composition and slots

Containment is component-specific and slot-based, never a generic flexbox or
an arbitrary `UiNode.children` tree. Each container owns a deliberately closed
schema that preserves the meaning needed by Ratatui, native renderers, the
web, and agent participants:

- `List` contains only keyed `ListItem` values, not arbitrary nodes;
- `ListItem` owns row fields such as label and detail plus enumerated slots
  such as `leading`, `trailing`, and `accessory`;
- each slot accepts only the schema-enumerated controls allowed in that role;
  v1 has completion `Toggle`, selection `Checkmark`, navigation `Disclosure`,
  static status-symbol, and `Badge` values; and
- `Page` and later containers expose named, purpose-specific regions only when
  their cross-platform semantics are defined.

The current master/detail extension stays inside those named semantics:
`ListItem.detail` is secondary copy, `ListItem.value` is a trailing read-only
value that disappears rather than crushing the label below its width contract,
`ListItem.busy` is a spinner/progress state, and `ListItem.activate` is one
declared row action; `Page.back` is one declared return action. They are not
arbitrary children. Renderers advertise `listItemMetadata`,
`listItemPresentation`, `listItemActivate`, and `pageBack` before receiving a
tree that depends on them. Interactive row roles additionally require
`listItemRole`, so an older renderer falls back instead of guessing at Enter,
Space, or navigation behavior.

Slot payloads are closed enums, not nested `UiNode` escape hatches. For
example, a trailing `Toggle` remains a native SwiftUI list-row toggle and an
accessible web checkbox while retaining its label, boolean value, and action
identity for an agent. Adding a new embeddable control therefore requires an
explicit schema and renderer change. This is the D16 boundary that prevents
App Kit from becoming an unbounded remote widget toolkit.

## Page component family v1: canonical Todo

[`examples/todo.rs`](../examples/todo.rs) is the canonical App Kit demo and the
source of the Page/List/ListItem/Toggle shared fixture. It has one model and one
reducer:

```text
Page "Todos"
├─ header: Input "New todo" → submit("add-todo", text)
└─ body: List "todos"
   └─ ListItem { id, label, done, delete }
      └─ trailing: Toggle → change("set-done", bool)
```

The standalone presentation uses App Kit's `ListWidget`, built from
`SelectableRow` and `VerticalScrollbar`, plus the existing `InputField`. It
remains fully usable with `ui-bridge` compiled out. When a
Host endpoint is present, the same owned values serialize as one Page root;
SwiftUI `PageView` maps them to a native `List`, `Toggle`, and `TextField`, and
the web `PageRenderer` maps them to a list, checkbox controls, and a labeled
text input. No renderer scrapes terminal output.

Containment is deliberately closed:

- Page's v1 header accepts Input and its body accepts List;
- List accepts only ListItem rows; and
- ListItem's `leading`, `trailing`, and `accessory` slots accept only Toggle,
  Checkmark, Disclosure, status-symbol, or Badge values, never an arbitrary
  `UiNode`; it accepts at most one primary role, and a completion Toggle's
  value and the row's `done` state are one validated invariant.

Rows deliberately separate three layers:

- **Focus** is `RowNavigationState` plus `SelectableRow`: behavior-agnostic
  selection, viewport, reveal, hit testing, and the one exact selected-row
  painter. `ListState` adds only List-specific spinner presentation around
  that engine.
- **Role** is closed `ListItem` vocabulary. A Toggle is the checkbox-style
  boolean row; Disclosure is a chevron/navigation affordance; Checkmark is a
  selection-mode row; `activate` without an accessory is a command row and
  may declare `actionRole: destructive`; no role/action is static information.
- **Navigation** is App-owned and server-driven. Activating a Disclosure sends
  the declared action to the reducer; the App publishes the next Page, and
  that Page's `back` action pops it. SwiftUI may animate that replacement, but
  no renderer becomes the router or durable owner of the navigation stack.

`ListKeymap::decision_for_key` owns the keyboard decision table once:

| Key | Focused row result |
| --- | --- |
| Enter | invoke Toggle, Checkmark, Disclosure, command, or destructive primary role; static rows do nothing |
| Space | invoke only a Toggle primary role; otherwise PageDown |
| Down/`j`, Up/`k`, Home/`g`, End/`G`, PageUp/PageDown | move focus through the shared navigation engine |
| Escape/`q` | request the current Page's App-owned `back` action |

Flat List focus clamps at both ends, never wraps, pages by the rendered
viewport minus `pageOverlap`, and reveals focus with configurable
`scrollPadding`. A List may declare page behavior `scroll` for screens such as
Usage whose Page keys move only the viewport. Ratatui, SwiftUI, and web use
the same decision table; `selectedId` and the idempotent `select` action carry
the authoritative identity over the semantic channel. The older
`action_for_key`/`spacePagesDown` surface remains wire/API-compatible for
existing Apps, but new role-aware Apps use `decision_for_key`.

The terminal renderer deliberately reproduces the established App rows by
construction: selected background across the complete row, exactly two cells
of left content padding and one cell on the right, borderless single-line
rows, and a right-edge scrollbar only when content overflows. A leading status
symbol, inline badge, spinner, and right-aligned value are schema fields rather
than arbitrary styled spans. This is the visual contract used for sibling-App
buffer parity tests.

The wire declares all nested component ids and actions, so an agent can reason
about “complete todo 2” rather than terminal coordinates. A neighboring agent
attached by the existing Host with `view + edit` can submit, toggle, or delete
through the same idempotent semantic events as a person. `UiBridge` enforces
the edit grant for `change` and `submit`; the Todo reducer validates the target,
persists the result, publishes a compact delta, and acknowledges the event.
This is the smallest end-to-end proof of the always-on-App-plus-agents model.

Todo also prototypes the portable save/restore convention. Its cwd file
`.unpeel-todo.json` (override with `UNPEEL_TODO_PATH`) contains a stable format
name, format version, App state-schema version, current semantic revision,
next id, and App-owned todos. Each durable mutation is written to a private
temporary file, synced, and atomically renamed before the new revision is
published. A relaunch loads and validates this file before its first snapshot.
Hosted production Apps may place the same model inside `UiStateStore`; the
renderer is never the durable owner in either form.

Page adds six compact operations: `toggleSetValue`, `checkmarkSetValue`,
`inputSetValue`, `listInsertItem`, `listRemoveItem`, and `listSetSelection`. A
Toggle update also updates its row's denormalized `done` value, preserving one
semantic invariant across all three renderers.

The pane-level degradation rule is explicit: if a renderer does not recognize
the Page root, any named slot/role kind, or any required
Page/List/ListItem/Toggle/Input, `listItemRole`, ListItem
metadata/activation, or Page-back capability, it keeps the attachment alive
and requests the complete terminal view for that pane. It never rejects the
attach merely because its component vocabulary is older.

The sixteenth shared NDJSON fixture exercises the same Page family as a Usage
master/detail screen: provider rows carry a leading health status, plan badge,
emphasis, detail/value metadata, narrow-width value policy, busy state,
selection, and an activation action, while the detail Page carries a back
action. Rich Ratatui meters remain App-owned, but native/web users can navigate
and refresh the same authoritative provider model without introducing a
generic dashboard or flex container.

## Explorer/Tree v1

Filetree and Markdown's note picker are not flat Lists. Their standalone TUIs
keep the existing `Explorer` outward contract unchanged, including its
current-folder navigation, filter focus, synthetic parent row, directory/file
distinction, selection wrapping, page behavior, path hit testing, and drag
registration. They must not be migrated by serializing the visible rows as
`ListItem`s: doing so would erase hierarchy and make parent navigation look
like file activation.

The complete semantic slice now ships as `Tree`: a standalone Ratatui
`TreeWidget`, `Explorer::semantic_tree`/`handle_ui_event` adapters, SwiftUI
`TreeView`, DOM `TreeRenderer`, validation, capability negotiation, shared
fixtures, and compact deltas. Markdown's vault picker and unpeel-app-filetree
publish it from their existing Explorer state. Their TUI behavior and painter
remain unchanged.

### Shared primitives and wire component

The prerequisite internal refactor is complete. Ratatui `Explorer` now paints
every selectable entry through `SelectableRow`, uses `VerticalScrollbar`, and
owns a `RowNavigationState` from the same focus engine wrapped by `ListState`.
Its filter-aware key adapter delegates the common Enter/Space/movement/back
decision to `ListKeymap`; App-specific create, quit, and menu commands stay in
the owning App.

The common navigation engine must expose an explicit boundary policy rather
than forcing Explorer into List's clamp policy. Flat Lists stay clamped;
Explorer preserves its current page/wrap contract exactly (single-row moves
wrap, while page moves retain the existing viewport and boundary behavior).
Explorer's filter-aware key adapter also keeps printable `j`, `k`, and `q` as
filter input instead of inheriting List aliases. Shared implementation means
shared mechanics, not identical public bindings in incompatible focus modes.

The refactor is guarded by a frozen legacy painter comparison that checks the
Ratatui row buffer cell for cell, plus key-sequence tests for selection,
scroll, filter focus, directory, parent-row, and activation behavior. Filetree
and Markdown's picker consume the same adapter. The semantic projection was
added only after that terminal contract was frozen.

The hosted component is one closed `Explorer`/`Tree` family, not an arbitrary
recursive `UiNode` container. Its snapshot has:

- one keyed Explorer root with a display location, optional named filter,
  selected entry id, empty-state text, and declared select/open/parent/filter
  actions, an optional constrained primary Button, plus one idempotent
  set-expanded action when outline mode is used;
- ordered `ExplorerItem` values whose closed kind is `parent`, `directory`, or
  `file`, plus label, symlink/hidden metadata, and only directory-owned
  `ExplorerItem` children;
- an explicit child state (`loaded`, `unloaded`, or `loading`) so a renderer
  never mistakes an empty directory for a lazily omitted subtree; and
- bounded node count/depth validation. Items cannot contain controls, Pages,
  Lists, or arbitrary component children.

The same model supports a drill-down presentation, matching today's Ratatui
Explorer and Markdown picker, or a native outline presentation. Ordering and
identity remain App-owned in both cases. An entry id is an opaque App key sent
back by actions; it is not an absolute Host filesystem path. The App resolves
that key inside its scoped navigation root. Local Ratatui drag registration may
continue using absolute paths, but those paths and drag URLs never enter the
remote semantic snapshot.

The synthetic parent item is a distinct semantic kind. It stays visible while
filtering, is absent at a scoped root, and invokes the root's `parent` action;
it never masquerades as a directory or emits the ordinary file `open` action.
Breadcrumbs or ancestor labels are presentation metadata, not additional
selectable rows.

Explorer keeps its existing interaction contract rather than inheriting
`ListKeymap`'s clamped behavior:

- with the tree focused, Up/Down wrap one row, Home/End select boundaries,
  PageUp/PageDown move by the rendered viewport and clamp, Right/Enter opens,
  and Left/Backspace/Escape navigates to the parent;
- Tab, `/`, Ctrl-F, or typing any unmodified printable character focuses the
  filter; the first printable character is inserted, including `j`, `k`, and
  `q` rather than interpreting them as List navigation;
- with the filter focused, Tab or Down returns to the tree, Up moves selection
  while keeping filter focus, Escape navigates to the parent, and the normal
  text-selection/editing keys remain native; and
- Up from the first tree row focuses the filter, preserving the current
  Filetree and Markdown picker focus loop.

Filter focus is ephemeral per renderer/participant and must not steal focus
from another participant. The focus loop is a renderer rule; filter value,
current location, entry projection, and selected id remain authoritative App
state (or a participant-targeted projection when the App wants private
selection). Native Swift uses a `TextField` plus `List`/`OutlineGroup`; web uses
an input plus an ARIA `tree` with roving focus and `treeitem`
`aria-expanded`/`aria-selected` state. Both implement the same keys above.
`TreeWidget` is the direct Ratatui interpretation when an App wants to render
the owned specification; existing Explorer Apps keep rendering Explorer so
their frozen path drag/hit behavior remains exact.

The delta vocabulary includes keyed selection, filter replacement, location
replacement, child splice, child loading-state, and expansion updates. Any
non-contiguous base or invalid hierarchy requests a complete snapshot.
Renderers advertise `tree`, `treeHierarchy`, `treeFilter`, `treeParent`, and
`button` only as required by the exact projection; an older renderer falls
back to the terminal for the whole pane without failing attach.

This is a bounded file/document-navigation primitive for Apps such as a file
browser or note picker, not IDE project chrome. It remains the D16 semantic
side of those Apps; editing stays in `MarkdownEditor`, while canvas scenes stay
on the D14 Surface path.

## Menu v1

`SemanticMenu` is a bounded action list, not a generic popup container. Each
item has one opaque id, label, action, optional hint, disabled state, and a
closed `default` or `danger` role. The Menu declares a `popup` or `context`
presentation and a `control`, `caret`, or `pointer` anchor hint. Anchors never
carry global coordinates: terminal, native, and web resolve them inside their
own current layout, so stale geometry cannot cross clients.

The standalone terminal path reuses the existing painter exactly:
`SemanticMenu::popup(position, theme)` adapts the specification into
`PopupMenu<String>`, including disabled-row skipping, danger tones, scrolling,
hover, and keyboard selection. Markdown retains its compact bordered
shortcut/name/sample insert layout as a specialized terminal interpretation,
but its items, selected id, actions, and reducer are the same semantic Menu
published to other renderers.

Swift maps a root Menu to `SemanticMenuView`, caret menus to an `NSPopover`
that leaves `NSTextView` first responder, and context menus to `NSMenu`. Web
maps both root and nested menus to an ARIA menu with roving selection. All
interpretations support Up/Down, Home/End, Enter/Space, Escape dismissal,
disabled items, and danger styling. A root Menu uses `menuSetSelection`;
Markdown changes both nested menu descriptors with `markdownSetMenus`, so
snapshots remain small and menu state follows the App revision.

Markdown declares an optional `openMenu` action with a closed `slash` or
`palette` value. Native and web intercept `/` or `\` only on a blank,
unselected, unfenced line and request that action; the Rust reducer rechecks
authoritative state, inserts the slash only for the slash flow, and publishes
the Menu. No renderer owns routing or smuggles a palette trigger through a
text edit. Context actions likewise return the declared item id/action to the
same reducer.

`menu` and `menuAnchor` are explicit capabilities. If either is absent, a
renderer keeps the attachment and uses the complete terminal pane rather than
rejecting the App. The shared NDJSON stream covers a root context Menu,
selection delta, and Markdown-nested insert/context menus across Rust, Swift,
and web.

## Content v1

`Content` is the deliberate detail-screen primitive for read-only issue
bodies, comments, patches, logs, and prose. It is a closed Page body slot, not
a generic child container and not a code editor. An App publishes keyed
logical lines made from styled runs. Runs use only the shared
default/muted/accent/info/success/warning/danger tones plus
regular/strong/italic emphasis; whole lines may be default, muted, header,
added, or removed. The component also declares body or monospace typography,
wrap/no-wrap intent, an accessibility label, and empty-state text.

Scrolling and platform text selection are renderer-local presentation. When
an App needs selection as durable collaborative state, it declares one
idempotent `select` action and publishes an inclusive keyed anchor/head line
range. Context actions use one bounded `SemanticMenu`; renderers return the
selected menu action together with the target line id. Filesystem paths,
arbitrary colors, source-language services, editing commands, and opaque view
children are intentionally absent.

The terminal interpretation is `ContentWidget`, using Ratatui styled lines,
wrap/horizontal scroll, diff row backgrounds, synchronized selection, and the
shared vertical scrollbar. SwiftUI renders a selectable scrolling document
with native context menus. Web renders an accessible `role=document` surface
whose styled DOM lines support browser selection, drag line ranges, and the
same menu actions. `contentSpliceLines` changes a keyed line collection by
delta and `contentSetSelection` synchronizes its range, so large details do
not fall back to snapshot-per-change.

Renderers advertise `content`, plus `contentSelection` and Menu capabilities
only when the exact projection uses them. A renderer missing any required
capability keeps the connection and shows the complete terminal pane. The
shared NDJSON fixtures cover a styled diff Page and both Content delta forms
in Rust, Swift, and web.

## MarkdownEditor v1

The first component reuses `tui-textarea-2` rather than introducing another
text engine. Its complete snapshot contains the Markdown document, selection,
presentation (`source`, `preview`, or `split`), dirty/read-only state, title,
placeholder, and declared actions.

Renderer-to-App actions are:

- `replace-range` with a half-open `TextEdit`;
- `set-selection` with oriented anchor/head positions;
- `save`, `undo`, and `redo` commands; and
- `set-presentation` with `source`, `preview`, or `split`; plus
- optional `open-menu` with the closed `slash` or `palette` trigger when the
  App publishes semantic insert commands.

Positions use zero-based lines and UTF-16 columns. Cocoa and JavaScript use
UTF-16 natively; the Ratatui adapter validates scalar boundaries and converts
them to `tui-textarea`'s character-wise cursor. The shared tests include emoji
to ensure edits never split a surrogate pair.

The native renderer uses `NSTextView` inside `NSViewRepresentable`, with
SwiftUI chrome and preview. The web renderer uses a native `<textarea>`, emits
minimal Unicode-safe range edits, and accepts an optional Markdown rendering
callback for preview HTML. Both keep keystrokes optimistic locally and
coalesce them into one edit at a time against the latest authoritative App
revision; a renderer never floods several range edits carrying the same stale
base revision. The native editor also preserves its local caret/selection
across unrelated presence, ack, and projection redraws; it reapplies an App
selection only on initial attach or a genuine external document replacement.

`MarkdownEditorInteraction` is the standalone terminal interaction layer. It
adds drag selection, double-click word selection, triple-click line selection,
Markdown-aware Enter/Backspace, and a closed `/` insert menu. The vocabulary is
Heading 1–6, Text, Bulleted list, Numbered list, To-do, Quote, Code, and
Divider; it is intentionally Markdown-specific rather than arbitrary child
nodes. The Rust App projects that vocabulary as `SemanticMenu`; Swift and web
render the App-owned descriptor over their native text controls. Choosing any
item returns its declared action to the same reducer and revision/ack path.

The terminal insert menu uses a compact bordered shortcut/name/sample layout
anchored to the caret, with a full-row gray keyboard or pointer selection.
The native popover leaves `NSTextView` as first responder, and a scoped key
monitor routes Up/Down, Home/End, Return/Tab, and Escape while the menu is
open. This prevents the popover from stealing document typing. Web keeps focus
in its textarea for the same reason. The standalone root Menu interpretation
remains focusable and keyboard navigable.

`markdown_delta_operations(previous, next)` turns an App-owned projection
change into one Unicode-safe contiguous range edit plus independent selection,
presentation, dirty, and metadata operations. Terminal multi-line drags and
double/triple-click selections therefore synchronize through the same
revision stream as native and web selections instead of replacing the whole
document root.

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

### Markdown semantic migration audit

The active navigation and editing surface graph is now fully projected:

- vault browsing is Tree, including filter, parent/directory/file roles,
  selection, open actions, location, and new-note primary action;
- new-note naming is a Page with Input and an App-owned back action;
- editing is MarkdownEditor with text, oriented selection, dirty/title,
  presentation, save/undo/redo, and compact deltas; and
- slash, backslash command palette, and selection context menus are
  SemanticMenu descriptors handled by the same reducer in every renderer.

No dialog, picker, menu, or editor state inside an attached Markdown App
session remains terminal-only. The first-run workspace chooser is Page +
Input; the editor title carries filename, save/dirty state, cursor position,
auto-save state, and transient notices; native/web task-marker clicks enter
the same text reducer; and trusted-local drops become semantic text edits
(browser sandboxing intentionally exposes filenames when it withholds paths).
The slash and context menus are closed SemanticMenu values, not renderer-owned
commands.

Caret/drop-hover visuals, local drag maps, and syntax colors remain
platform-specific presentation rather than alternate App state surfaces.
Unsupported renderers still receive the complete TUI.

### Five-App cross-platform audit

Kitchen Sink now builds and spawns Usage, Diffs, GitHub Issues, Markdown, and
File Tree against isolated deterministic fixtures. Its screen walker drives
the live App reducers over `ui.sock`; it does not substitute fixture snapshots
for the Apps. The current verified inventory is:

- Usage: provider catalog, provider detail, and Alerts;
- Diffs: changed-file list and complete styled Content detail;
- GitHub Issues: semantic filter/list and complete issue Content detail;
- Markdown: workspace chooser, Tree picker, new-note form, editor, context
  menu, and slash insert menu; and
- File Tree: root and nested Tree screens, including filter and context menu.

All five report **Terminal-only surfaces: none**. The retained Tree delta used
for directory navigation clears a disappearing selection before splicing the
new child collection, keeping every intermediate revision valid in Rust,
Swift, and web. The reproducible command and captured report live in
[`cross-platform-app-audit.md`](cross-platform-app-audit.md).

## Media v1

Media is a static-image component with no child nodes. Its snapshot contains:

- a source reference: trusted-local `path`, inline base64 capped at 256 KiB
  decoded, or `blob` with SHA-256, MIME type, and byte length;
- required intrinsic pixel dimensions;
- optional terminal `cells { w, h }` and native/web `points { w, h }` sizing,
  where either axis may be omitted and derived from intrinsic aspect;
- `contain`, `cover`, or `fill` fitting;
- alt text; and
- at most one optional `activate` action.

The reference is the transport. Large image bytes never enter snapshot or
delta JSON. `mediaSetSource` swaps the source plus intrinsic metadata in one
small operation. A blob resolver belongs to the existing Host route and its
session grants: App Kit supplies resolver hooks and verifies returned length
and SHA-256, but does not define a workspace server or a public filesystem
URL. Trusted local Swift and terminal renderers may open `path`; the Host must
translate it before forwarding a projection to a browser, and
`WorkspaceUiSession` defensively falls back to the PTY if one leaks through.

With the standalone `media` feature, `MediaPicker::from_query_stdio()` uses
`ratatui-image` capability detection. Kitty wins when available (including
Ghostty/libghostty), followed by iTerm2 or Sixel, with Unicode half-blocks as
the universal fallback. The component prepares a fixed Ratatui image at its
specified cell size so ordinary redraws remain cheap. The native renderer
loads `NSImage` locally or asynchronously through the Host blob resolver at
the resolved point size. The web renderer uses an accessible `<img>` with CSS
pixels and `object-fit`, and verifies resolved blobs before creating an object
URL.

Media v1 models static images only. It has no video, playback, frame, or
animation state. If an animated raster format reaches the terminal decoder,
only its first decoded frame is rendered; animation support for native/web is
a future component version, not part of this contract.

## Surface v1 embed

Surface is App Kit's canvas embed, not another component-rendering engine.
`UiComponent::Surface`, the shared fixtures, the Swift/web delegation wrappers,
and the default-off `surface-embed` Cargo integration are landed. The
`surface_planets` example loads unpeel-surface's existing planet WASM guest;
App Kit does not copy its scene logic or rendering path.

The design follows the working `unpeel-surface` runtime rather than copying
it. Its `host/src/ratatui.rs` already owns `SurfaceLayer`, `SurfaceView`, the
wgpu/WGSL renderer, mmap frame ring, Kitty image lifecycle, cell-to-logical
coordinate mapping, and automatic `UNPEEL_SURFACE_SOCKET` detection. Its
`host/src/remote.rs` owns the framed `USRF` v1 retained-scene/resource stream
and reverse event/resize packets. On Apple,
`RemoteSurfacePresenterView` consumes arbitrary USRF chunks and renders into a
`CAMetalLayer`; on web, `web/src/remote.rs` already combines `SceneDecoder`
with the shared WebGPU renderer and emits normalized Surface input.

The leaf component has a deliberately small closed specification:

- `reference { sessionId, streamId }` contains opaque, Host-resolved
  identifiers only. It never contains a Unix socket path, URL, credential,
  producer generation, scene command, resource body, or rendered pixel.
- `cells { w, h }` describes the terminal footprint and `points { w, h }`
  describes the native footprint; web treats points as CSS pixels. As with
  Media, either sizing object may be absent and either object may specify one
  axis, deriving the other from the current Surface logical-viewport aspect.
  With neither present, the containing pane or explicitly enumerated slot
  supplies the box.
- `background` is a closed policy: transparent, or a solid sRGBA color. It is
  the compositing background behind the Surface scene, not a scene command or
  arbitrary style map.
- `inputPolicy` is one of `none`, `pointer`, or `pointerAndKeyboard`.
  `pointer` covers pointer/touch, drag, scroll, and zoom. Keyboard events are
  forwarded only while the embed owns focus and only when USRF represents the
  event; text/document editing remains an App Kit semantic control rather than
  an invented canvas text protocol.

`Surface` has no arbitrary children, component slots, semantic actions, or
scene-shaped JSON. It may appear only as a root or in `CanvasPage.surface`, the
explicitly named slot whose schema admits exactly one Surface. The effective input permission
is the intersection of `inputPolicy` and the attached participant's existing
session grants; referencing a stream never grants permission to view or drive
it.

### CanvasPage + Button overlay

`CanvasPage` is the first closed composition around Surface. It contains
exactly one named `surface` slot and a bounded `controls` slot (32 entries)
whose v1 vocabulary accepts only `Button`. A Button has a stable id, label,
one action, and a closed `default`/`primary`/`destructive` role. There is no
generic child array, z-index, flexbox, coordinate, or arbitrary style map.

The overlay placement is part of the component contract: a toolbar across the
top of the canvas. Ratatui paints opaque toolbar cells over the local Surface
layer and exposes the same button rectangles for mouse hit-testing; SwiftUI
uses `CanvasPageView` with native Buttons over its injected Metal presenter;
web uses `CanvasPageRenderer` with an accessible `role=toolbar` over WebGPU.
Unknown control kinds trigger the normal whole-pane terminal fallback instead
of partially rendering a misleading canvas.

The `surface_canvas` example is fully standalone with only the
`surface-embed` feature, and optionally publishes the identical CanvasPage
tree when `ui-bridge` is enabled. Button actions are authenticated semantic
events on `unpeel.ui`; pointer/key input inside the remaining canvas rectangle
is USRF. The example maps Overview, Previous, Next, and Select Buttons to the
existing planet guest without putting a scene command or rendered frame in
semantic JSON.

### Local-GPU invariant: scenes, never frames

Every attached Surface client **must reconstruct and render the retained scene
locally on that client's own GPU** at its own resolution and backing scale.
USRF v1 scene commands and one-time, digest-verified immutable resources travel
to the presenter; rendered framebuffer images never do. An immutable RGBA image
resource referenced by a scene is permitted protocol input. A rasterized RGBA,
PNG, JPEG, video, or other per-frame representation of the composed scene is
not.

The existing Unpeel Host is a transport broker for this stream, never a
Surface renderer. After authentication and authorization it may envelope,
multiplex, journal/checkpoint, encrypt, and relay the USRF byte stream through
the existing Direct, SSH, or Link route, but the ordered USRF message bytes
forwarded to a presenter remain byte-for-byte unchanged. It must never decode
the scene for server-side rasterization, read a GPU framebuffer, transcode
frames, or substitute a frame-streaming protocol. Relay latency, an older
client, or a missing presenter capability does not relax this rule: the pane
uses its terminal fallback instead of silently moving rendering to the Host.

The mmap-backed Kitty path is exclusively a presentation medium for the local
terminal whose process can open the referenced frame files. Those file paths
and Kitty frame placements do not work across SSH and must never be forwarded
as a remote-Surface fallback. A remote terminal may still receive its ordinary
PTY stream, but its Surface layer travels separately over the connected USRF
presenter path and is rendered by that client's local wgpu/WebGPU/Metal
renderer. Apple connected presentation remains decoder-to-`CAMetalLayer` with
no frame readback, and web connected presentation remains decoder-to-WebGPU.

This scenes-never-frames rule is a load-bearing D14 invariant for every App Kit
Surface implementation, Host adapter, journal, relay, and reconnect path.

### Renderer integration

- **Terminal:** the default-off `surface-embed` Cargo feature adds the optional
  `unpeel-surface` dependency. App authors bind the opaque reference to the
  App-owned `Surface` adapter/`SurfaceLayer`; App Kit reserves the Ratatui cell rect,
  renders `SurfaceView` there, and delegates drawing, presentation, clearing,
  resizing, and coordinate conversion. Default and ordinary pure-TUI builds
  continue to pull neither `unpeel-surface` nor wgpu. The current
  `SurfaceLayer` is intentionally full-terminal, so arbitrary embedded rects
  require a small origin/rect API in `unpeel-surface`; App Kit must consume
  that API rather than reaching into `TerminalPresenter` or recreating its
  Kitty protocol.
- **Swift:** `SurfaceComponentView` allocates the component's point-size box
  and accepts an injected presenter view. The Host wraps Surface's
  transport-free `RemoteSurfacePresenterView` and feeds it USRF chunks from
  the existing authenticated Host route. `unpeel-surface` supplies both the
  UIKit and macOS AppKit/CAMetalLayer implementations; App Kit only allocates
  the semantic box and injects the presenter. A fixed logical viewport mode
  lets several differently sized local views consume one retained stream while
  mapping input back into the same scene coordinates.
- **Web:** `SurfaceRenderer` allocates the box and requires a
  `SurfacePresenterAdapter`; that adapter wraps the existing
  `web/src/remote.rs` `SceneDecoder`/WebGPU presenter around its canvas. The
  Host supplies routed chunks directly; it must not use the
  `surface-connect` development HTTP endpoint or implement another USRF
  decoder. Kitchen Sink's self-contained test adapter exposes a private,
  tokenized loopback HTTP stream solely to drive that existing presenter
  without Unpeel installed; production Hosts use their authenticated routes.

When a Host supplies the connected presenter used for terminal composition,
it also launches the producer with Surface's explicit
`SURFACE_TERMINAL_PROJECTION=retained-only` policy. The producer continues to
run the guest and publish retained scenes, but does not initialize a local GPU,
create mmap frames, or emit Kitty graphics into its PTY. A standalone launch
does not receive that variable and keeps Surface's normal local Kitty path.

The connected Swift presenters normalize pointer/touch, drag, scroll, zoom,
and focused keyboard/action input. The web package exposes
`sendRemoteKey(kind)`, so component adapters ask unpeel-surface to frame,
sequence, and send focused key actions instead of constructing USRF packets in
App Kit. A colocated Ghostty terminal keeps ordinary terminal keyboard input
separate. UIKit and AppKit presenters expose a local-only
`compositingBackgroundColor`, and WebGPU exposes `setRemoteBackground`; App Kit
passes the semantic component's closed background policy into both local GPU
compositors. The policy never enters USRF, but alpha resolves against the same
color in terminal, Metal, and WebGPU presentation.

Pointer and supported key events inside the allocated rectangle travel back
as USRF input after the Surface adapter maps them into logical coordinates.
All interaction outside that rectangle, plus App Kit controls over or beside
the canvas, stays on `unpeel.ui`. The same gesture or key must never be emitted
on both protocols.

The Host associates `sessionId`/`streamId` with its existing private Surface
producer connection and authorizes the requesting Controller or agent before
opening the stream. Those identifiers are broker routing metadata, not new
USRF header fields. A producer-generation change resets only the Surface
decoder/resource cache; an App Kit revision change resets neither Surface nor
the terminal. UI snapshots and deltas change only when the reference, sizing,
background, or input policy changes, so animation never drives semantic JSON.

### D14/D16 boundary

This composition is the scenes-versus-components split decided by D14 and
D16. Surface/USRF owns canvas scenes, GPU resources, pixels, presenter input,
and producer generations. App Kit/`unpeel.ui` owns Page, List, status, forms,
documents, accessibility, participant-aware actions, and semantic revisions.
App Kit Apps do not consume or project Surface guest capability exports such
as `surface_list_ptr/len` or `surface_status_ptr/len`; controls around an
embedded canvas use the App Kit vocabulary instead, so the protocols never
compete for the same UI.

Static Media remains a separate, simpler path: `ratatui-image` may use Kitty
for one referenced image, while Surface owns its dynamic wgpu/mmap/Kitty
pipeline. Neither implementation calls into or substitutes for the other.
If a renderer recognizes App Kit but lacks the Surface capability, cannot
resolve the authorized stream, or does not recognize the component version,
it keeps the attachment alive and falls back to the terminal view for the
complete pane.

## Revision and collaboration semantics

An event names the snapshot revision on which the interaction occurred. The
App accepts current-revision events, deduplicates their stable event IDs, and
publishes a new immutable revision after applying model changes. Stale events
receive a `stale` acknowledgement plus a fresh snapshot; future events are
rejected.

Server-to-client deltas are part of the foundation, not a post-DataGrid
optimization. `UiDelta` carries `baseRevision`, the next `revision`, and
ordered component operations. Markdown supports range replacement, selection,
presentation, dirty/read-only/title/placeholder/action updates; Media supports
an atomic source-reference and intrinsic-size swap; Page supports Toggle/Input
updates, ListItem insertion/removal, and keyed selection changes; and `replaceRoot` remains the escape
hatch. Swift and web clients apply deltas to their last complete snapshot and
expose the resulting complete state to renderers.

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

Tree and Menu now ship with all three interpretations. The remaining useful
vocabulary is intentionally conventional:

| Component | Ratatui foundation | Native/web meaning |
| --- | --- | --- |
| `Tabs` / `TabItem` | `ratatui::widgets::Tabs` | keyed tabs, `selectedId`, and one idempotent `select(id)` action; SwiftUI segmented control or `TabView`; web `tablist`/`tab` semantics with ARIA |
| `DataGrid` | table + virtual viewport | virtualized sheet with range/cell deltas |

Each should be added only with all three renderer interpretations and shared
fixtures. Media, Page, Tree, and Menu exercise the required pane-level
terminal fallback for renderers that do not advertise or recognize a kind;
every later component inherits that rule. This keeps App Kit opinionated and
prevents its public API from becoming an unbounded remote widget toolkit.
