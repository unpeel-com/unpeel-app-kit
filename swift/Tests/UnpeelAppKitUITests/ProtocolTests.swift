import Foundation
import Testing

@testable import UnpeelAppKitUI

@Test
func sharedProtocolFixturesDecode() throws {
    let testDirectory = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
    let fixture = testDirectory
        .appendingPathComponent("../../../protocol/unpeel-ui-v1.ndjson")
        .standardizedFileURL
    let stream = try String(contentsOf: fixture, encoding: .utf8)
    let messages = try stream
        .split(separator: "\n")
        .map { try JSONDecoder().decode(UIMessage.self, from: Data($0.utf8)) }

    #expect(messages.count == 15)
    guard case let .attach(attach) = messages[0] else {
        Issue.record("first fixture must attach an authenticated participant")
        return
    }
    #expect(attach.participantToken.hasPrefix("upui1."))
    #expect(attach.minProtocolVersion == 1)
    #expect(attach.maxProtocolVersion == 1)

    guard case let .snapshot(snapshot) = messages[2],
          case let .markdownEditor(editor) = snapshot.root.component
    else {
        Issue.record("third fixture must contain a Markdown editor snapshot")
        return
    }
    #expect(snapshot.revision == 7)
    #expect(editor.text == "# Hello\n🙂 world")
    #expect(editor.selection.head == UITextPosition(line: 1, utf16Column: 2))
    #expect(editor.presentation == .split)

    guard case let .presence(presence) = messages[3] else {
        Issue.record("fourth fixture must contain multi-user presence")
        return
    }
    #expect(presence.members.count == 2)

    guard case let .event(edit) = messages[4],
          case let .textEdit(value) = edit.action.value
    else {
        Issue.record("fourth fixture must contain a text edit")
        return
    }
    #expect(value.range.end.utf16Column == 2)
    #expect(value.text == "Hello")

    guard case let .delta(delta) = messages[10] else {
        Issue.record("eleventh fixture must contain the Markdown server delta")
        return
    }
    let updated = try snapshot.applying(delta)
    guard case let .markdownEditor(updatedEditor) = updated.root.component else {
        Issue.record("delta result must remain a Markdown editor")
        return
    }
    #expect(updated.revision == 8)
    #expect(updatedEditor.text == "# Hello\nHello world")
    #expect(updatedEditor.selection.head == UITextPosition(line: 1, utf16Column: 5))

    guard case let .snapshot(mediaSnapshot) = messages[11],
          case let .media(media) = mediaSnapshot.root.component
    else {
        Issue.record("twelfth fixture must contain Media")
        return
    }
    #expect(media.alt == "Tiny fixture pixel")
    #expect(media.resolvedPointSize.w == 40)
    #expect(media.resolvedPointSize.h == 40)

    guard case let .delta(mediaDelta) = messages[12] else {
        Issue.record("thirteenth fixture must contain a Media reference delta")
        return
    }
    let updatedMedia = try mediaSnapshot.applying(mediaDelta)
    guard case let .media(nextMedia) = updatedMedia.root.component,
          case let .blob(reference) = nextMedia.source
    else {
        Issue.record("Media delta must replace the source with a blob reference")
        return
    }
    #expect(reference.byteLength == 68)

    guard case let .snapshot(todoSnapshot) = messages[13],
          case let .page(page) = todoSnapshot.root.component,
          case let .list(list) = page.body,
          case let .input(input) = page.header,
          case let .toggle(toggle)? = list.items[1].trailing
    else {
        Issue.record("fourteenth fixture must contain the canonical Todo Page")
        return
    }
    #expect(page.title == "Todos")
    #expect(list.items.map(\.label) == [
        "Run the standalone TUI",
        "Attach SwiftUI or web",
        "Invite an agent with edit grant",
    ])
    #expect(input.submit == "add-todo")
    #expect(toggle.value == false)
    #expect(page.requiredCapabilities == ["page", "list", "listItem", "input", "toggle"])

    guard case let .delta(todoDelta) = messages[14],
          case let .page(updatedPage) = try todoSnapshot.applying(todoDelta).root.component,
          case let .list(updatedList) = updatedPage.body
    else {
        Issue.record("fifteenth fixture must update Todo Page")
        return
    }
    #expect(updatedList.items[1].done)
}

@Test
func eventEncodingUsesTheWireEnvelope() throws {
    let snapshot = UISnapshot(
        appInstanceID: "app-fixture",
        clientID: "client-alice-mac",
        viewID: "main",
        revision: 3,
        root: UINode(
            id: "editor",
            component: .markdownEditor(MarkdownEditorSpec(
                text: "hello",
                selection: .caret(UITextPosition(line: 0, utf16Column: 0))
            ))
        )
    )
    let event = UIEvent(
        snapshot: snapshot,
        participantID: "person-alice",
        rendererID: "renderer-alice-swift",
        eventID: "event-save-1",
        action: UIAction(nodeID: "editor", action: "save", kind: .command)
    )
    let data = try JSONEncoder().encode(UIMessage.event(event))
    let object = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
    #expect(object["type"] as? String == "event")
    #expect(object["protocol"] as? String == "unpeel.ui")
    #expect(object["appInstanceId"] as? String == "app-fixture")
    #expect(object["participantId"] as? String == "person-alice")
    #expect(object["eventId"] as? String == "event-save-1")
    #expect(object["baseRevision"] as? Int == 3)
    #expect(object["nodeId"] as? String == "editor")
    #expect((object["value"] as? [String: Any])?["type"] as? String == "none")
}

@Test
func unsupportedProtocolVersionIsRejected() {
    let frame = Data(
        #"{"type":"error","protocol":"unpeel.ui","protocolVersion":2,"code":"test","message":"test"}"#.utf8
    )
    #expect(throws: DecodingError.self) {
        try JSONDecoder().decode(UIMessage.self, from: frame)
    }
}

@Test
func attachUsesVersionRangeNegotiation() throws {
    let attach = UIAttach(
        participantToken: "upui1.payload.signature",
        clientID: "client-1",
        renderer: UIRendererMetadata(id: "renderer-1", kind: "swiftUI"),
        viewID: "main",
        minProtocolVersion: 2,
        maxProtocolVersion: 3
    )
    let data = try JSONEncoder().encode(UIMessage.attach(attach))
    let object = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
    #expect(object["protocolVersion"] == nil)
    #expect(object["minProtocolVersion"] as? Int == 2)
    #expect(object["maxProtocolVersion"] as? Int == 3)
    #expect(try JSONDecoder().decode(UIMessage.self, from: data) == .attach(attach))
    #expect(UnpeelUIProtocol.negotiate(minimum: 1, maximum: 3) == 1)
    #expect(UnpeelUIProtocol.negotiate(minimum: 2, maximum: 3) == nil)
}

@Test
func nativeHostMintsRouteBoundAgentCredentials() throws {
    let issuer = try UIParticipantTokenIssuer(
        signingKey: "0123456789abcdef0123456789abcdef",
        appSessionID: "app-session"
    )
    let token = try issuer.issue(
        participant: UIParticipant(
            id: "agent:neighbor",
            kind: .agent,
            sourceSessionID: "neighbor-session",
            displayName: "Review agent",
            grants: ["view", "edit"]
        ),
        clientID: "client-agent",
        rendererID: "renderer-agent",
        viewID: "main",
        tokenID: "token-1",
        validFor: 60,
        now: Date(timeIntervalSince1970: 1_000)
    )
    #expect(token.hasPrefix("upui1."))
    #expect(token.split(separator: ".").count == 3)
    #expect(!token.contains("admin"))
}

@Test
func recognizedMessagesIgnoreUnknownFields() throws {
    let fixture: [String: Any] = [
        "type": "snapshot",
        "protocol": "unpeel.ui",
        "protocolVersion": 1,
        "appInstanceId": "app-fixture",
        "clientId": "client-1",
        "viewId": "main",
        "revision": 1,
        "futureEnvelopeField": ["v": 2],
        "root": [
            "id": "editor",
            "type": "markdownEditor",
            "text": "hello",
            "selection": [
                "anchor": ["line": 0, "utf16Column": 0],
                "head": ["line": 0, "utf16Column": 0],
                "futureSelectionField": true,
            ],
            "futureComponentField": "ignored",
        ],
    ]
    let data = try JSONSerialization.data(withJSONObject: fixture)
    guard case let .snapshot(snapshot) = try JSONDecoder().decode(UIMessage.self, from: data) else {
        Issue.record("recognized snapshot should decode")
        return
    }
    #expect(snapshot.root.id == "editor")
}

@Test
func unknownComponentDecodesForTerminalFallbackWithoutRejectingAttachment() throws {
    let fixture: [String: Any] = [
        "type": "snapshot",
        "protocol": "unpeel.ui",
        "protocolVersion": 1,
        "appInstanceId": "app-fixture",
        "clientId": "client-1",
        "viewId": "main",
        "revision": 1,
        "root": [
            "id": "future-root",
            "type": "futureComponent",
            "privatePayload": ["ignored": true],
        ],
    ]
    let data = try JSONSerialization.data(withJSONObject: fixture)
    guard case let .snapshot(snapshot) = try JSONDecoder().decode(UIMessage.self, from: data),
          case let .unsupported(kind) = snapshot.root.component
    else {
        Issue.record("unknown component must remain a decodable snapshot")
        return
    }
    #expect(kind == "futureComponent")
    #expect(snapshot.root.component.requiredCapability == nil)
}

@Test
func unknownPageSlotDecodesAndRequiresTerminalFallback() throws {
    let frame = Data(#"{"type":"snapshot","protocol":"unpeel.ui","protocolVersion":1,"appInstanceId":"app-fixture","clientId":"client-1","viewId":"main","revision":1,"root":{"id":"page","type":"page","title":"Future Page","body":{"type":"list","id":"rows","items":[{"id":"row-1","label":"Row","trailing":{"type":"futureControl","id":"control-1"}}]}}}"#.utf8)
    guard case let .snapshot(snapshot) = try JSONDecoder().decode(UIMessage.self, from: frame),
          case let .page(page) = snapshot.root.component,
          case let .list(list) = page.body,
          case let .unsupported(kind)? = list.items[0].trailing
    else {
        Issue.record("future row slot should stay decodable")
        return
    }
    #expect(kind == "futureControl")
    #expect(page.requiredCapabilities == nil)
    #expect(snapshot.root.component.requiredCapability == nil)
}

@Test
func pageInputAndListDeltasPreserveTheClosedRoot() throws {
    let first = UIListItemSpec(id: "todo-1", label: "First")
    let snapshot = UISnapshot(
        appInstanceID: "app-fixture",
        clientID: "client-1",
        viewID: "main",
        revision: 1,
        root: UINode(
            id: "todo-page",
            component: .page(PageSpec(
                title: "Todos",
                header: .input(UIInputSpec(id: "new-todo", label: "New todo")),
                body: .list(UIListSpec(id: "todos", items: [first]))
            ))
        )
    )
    let delta = UIDelta(
        appInstanceID: "app-fixture",
        clientID: "client-1",
        viewID: "main",
        baseRevision: 1,
        revision: 2,
        operations: [
            .inputSetValue(nodeID: "new-todo", value: "draft"),
            .listInsertItem(
                listID: "todos",
                index: 1,
                item: UIListItemSpec(id: "todo-2", label: "Second")
            ),
            .listRemoveItem(listID: "todos", itemID: "todo-1"),
        ]
    )
    guard case let .page(page) = try snapshot.applying(delta).root.component,
          case let .input(input)? = page.header,
          case let .list(list) = page.body
    else {
        Issue.record("Page deltas should preserve the Page root")
        return
    }
    #expect(input.value == "draft")
    #expect(list.items.map(\.id) == ["todo-2"])
}

@Test
func mediaPointSizingUsesExactIntegerAspectMath() {
    let media = MediaSpec(
        source: .path("/tmp/image.png"),
        intrinsic: MediaPixelSize(w: 4_294_967_291, h: 4_294_967_279),
        points: MediaPointSize(w: 4_294_967_283),
        alt: "Large dimensions"
    )
    #expect(media.resolvedPointSize.w == 4_294_967_283)
    #expect(media.resolvedPointSize.h == 4_294_967_272)
}

@Test
func mediaRejectsNoncanonicalInlineBytesAndActions() {
    let inline = Data(
        #"{"kind":"inline","mediaType":"image/png","base64":"AB=="}"#.utf8
    )
    #expect(throws: DecodingError.self) {
        try JSONDecoder().decode(MediaSource.self, from: inline)
    }

    let invalidAction = Data(
        #"{"source":{"kind":"path","path":"/tmp/image.png"},"intrinsic":{"w":1,"h":1},"alt":"Image","activate":"not portable"}"#.utf8
    )
    #expect(throws: DecodingError.self) {
        try JSONDecoder().decode(MediaSpec.self, from: invalidAction)
    }
}
