import { describe, expect, test } from "bun:test";

import {
  decodeUiMessage,
  diffText,
  negotiateUiProtocolVersion,
  type UiAttached,
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
    expect(messages).toHaveLength(10);
    expect(messages[0]?.type).toBe("attach");
    if (messages[0]?.type === "attach") {
      expect(messages[0].minProtocolVersion).toBe(1);
      expect(messages[0].maxProtocolVersion).toBe(1);
      expect(messages[0]).not.toHaveProperty("protocolVersion");
    }
    expect(messages[1]?.type).toBe("attached");
    const snapshot = messages[2] as UiSnapshot;
    expect(snapshot.appInstanceId).toBe("app-fixture");
    expect(snapshot.clientId).toBe("client-alice-mac");
    expect(snapshot.root.type).toBe("markdownEditor");
    expect(snapshot.root.text).toBe("# Hello\n🙂 world");
    expect(snapshot.root.selection.head.utf16Column).toBe(2);
    expect(messages[3]?.type).toBe("presence");
    if (messages[3]?.type === "presence") {
      expect(messages[3].members.map((member) => member.participant.id)).toEqual([
        "person-alice",
        "person-bob",
      ]);
    }
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
      authToken: "secret",
      participant: { id: "person-1", grants: ["view"] },
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
    expect(() => decodeUiMessage(snapshot)).toThrow("Unsupported UI component");
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
    expect(resume).not.toHaveProperty("participant");
    expect(resume).not.toHaveProperty("participantId");

    socket.message(attachedFrame());
    socket.message(snapshotFrame());
    socket.message({
      type: "presence",
      protocol: "unpeel.ui",
      protocolVersion: 1,
      appInstanceId: "app-fixture",
      viewId: "main",
      members: [{
        participant: { id: "person-alice", grants: ["view", "edit"] },
        clientId: "client-alice-web",
        renderer: { id: "renderer-alice-web", kind: "web" },
        state: { rendererVisible: true, terminalVisible: false },
      }],
    });
    expect(snapshots).toHaveLength(1);
    expect(presence?.members[0]?.participant).not.toHaveProperty("grants");
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
      baseRevision: 7,
      nodeId: "editor",
      action: "save",
      kind: "command",
    });
    expect(action).not.toHaveProperty("authToken");
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
});

describe("MarkdownEditor", () => {
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
