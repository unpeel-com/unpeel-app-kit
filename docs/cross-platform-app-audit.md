# Cross-platform App audit

Verified on 2026-09-01 with the standalone Kitchen Sink mini-host on macOS.
The harness built each sibling App, spawned it as the direct child of a real
libghostty PTY, injected an isolated App-owned `ui.sock` and signing key, and
drove the live reducer with scoped semantic actions. SwiftUI and the bundled
DOM renderer consume that same projection; no Unpeel process or workspace
server participated.

Run it again from the App Kit repository root:

```sh
UNPEEL_KITCHEN_AUTO_WALK=1 \
UNPEEL_KITCHEN_AUTO_EXIT=1 \
UNPEEL_KITCHEN_AUDIT_REPORT=/tmp/app-kit-audit.md \
swift run --package-path swift/Examples/KitchenSink
```

| App | Screens observed from the live semantic channel | Terminal-only surfaces |
| --- | --- | --- |
| Usage | Provider catalog; provider detail; Alerts | None |
| Diffs | Changed-file list; complete styled diff Content detail | None |
| GitHub Issues | Filtered issue list; complete issue Content detail | None |
| Markdown | Workspace chooser; note Tree picker; new-note form; Markdown editor; editor context menu; slash insert menu | None |
| File Tree | Root Tree; nested Tree | None |

Every scripted action received an `applied` acknowledgement; revision numbers
remain App-owned and may advance further when a background refresh lands. The
walk also caught and fixed a retained-Tree delta ordering defect:
directory navigation now clears a selected ID that will disappear before the
child splice, then selects the new entry after the collection exists. Thus
every intermediate delta state is valid for Rust, Swift, and web renderers.

Platform-local presentation details—terminal cell geometry, syntax colors,
native animation, browser focus rings, and text-selection drawing—are not App
state surfaces and are intentionally renderer-specific. If a renderer lacks a
declared component capability, the whole pane falls back to the complete TUI;
that graceful fallback is not counted as a migrated screen.
