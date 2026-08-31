# App Kit Kitchen Sink

This macOS 14+ SwiftUI executable is a self-contained mini-host for App Kit.
It does not install, import, or launch Unpeel. Run it from the repository root:

```sh
swift run --package-path swift/Examples/KitchenSink
```

The first launch fetches SwiftTerm 1.19.0 and builds the repository's Todo,
Markdown, and Media examples into `target/kitchen-sink`. Each example then
runs as the direct child of a real SwiftTerm PTY in a private, short-lived
`/tmp/upkit-…` session directory.

For each session the mini-host:

- creates `ui.sock` and a random per-session signing key;
- injects `UNPEEL_UI_SOCKET` and `UNPEEL_UI_TOKEN` only into the App process,
  along with its session id/directory;
- retains the signing key and mints short-lived, route-bound participant
  tokens with `UIParticipantTokenIssuer`;
- attaches `UIUnixSessionClient` to the same process shown in the terminal;
- switches among Terminal, Native, and Split lifecycle states so the Rust
  App's `UiBridge::should_render_terminal()` path is exercised; and
- keeps the session directory when the child is killed, so Restart proves
  state restore while producing a new `appInstanceId`.

The bottom harness can disconnect/reconnect the native renderer, restart the
authoritative App process, attach a hidden agent participant with selectable
grants, invoke a semantic action as that agent, inspect presence and final
acks, and distinguish raw snapshots from server deltas. The participant's
personalized title/alt text also proves `publish_to` isolation.

`TerminalEngineController` is the only SwiftTerm-specific boundary. Product
Hosts can replace that small wrapper with GhosttyKit without changing session,
token, renderer, or harness logic.

This package depends on the sibling `UnpeelAppKitUI` library, but is a separate
SwiftPM package and is never a dependency of that library. Its workflow is
manual-only, so ordinary Rust and renderer CI do not fetch or compile
SwiftTerm.
