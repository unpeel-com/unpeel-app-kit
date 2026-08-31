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

    #expect(messages.count == 10)
    guard case let .attach(attach) = messages[0] else {
        Issue.record("first fixture must attach an authenticated participant")
        return
    }
    #expect(attach.participant.id == "person-alice")
    #expect(attach.grantsContain("edit"))
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
        authToken: "secret",
        participant: UIParticipant(id: "person-1", grants: ["view"]),
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

private extension UIAttach {
    func grantsContain(_ grant: String) -> Bool {
        participant.grants.contains(grant)
    }
}
