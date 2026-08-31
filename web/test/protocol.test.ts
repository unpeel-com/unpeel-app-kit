import { describe, expect, test } from "bun:test";

import {
  decodeUiMessage,
  diffText,
  markdownBlockReplacement,
  visibleMarkdownInsertItems,
  isMarkdownEditorNode,
  isMediaNode,
  isPageNode,
  isRenderablePageNode,
  negotiateUiProtocolVersion,
  applyUiDelta,
  resolveMediaPointSize,
  uiNodeCapability,
  uiNodeCapabilities,
  verifyMediaBlobBytes,
  type UiAttached,
  type UiDelta,
  type UiPresence,
  type UiSnapshot,
  type WorkspaceWebSocket,
  WorkspaceUiSession,
  uiAction,
  uiEvent,
} from "../src";

describe("shared protocol", () => {
  test("decodes every Rust and Swift fixture frame", async () => {
    const fixture = Bun.file(new URL("../../protocol/unpeel-ui-v1.ndjson", import.meta.url));
    const lines = (await fixture.text()).trim().split("\n");
    const messages = lines.map(decodeUiMessage);
    expect(messages).toHaveLength(15);
    expect(messages[0]?.type).toBe("attach");
    if (messages[0]?.type === "attach") {
      expect(messages[0].minProtocolVersion).toBe(1);
      expect(messages[0].maxProtocolVersion).toBe(1);
      expect(messages[0].participantToken).toStartWith("upui1.");
      expect(messages[0]).not.toHaveProperty("protocolVersion");
      expect(messages[0]).not.toHaveProperty("participant");
    }
    expect(messages[1]?.type).toBe("attached");
    const snapshot = messages[2] as UiSnapshot;
    expect(snapshot.appInstanceId).toBe("app-fixture");
    expect(snapshot.clientId).toBe("client-alice-mac");
    expect(snapshot.root.type).toBe("markdownEditor");
    if (!isMarkdownEditorNode(snapshot.root)) throw new Error("expected Markdown fixture");
    expect(snapshot.root.text).toBe("# Hello\n🙂 world");
    expect(snapshot.root.selection.head.utf16Column).toBe(2);
    expect(messages[3]?.type).toBe("presence");
    if (messages[3]?.type === "presence") {
      expect(messages[3].members.map((member) => member.participant.id)).toEqual([
        "person-alice",
        "person-bob",
      ]);
    }
    const delta = messages[10] as UiDelta;
    const updated = applyUiDelta(snapshot, delta);
    expect(updated.revision).toBe(8);
    if (!isMarkdownEditorNode(updated.root)) throw new Error("expected Markdown delta result");
    expect(updated.root.text).toBe("# Hello\nHello world");
    expect(updated.root.selection.head.utf16Column).toBe(5);

    const mediaSnapshot = messages[11] as UiSnapshot;
    if (!isMediaNode(mediaSnapshot.root)) throw new Error("expected Media fixture");
    expect(mediaSnapshot.root.alt).toBe("Tiny fixture pixel");
    expect(resolveMediaPointSize(mediaSnapshot.root)).toEqual({ w: 40, h: 40 });
    const mediaDelta = messages[12] as UiDelta;
    const updatedMedia = applyUiDelta(mediaSnapshot, mediaDelta);
    if (!isMediaNode(updatedMedia.root)) throw new Error("expected Media delta result");
    expect(updatedMedia.root.source.kind).toBe("blob");

    const todoSnapshot = messages[13] as UiSnapshot;
    if (!isPageNode(todoSnapshot.root) || !isRenderablePageNode(todoSnapshot.root)) {
      throw new Error("expected canonical Todo Page fixture");
    }
    expect(todoSnapshot.root.title).toBe("Todos");
    expect(todoSnapshot.root.body.items.map((item) => item.label)).toEqual([
      "Run the standalone TUI",
      "Attach SwiftUI or web",
      "Invite an agent with edit grant",
    ]);
    expect(uiNodeCapabilities(todoSnapshot.root)).toEqual([
      "page",
      "list",
      "listItem",
      "input",
      "toggle",
    ]);
    const todoDelta = messages[14] as UiDelta;
    const updatedTodo = applyUiDelta(todoSnapshot, todoDelta);
    if (!isRenderablePageNode(updatedTodo.root)) throw new Error("expected Todo Page delta");
    expect(updatedTodo.root.body.items[1]?.done).toBe(true);
  });

  test("builds the same command envelope", () => {
    const snapshot: UiSnapshot = {
      type: "snapshot",
      protocol: "unpeel.ui",
      protocolVersion: 1,
      appInstanceId: "app-fixture",
      clientId: "client-alice-mac",
      viewId: "main",
      revision: 7,
      root: {
        id: "editor",
        type: "markdownEditor",
        text: "# Hello",
        selection: {
          anchor: { line: 0, utf16Column: 7 },
          head: { line: 0, utf16Column: 7 },
        },
      },
    };
    const event = uiEvent(
      snapshot,
      "person-alice",
      "renderer-alice-web",
      uiAction("editor", "save", "command"),
      "event-save-1",
    );
    expect(event).toEqual({
      type: "event",
      protocol: "unpeel.ui",
      protocolVersion: 1,
      appInstanceId: "app-fixture",
      participantId: "person-alice",
      clientId: "client-alice-mac",
      rendererId: "renderer-alice-web",
      viewId: "main",
      eventId: "event-save-1",
      baseRevision: 7,
      nodeId: "editor",
      action: "save",
      kind: "command",
      value: { type: "none" },
    });
  });

  test("negotiates attach ranges and ignores unknown fields", () => {
    const attach = decodeUiMessage({
      type: "attach",
      protocol: "unpeel.ui",
      minProtocolVersion: 2,
      maxProtocolVersion: 3,
      participantToken: "upui1.payload.signature",
      clientId: "client-1",
      renderer: { id: "renderer-1", kind: "web" },
      viewId: "main",
      futureEnvelopeField: true,
    });
    expect(attach.type).toBe("attach");
    expect(negotiateUiProtocolVersion(1, 3)).toBe(1);
    expect(negotiateUiProtocolVersion(2, 3)).toBeUndefined();

    const snapshot = snapshotFrame() as unknown as Record<string, unknown>;
    snapshot.futureEnvelopeField = { v: 2 };
    const root = snapshot.root as Record<string, unknown>;
    root.futureComponentField = "ignored";
    expect(decodeUiMessage(snapshot).type).toBe("snapshot");

    root.type = "futureEditor";
    const future = decodeUiMessage(snapshot);
    expect(future.type).toBe("snapshot");
    if (future.type === "snapshot") {
      expect(uiNodeCapability(future.root)).toBeUndefined();
    }
  });

  test("keeps unknown Page slots attached but requires terminal fallback", () => {
    const future = decodeUiMessage({
      type: "snapshot",
      protocol: "unpeel.ui",
      protocolVersion: 1,
      appInstanceId: "app-fixture",
      clientId: "client-1",
      viewId: "main",
      revision: 1,
      root: {
        id: "page",
        type: "page",
        title: "Future Page",
        body: {
          type: "list",
          id: "rows",
          items: [{
            id: "row-1",
            label: "Row",
            trailing: { type: "futureControl", id: "control-1" },
          }],
        },
      },
    });
    expect(future.type).toBe("snapshot");
    if (future.type !== "snapshot") return;
    expect(isPageNode(future.root)).toBe(true);
    expect(isRenderablePageNode(future.root)).toBe(false);
    expect(uiNodeCapabilities(future.root)).toBeUndefined();
  });

  test("applies Input and List deltas to a Page without replacing its root", async () => {
    const lines = (await Bun.file(new URL(
      "../../protocol/unpeel-ui-v1.ndjson",
      import.meta.url,
    )).text()).trim().split("\n");
    const snapshot = decodeUiMessage(lines[13]!) as UiSnapshot;
    const delta: UiDelta = {
      type: "delta",
      protocol: "unpeel.ui",
      protocolVersion: 1,
      appInstanceId: snapshot.appInstanceId,
      clientId: snapshot.clientId,
      viewId: snapshot.viewId,
      baseRevision: 11,
      revision: 12,
      operations: [
        { op: "inputSetValue", nodeId: "new-todo", value: "draft" },
        {
          op: "listInsertItem",
          listId: "todos",
          index: 3,
          item: { id: "todo-4", label: "Fourth", done: false },
        },
        { op: "listRemoveItem", listId: "todos", itemId: "todo-1" },
      ],
    };
    const updated = applyUiDelta(snapshot, delta);
    if (!isRenderablePageNode(updated.root)) throw new Error("expected updated Page");
    expect(updated.root.header?.value).toBe("draft");
    expect(updated.root.body.items.map((item) => item.id)).toEqual([
      "todo-2",
      "todo-3",
      "todo-4",
    ]);
  });
});

describe("WorkspaceUiSession", () => {
  test("keeps the shared workspace fixture free of trusted identity", async () => {
    const fixture = Bun.file(new URL(
      "../../protocol/unpeel-workspace-ui-v1.ndjson",
      import.meta.url,
    ));
    const messages = (await fixture.text())
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line) as Record<string, unknown>);
    expect(messages.map((message) => message.type)).toEqual([
      "resume",
      "action",
      "lifecycle",
      "requestSnapshot",
    ]);
    for (const message of messages) {
      expect(message.protocol).toBe("unpeel.workspace.ui");
      expect(message).not.toHaveProperty("authToken");
      expect(message).not.toHaveProperty("participantToken");
      expect(message).not.toHaveProperty("participant");
      expect(message).not.toHaveProperty("participantId");
      expect(message).not.toHaveProperty("grants");
    }
  });

  test("keeps broker credentials and participant claims out of browser frames", () => {
    const socket = new FakeSocket();
    const snapshots: UiSnapshot[] = [];
    let presence: UiPresence | undefined;
    const session = new WorkspaceUiSession({
      url: "wss://workspace.example/apps/terminal-9/ui",
      appSessionId: "terminal-9",
      clientId: "client-alice-web",
      rendererId: "renderer-alice-web",
      viewId: "main",
      capabilities: ["markdownEditor"],
      supportedComponentCapabilities: ["markdownEditor"],
      onSnapshot: (snapshot) => snapshots.push(snapshot),
      onPresence: (value) => {
        presence = value;
      },
      webSocketFactory: () => socket,
    });

    session.start();
    socket.open();
    const resume = JSON.parse(socket.sent[0]!) as Record<string, unknown>;
    expect(resume.type).toBe("resume");
    expect(resume.protocol).toBe("unpeel.workspace.ui");
    expect(resume).not.toHaveProperty("authToken");
    expect(resume).not.toHaveProperty("participantToken");
    expect(resume).not.toHaveProperty("participant");
    expect(resume).not.toHaveProperty("participantId");
    expect((resume.renderer as { capabilities: string[] }).capabilities).toEqual([
      "markdownEditor",
      "serverDelta",
    ]);

    socket.message(attachedFrame());
    socket.message(snapshotFrame());
    socket.message({
      type: "presence",
      protocol: "unpeel.ui",
      protocolVersion: 1,
      appInstanceId: "app-fixture",
      viewId: "main",
      members: [{
        participant: {
          id: "agent:neighbor",
          kind: "agent",
          sourceSessionId: "session-neighbor",
          grants: ["view", "edit"],
        },
        clientId: "client-alice-web",
        renderer: { id: "renderer-alice-web", kind: "web" },
        state: { rendererVisible: true, terminalVisible: false },
      }],
    });
    expect(snapshots).toHaveLength(1);
    expect(presence?.members[0]?.participant).not.toHaveProperty("grants");
    expect(presence?.members[0]?.participant).not.toHaveProperty("sourceSessionId");
    socket.message({
      type: "delta",
      protocol: "unpeel.ui",
      protocolVersion: 1,
      appInstanceId: "app-fixture",
      clientId: "client-alice-web",
      viewId: "main",
      baseRevision: 7,
      revision: 8,
      operations: [{ op: "markdownSetDirty", nodeId: "editor", dirty: true }],
    });
    expect(snapshots).toHaveLength(2);
    expect(snapshots[1]?.revision).toBe(8);
    const dirtyRoot = snapshots[1]?.root;
    expect(dirtyRoot && isMarkdownEditorNode(dirtyRoot) ? dirtyRoot.dirty : undefined).toBe(true);
    expect(session.send(
      uiAction("editor", "save", "command"),
      "event-browser-save-1",
    )).toBe("event-browser-save-1");

    const action = JSON.parse(socket.sent.at(-1)!) as Record<string, unknown>;
    expect(action).toMatchObject({
      type: "action",
      protocol: "unpeel.workspace.ui",
      appSessionId: "terminal-9",
      appInstanceId: "app-fixture",
      clientId: "client-alice-web",
      rendererId: "renderer-alice-web",
      viewId: "main",
      eventId: "event-browser-save-1",
      baseRevision: 8,
      nodeId: "editor",
      action: "save",
      kind: "command",
    });
    expect(action).not.toHaveProperty("authToken");
    expect(action).not.toHaveProperty("participantToken");
    expect(action).not.toHaveProperty("participant");
    expect(action).not.toHaveProperty("participantId");
    expect(action).not.toHaveProperty("grants");
    session.stop();
  });

  test("resumes the same App and replays an unacknowledged event ID", async () => {
    const sockets: FakeSocket[] = [];
    const session = new WorkspaceUiSession({
      url: "wss://workspace.example/apps/terminal-9/ui",
      appSessionId: "terminal-9",
      clientId: "client-alice-web",
      rendererId: "renderer-alice-web",
      viewId: "main",
      onSnapshot: () => {},
      webSocketFactory: () => {
        const socket = new FakeSocket();
        sockets.push(socket);
        return socket;
      },
    });

    session.start();
    sockets[0]!.open();
    sockets[0]!.message(attachedFrame());
    sockets[0]!.message(snapshotFrame());
    session.send(uiAction("editor", "save", "command"), "event-replay-1");
    expect(session.pendingEventCount).toBe(1);
    sockets[0]!.close();

    await Bun.sleep(120);
    expect(sockets).toHaveLength(2);
    sockets[1]!.open();
    const resume = JSON.parse(sockets[1]!.sent[0]!) as Record<string, unknown>;
    expect(resume.expectedAppInstanceId).toBe("app-fixture");
    expect(resume.lastSeenRevision).toBe(7);
    sockets[1]!.message({ ...attachedFrame(), resumed: true });
    const replay = JSON.parse(sockets[1]!.sent[1]!) as Record<string, unknown>;
    expect(replay.eventId).toBe("event-replay-1");

    sockets[1]!.message({
      type: "ack",
      protocol: "unpeel.ui",
      protocolVersion: 1,
      appInstanceId: "app-fixture",
      clientId: "client-alice-web",
      rendererId: "renderer-alice-web",
      viewId: "main",
      eventId: "event-replay-1",
      status: "applied",
      revision: 8,
    });
    expect(session.pendingEventCount).toBe(0);
    session.stop();
  });

  test("falls back the pane without closing an unsupported component attachment", () => {
    const socket = new FakeSocket();
    const snapshots: UiSnapshot[] = [];
    const fallbacks: string[] = [];
    const session = new WorkspaceUiSession({
      url: "wss://workspace.example/apps/terminal-9/ui",
      appSessionId: "terminal-9",
      clientId: "client-alice-web",
      rendererId: "renderer-alice-web",
      viewId: "main",
      supportedComponentCapabilities: ["markdownEditor"],
      onSnapshot: (snapshot) => snapshots.push(snapshot),
      onTerminalFallback: (kind) => fallbacks.push(kind),
      webSocketFactory: () => socket,
    });

    session.start();
    socket.open();
    socket.message(attachedFrame());
    socket.message(mediaSnapshotFrame());

    expect(snapshots).toHaveLength(0);
    expect(fallbacks).toEqual(["media"]);
    expect(socket.readyState).toBe(1);
    const lifecycle = JSON.parse(socket.sent.at(-1)!) as Record<string, unknown>;
    expect(lifecycle).toMatchObject({
      type: "lifecycle",
      protocol: "unpeel.workspace.ui",
      state: { rendererVisible: false, terminalVisible: true },
    });
    expect(session.send(uiAction("hero-image", "open-image", "activate"))).toBeUndefined();

    socket.message({ ...snapshotFrame(), revision: 10 });
    expect(snapshots).toHaveLength(1);
    expect(snapshots[0]?.root.type).toBe("markdownEditor");
    const restored = JSON.parse(socket.sent.at(-1)!) as Record<string, unknown>;
    expect(restored).toMatchObject({
      type: "lifecycle",
      state: { rendererVisible: true, terminalVisible: false },
    });
    expect(session.send(
      uiAction("editor", "save", "command"),
      "event-after-fallback",
    )).toBe("event-after-fallback");
    session.stop();
  });

  test("falls back for unknown roots and prevents path Media from reaching browser renderers", () => {
    const socket = new FakeSocket();
    const snapshots: UiSnapshot[] = [];
    const fallbacks: string[] = [];
    const session = new WorkspaceUiSession({
      url: "wss://workspace.example/apps/terminal-9/ui",
      appSessionId: "terminal-9",
      clientId: "client-alice-web",
      rendererId: "renderer-alice-web",
      viewId: "main",
      onSnapshot: (snapshot) => snapshots.push(snapshot),
      onTerminalFallback: (kind) => fallbacks.push(kind),
      webSocketFactory: () => socket,
    });

    session.start();
    socket.open();
    socket.message(attachedFrame());
    socket.message({
      ...mediaSnapshotFrame(),
      root: {
        ...mediaSnapshotFrame().root,
        source: { kind: "path", path: "/private/app/secret.png" },
      },
    });
    socket.message({
      ...snapshotFrame(),
      revision: 10,
      root: { id: "future-root", type: "futureGrid", privatePayload: true },
    });

    expect(snapshots).toHaveLength(0);
    expect(fallbacks).toEqual(["media", "futureGrid"]);
    expect(socket.readyState).toBe(1);
    expect(socket.sent.join("\n")).not.toContain("/private/app/secret.png");
    session.stop();
  });

  test("falls back when any nested Page capability is missing", () => {
    const socket = new FakeSocket();
    const fallbacks: string[] = [];
    const session = new WorkspaceUiSession({
      url: "wss://workspace.example/apps/terminal-9/ui",
      appSessionId: "terminal-9",
      clientId: "client-alice-web",
      rendererId: "renderer-alice-web",
      viewId: "main",
      supportedComponentCapabilities: ["page", "list", "listItem", "input"],
      onSnapshot: () => { throw new Error("Page should use terminal fallback"); },
      onTerminalFallback: (kind) => fallbacks.push(kind),
      webSocketFactory: () => socket,
    });
    session.start();
    socket.open();
    socket.message(attachedFrame());
    socket.message({
      ...snapshotFrame(),
      root: {
        id: "todo-page",
        type: "page",
        title: "Todos",
        body: {
          type: "list",
          id: "todos",
          items: [{
            id: "todo-1",
            label: "First",
            done: false,
            trailing: {
              type: "toggle",
              id: "todo-1-toggle",
              label: "Completed",
              value: false,
              setValue: "set-done",
            },
          }],
        },
      },
    });
    expect(fallbacks).toEqual(["page"]);
    expect(socket.readyState).toBe(1);
    session.stop();
  });
});

describe("MarkdownEditor", () => {
  test("uses the closed insert vocabulary across renderers", () => {
    expect(visibleMarkdownInsertItems("todo").map((item) => item.kind)).toEqual(["todo"]);
    expect(markdownBlockReplacement("heading2", "  ")).toEqual({
      text: "  ## ",
      caretOffset: 5,
    });
    expect(markdownBlockReplacement("codeBlock", "")).toEqual({
      text: "```\n\n```",
      caretOffset: 4,
    });
  });

  test("produces a Unicode-safe minimal range edit", () => {
    expect(diffText("a🙂b\nold", "a🙂b\nnew")).toEqual({
      range: {
        start: { line: 1, utf16Column: 0 },
        end: { line: 1, utf16Column: 3 },
      },
      text: "new",
    });
  });

  test("never splits an emoji surrogate pair", () => {
    expect(diffText("a🙂b", "aXb")).toEqual({
      range: {
        start: { line: 0, utf16Column: 1 },
        end: { line: 0, utf16Column: 3 },
      },
      text: "X",
    });
  });
});

describe("Media", () => {
  test("rejects non-canonical inline base64", () => {
    const snapshot = mediaSnapshotFrame();
    expect(() => decodeUiMessage({
      ...snapshot,
      root: {
        ...snapshot.root,
        source: { kind: "inline", mediaType: "image/png", base64: "AB==" },
      },
    })).toThrow("base64");
  });

  test("derives omitted point axes with integer-safe aspect math", () => {
    const snapshot = mediaSnapshotFrame();
    if (!isMediaNode(snapshot.root)) throw new Error("expected Media fixture");
    const media = {
      ...snapshot.root,
      intrinsic: { w: 4_294_967_291, h: 4_294_967_279 },
      points: { w: 4_294_967_283 },
    };
    expect(resolveMediaPointSize(media)).toEqual({
      w: 4_294_967_283,
      h: 4_294_967_272,
    });
  });

  test("verifies bytes resolved through the broker by length and SHA-256", async () => {
    const binary = atob(
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
    );
    const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
    const source = {
      kind: "blob" as const,
      sha256: "431ced6916a2a21a156e38701afe55bbd7f88969fbbfc56d7fe099d47f265460",
      mediaType: "image/png",
      byteLength: 68,
    };
    await verifyMediaBlobBytes(source, bytes.buffer);
    await expect(verifyMediaBlobBytes(
      { ...source, sha256: "0".repeat(64) },
      bytes.buffer,
    )).rejects.toThrow("SHA-256 mismatch");
  });
});

class FakeSocket implements WorkspaceWebSocket {
  readyState = 0;
  readonly sent: string[] = [];
  private readonly listeners = new Map<string, EventListenerOrEventListenerObject[]>();

  addEventListener(type: string, listener: EventListenerOrEventListenerObject): void {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    this.readyState = 3;
    this.emit("close", new Event("close"));
  }

  open(): void {
    this.readyState = 1;
    this.emit("open", new Event("open"));
  }

  message(value: unknown): void {
    this.emit("message", new MessageEvent("message", { data: JSON.stringify(value) }));
  }

  private emit(type: string, event: Event): void {
    for (const listener of this.listeners.get(type) ?? []) {
      if (typeof listener === "function") listener(event);
      else listener.handleEvent(event);
    }
  }
}

function attachedFrame(): UiAttached {
  return {
    type: "attached",
    protocol: "unpeel.ui",
    protocolVersion: 1,
    minProtocolVersion: 1,
    maxProtocolVersion: 1,
    app: { id: "markdown", name: "Markdown", version: "0.1.0" },
    appInstanceId: "app-fixture",
    participantId: "person-alice",
    clientId: "client-alice-web",
    rendererId: "renderer-alice-web",
    viewId: "main",
    resumed: false,
    currentRevision: 7,
  };
}

function snapshotFrame(): UiSnapshot {
  return {
    type: "snapshot",
    protocol: "unpeel.ui",
    protocolVersion: 1,
    appInstanceId: "app-fixture",
    clientId: "client-alice-web",
    viewId: "main",
    revision: 7,
    root: {
      id: "editor",
      type: "markdownEditor",
      text: "# Hello",
      selection: {
        anchor: { line: 0, utf16Column: 7 },
        head: { line: 0, utf16Column: 7 },
      },
    },
  };
}

function mediaSnapshotFrame(): UiSnapshot {
  return {
    type: "snapshot",
    protocol: "unpeel.ui",
    protocolVersion: 1,
    appInstanceId: "app-fixture",
    clientId: "client-alice-web",
    viewId: "main",
    revision: 9,
    root: {
      id: "hero-image",
      type: "media",
      source: {
        kind: "inline",
        mediaType: "image/png",
        base64: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
      },
      intrinsic: { w: 1, h: 1 },
      points: { h: 40 },
      alt: "Tiny fixture pixel",
      activate: "open-image",
    },
  };
}
