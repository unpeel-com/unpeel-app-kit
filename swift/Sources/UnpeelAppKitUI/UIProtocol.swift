import Foundation

public enum UnpeelUIProtocol {
    public static let name = "unpeel.ui"
    public static let minimumVersion = 1
    public static let maximumVersion = 1
    public static let version = maximumVersion
    private static let maximumWireVersion = Int(UInt32.max)

    public static func supports(_ version: Int) -> Bool {
        (minimumVersion...maximumVersion).contains(version)
    }

    public static func negotiate(minimum: Int, maximum: Int) -> Int? {
        guard minimum > 0, minimum <= maximum, maximum <= maximumWireVersion else { return nil }
        let sharedMinimum = Swift.max(minimum, minimumVersion)
        let sharedMaximum = Swift.min(maximum, maximumVersion)
        return sharedMinimum <= sharedMaximum ? sharedMaximum : nil
    }
}

public struct AppMetadata: Codable, Equatable, Sendable {
    public let id: String
    public let name: String
    public let version: String
    public let description: String?

    public init(id: String, name: String, version: String, description: String? = nil) {
        self.id = id
        self.name = name
        self.version = version
        self.description = description
    }
}

/// Opaque workspace identity and broker-attested access grants.
public struct UIParticipant: Codable, Equatable, Sendable {
    public let id: String
    public let displayName: String?
    public let color: String?
    public let grants: [String]

    public init(
        id: String,
        displayName: String? = nil,
        color: String? = nil,
        grants: [String] = []
    ) {
        self.id = id
        self.displayName = displayName
        self.color = color
        self.grants = grants
    }

    enum CodingKeys: String, CodingKey {
        case id
        case displayName
        case color
        case grants
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        displayName = try container.decodeIfPresent(String.self, forKey: .displayName)
        color = try container.decodeIfPresent(String.self, forKey: .color)
        grants = try container.decodeIfPresent([String].self, forKey: .grants) ?? []
    }
}

public struct UIRendererMetadata: Codable, Equatable, Sendable {
    public let id: String
    public let kind: String
    public let capabilities: [String]

    public init(id: String, kind: String, capabilities: [String] = []) {
        self.id = id
        self.kind = kind
        self.capabilities = capabilities
    }

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case capabilities
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        kind = try container.decode(String.self, forKey: .kind)
        capabilities = try container.decodeIfPresent(
            [String].self,
            forKey: .capabilities
        ) ?? []
    }
}

public struct UIRendererState: Codable, Equatable, Sendable {
    public let rendererVisible: Bool
    public let terminalVisible: Bool

    public init(rendererVisible: Bool, terminalVisible: Bool) {
        self.rendererVisible = rendererVisible
        self.terminalVisible = terminalVisible
    }

    public static let terminal = Self(rendererVisible: false, terminalVisible: true)
    public static let component = Self(rendererVisible: true, terminalVisible: false)
    public static let hidden = Self(rendererVisible: false, terminalVisible: false)
}

/// Authenticated local-broker attachment. Never expose `authToken` to web code.
public struct UIAttach: Codable, Equatable, Sendable, CustomDebugStringConvertible {
    public let protocolName: String
    public let minProtocolVersion: Int
    public let maxProtocolVersion: Int
    public let authToken: String
    public let participant: UIParticipant
    public let clientID: String
    public let renderer: UIRendererMetadata
    public let viewID: String
    public let expectedAppInstanceID: String?
    public let lastSeenRevision: Int?
    public let state: UIRendererState

    public init(
        authToken: String,
        participant: UIParticipant,
        clientID: String,
        renderer: UIRendererMetadata,
        viewID: String,
        minProtocolVersion: Int = UnpeelUIProtocol.minimumVersion,
        maxProtocolVersion: Int = UnpeelUIProtocol.maximumVersion,
        expectedAppInstanceID: String? = nil,
        lastSeenRevision: Int? = nil,
        state: UIRendererState = .terminal
    ) {
        protocolName = UnpeelUIProtocol.name
        self.minProtocolVersion = minProtocolVersion
        self.maxProtocolVersion = maxProtocolVersion
        self.authToken = authToken
        self.participant = participant
        self.clientID = clientID
        self.renderer = renderer
        self.viewID = viewID
        self.expectedAppInstanceID = expectedAppInstanceID
        self.lastSeenRevision = lastSeenRevision
        self.state = state
    }

    public var debugDescription: String {
        "UIAttach(participant: \(participant.id), client: \(clientID), token: [REDACTED])"
    }

    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case minProtocolVersion
        case maxProtocolVersion
        case authToken
        case participant
        case clientID = "clientId"
        case renderer
        case viewID = "viewId"
        case expectedAppInstanceID = "expectedAppInstanceId"
        case lastSeenRevision
        case state
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        protocolName = try container.decode(String.self, forKey: .protocolName)
        minProtocolVersion = try container.decode(Int.self, forKey: .minProtocolVersion)
        maxProtocolVersion = try container.decode(Int.self, forKey: .maxProtocolVersion)
        guard minProtocolVersion > 0,
              minProtocolVersion <= maxProtocolVersion,
              maxProtocolVersion <= Int(UInt32.max)
        else {
            throw DecodingError.dataCorruptedError(
                forKey: .minProtocolVersion,
                in: container,
                debugDescription: "Invalid UI protocol version range"
            )
        }
        authToken = try container.decode(String.self, forKey: .authToken)
        participant = try container.decode(UIParticipant.self, forKey: .participant)
        clientID = try container.decode(String.self, forKey: .clientID)
        renderer = try container.decode(UIRendererMetadata.self, forKey: .renderer)
        viewID = try container.decode(String.self, forKey: .viewID)
        expectedAppInstanceID = try container.decodeIfPresent(
            String.self,
            forKey: .expectedAppInstanceID
        )
        lastSeenRevision = try container.decodeIfPresent(Int.self, forKey: .lastSeenRevision)
        state = try container.decodeIfPresent(UIRendererState.self, forKey: .state) ?? .terminal
    }
}

public struct UIAttached: Codable, Equatable, Sendable {
    public let protocolName: String
    public let protocolVersion: Int
    public let minProtocolVersion: Int
    public let maxProtocolVersion: Int
    public let app: AppMetadata
    public let appInstanceID: String
    public let participantID: String
    public let clientID: String
    public let rendererID: String
    public let viewID: String
    public let resumed: Bool
    public let currentRevision: Int?

    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case protocolVersion
        case minProtocolVersion
        case maxProtocolVersion
        case app
        case appInstanceID = "appInstanceId"
        case participantID = "participantId"
        case clientID = "clientId"
        case rendererID = "rendererId"
        case viewID = "viewId"
        case resumed
        case currentRevision
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        protocolName = try container.decode(String.self, forKey: .protocolName)
        protocolVersion = try container.decode(Int.self, forKey: .protocolVersion)
        minProtocolVersion = try container.decode(Int.self, forKey: .minProtocolVersion)
        maxProtocolVersion = try container.decode(Int.self, forKey: .maxProtocolVersion)
        guard minProtocolVersion > 0,
              minProtocolVersion <= maxProtocolVersion,
              maxProtocolVersion <= Int(UInt32.max),
              (minProtocolVersion...maxProtocolVersion).contains(protocolVersion)
        else {
            throw DecodingError.dataCorruptedError(
                forKey: .protocolVersion,
                in: container,
                debugDescription: "Selected UI protocol version is outside the server range"
            )
        }
        app = try container.decode(AppMetadata.self, forKey: .app)
        appInstanceID = try container.decode(String.self, forKey: .appInstanceID)
        participantID = try container.decode(String.self, forKey: .participantID)
        clientID = try container.decode(String.self, forKey: .clientID)
        rendererID = try container.decode(String.self, forKey: .rendererID)
        viewID = try container.decode(String.self, forKey: .viewID)
        resumed = try container.decode(Bool.self, forKey: .resumed)
        currentRevision = try container.decodeIfPresent(Int.self, forKey: .currentRevision)
    }
}

public struct UITextPosition: Codable, Equatable, Comparable, Sendable {
    public let line: Int
    public let utf16Column: Int

    public init(line: Int, utf16Column: Int) {
        self.line = line
        self.utf16Column = utf16Column
    }

    public static func < (lhs: Self, rhs: Self) -> Bool {
        (lhs.line, lhs.utf16Column) < (rhs.line, rhs.utf16Column)
    }
}

public struct UITextRange: Codable, Equatable, Sendable {
    public let start: UITextPosition
    public let end: UITextPosition

    public init(start: UITextPosition, end: UITextPosition) {
        self.start = start
        self.end = end
    }
}

public struct UITextSelection: Codable, Equatable, Sendable {
    public let anchor: UITextPosition
    public let head: UITextPosition

    public init(anchor: UITextPosition, head: UITextPosition) {
        self.anchor = anchor
        self.head = head
    }

    public static func caret(_ position: UITextPosition) -> Self {
        Self(anchor: position, head: position)
    }
}

public struct UITextEdit: Codable, Equatable, Sendable {
    public let range: UITextRange
    public let text: String

    public init(range: UITextRange, text: String) {
        self.range = range
        self.text = text
    }
}

public enum MarkdownPresentation: String, Codable, Equatable, Sendable {
    case source
    case preview
    case split
}

public struct MarkdownEditorActions: Codable, Equatable, Sendable {
    public let replaceRange: String?
    public let setSelection: String?
    public let save: String?
    public let undo: String?
    public let redo: String?
    public let setPresentation: String?

    public init(
        replaceRange: String? = "replace-range",
        setSelection: String? = "set-selection",
        save: String? = "save",
        undo: String? = "undo",
        redo: String? = "redo",
        setPresentation: String? = "set-presentation"
    ) {
        self.replaceRange = replaceRange
        self.setSelection = setSelection
        self.save = save
        self.undo = undo
        self.redo = redo
        self.setPresentation = setPresentation
    }
}

public struct MarkdownEditorSpec: Codable, Equatable, Sendable {
    public let text: String
    public let selection: UITextSelection
    public let presentation: MarkdownPresentation
    public let readOnly: Bool
    public let dirty: Bool
    public let placeholder: String
    public let title: String?
    public let actions: MarkdownEditorActions

    public init(
        text: String,
        selection: UITextSelection,
        presentation: MarkdownPresentation = .source,
        readOnly: Bool = false,
        dirty: Bool = false,
        placeholder: String = "",
        title: String? = nil,
        actions: MarkdownEditorActions = .init()
    ) {
        self.text = text
        self.selection = selection
        self.presentation = presentation
        self.readOnly = readOnly
        self.dirty = dirty
        self.placeholder = placeholder
        self.title = title
        self.actions = actions
    }

    enum CodingKeys: String, CodingKey {
        case text
        case selection
        case presentation
        case readOnly
        case dirty
        case placeholder
        case title
        case actions
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        text = try container.decode(String.self, forKey: .text)
        selection = try container.decode(UITextSelection.self, forKey: .selection)
        presentation = try container.decodeIfPresent(
            MarkdownPresentation.self,
            forKey: .presentation
        ) ?? .source
        readOnly = try container.decodeIfPresent(Bool.self, forKey: .readOnly) ?? false
        dirty = try container.decodeIfPresent(Bool.self, forKey: .dirty) ?? false
        placeholder = try container.decodeIfPresent(String.self, forKey: .placeholder) ?? ""
        title = try container.decodeIfPresent(String.self, forKey: .title)
        actions = try container.decodeIfPresent(
            MarkdownEditorActions.self,
            forKey: .actions
        ) ?? .init()
    }
}

public enum UIComponent: Equatable, Sendable {
    case markdownEditor(MarkdownEditorSpec)
}

public struct UINode: Equatable, Sendable {
    public let id: String
    public let component: UIComponent

    public init(id: String, component: UIComponent) {
        self.id = id
        self.component = component
    }
}

extension UINode: Codable {
    enum CodingKeys: String, CodingKey {
        case id
        case type
    }

    enum ComponentType: String, Codable {
        case markdownEditor
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        switch try container.decode(ComponentType.self, forKey: .type) {
        case .markdownEditor:
            component = .markdownEditor(try MarkdownEditorSpec(from: decoder))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(id, forKey: .id)
        switch component {
        case let .markdownEditor(editor):
            try container.encode(ComponentType.markdownEditor, forKey: .type)
            try editor.encode(to: encoder)
        }
    }
}

public struct UISnapshot: Codable, Equatable, Sendable {
    public let protocolName: String
    public let protocolVersion: Int
    public let appInstanceID: String
    public let clientID: String
    public let viewID: String
    public let revision: Int
    public let root: UINode

    public init(
        appInstanceID: String,
        clientID: String,
        viewID: String,
        revision: Int,
        root: UINode
    ) {
        protocolName = UnpeelUIProtocol.name
        protocolVersion = UnpeelUIProtocol.version
        self.appInstanceID = appInstanceID
        self.clientID = clientID
        self.viewID = viewID
        self.revision = revision
        self.root = root
    }

    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case protocolVersion
        case appInstanceID = "appInstanceId"
        case clientID = "clientId"
        case viewID = "viewId"
        case revision
        case root
    }
}

public enum UIEventKind: String, Codable, Equatable, Sendable {
    case activate
    case select
    case change
    case submit
    case cancel
    case command
}

public enum UIEventValue: Equatable, Sendable {
    case none
    case bool(Bool)
    case index(Int)
    case integer(Int)
    case number(Double)
    case text(String)
    case textList([String])
    case textEdit(UITextEdit)
    case textSelection(UITextSelection)
}

extension UIEventValue: Codable {
    enum CodingKeys: String, CodingKey {
        case type
        case value
    }

    enum ValueType: String, Codable {
        case none
        case bool
        case index
        case integer
        case number
        case text
        case textList
        case textEdit
        case textSelection
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(ValueType.self, forKey: .type) {
        case .none:
            self = .none
        case .bool:
            self = .bool(try container.decode(Bool.self, forKey: .value))
        case .index:
            self = .index(try container.decode(Int.self, forKey: .value))
        case .integer:
            self = .integer(try container.decode(Int.self, forKey: .value))
        case .number:
            self = .number(try container.decode(Double.self, forKey: .value))
        case .text:
            self = .text(try container.decode(String.self, forKey: .value))
        case .textList:
            self = .textList(try container.decode([String].self, forKey: .value))
        case .textEdit:
            self = .textEdit(try container.decode(UITextEdit.self, forKey: .value))
        case .textSelection:
            self = .textSelection(try container.decode(UITextSelection.self, forKey: .value))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .none:
            try container.encode(ValueType.none, forKey: .type)
        case let .bool(value):
            try container.encode(ValueType.bool, forKey: .type)
            try container.encode(value, forKey: .value)
        case let .index(value):
            try container.encode(ValueType.index, forKey: .type)
            try container.encode(value, forKey: .value)
        case let .integer(value):
            try container.encode(ValueType.integer, forKey: .type)
            try container.encode(value, forKey: .value)
        case let .number(value):
            try container.encode(ValueType.number, forKey: .type)
            try container.encode(value, forKey: .value)
        case let .text(value):
            try container.encode(ValueType.text, forKey: .type)
            try container.encode(value, forKey: .value)
        case let .textList(value):
            try container.encode(ValueType.textList, forKey: .type)
            try container.encode(value, forKey: .value)
        case let .textEdit(value):
            try container.encode(ValueType.textEdit, forKey: .type)
            try container.encode(value, forKey: .value)
        case let .textSelection(value):
            try container.encode(ValueType.textSelection, forKey: .type)
            try container.encode(value, forKey: .value)
        }
    }
}

/// Renderer-local action. The session transport applies identity and revision.
public struct UIAction: Codable, Equatable, Sendable {
    public let nodeID: String
    public let action: String
    public let kind: UIEventKind
    public let value: UIEventValue

    public init(
        nodeID: String,
        action: String,
        kind: UIEventKind,
        value: UIEventValue = .none
    ) {
        self.nodeID = nodeID
        self.action = action
        self.kind = kind
        self.value = value
    }

    enum CodingKeys: String, CodingKey {
        case nodeID = "nodeId"
        case action
        case kind
        case value
    }
}

public struct UIEvent: Equatable, Sendable {
    public let protocolName: String
    public let protocolVersion: Int
    public let appInstanceID: String
    public let participantID: String
    public let clientID: String
    public let rendererID: String
    public let viewID: String
    public let eventID: String
    public let baseRevision: Int
    public let action: UIAction

    public init(
        snapshot: UISnapshot,
        participantID: String,
        rendererID: String,
        eventID: String = UUID().uuidString.lowercased(),
        action: UIAction
    ) {
        protocolName = UnpeelUIProtocol.name
        protocolVersion = snapshot.protocolVersion
        appInstanceID = snapshot.appInstanceID
        self.participantID = participantID
        clientID = snapshot.clientID
        self.rendererID = rendererID
        viewID = snapshot.viewID
        self.eventID = eventID
        baseRevision = snapshot.revision
        self.action = action
    }
}

extension UIEvent: Codable {
    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case protocolVersion
        case appInstanceID = "appInstanceId"
        case participantID = "participantId"
        case clientID = "clientId"
        case rendererID = "rendererId"
        case viewID = "viewId"
        case eventID = "eventId"
        case baseRevision
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        protocolName = try container.decode(String.self, forKey: .protocolName)
        protocolVersion = try container.decode(Int.self, forKey: .protocolVersion)
        appInstanceID = try container.decode(String.self, forKey: .appInstanceID)
        participantID = try container.decode(String.self, forKey: .participantID)
        clientID = try container.decode(String.self, forKey: .clientID)
        rendererID = try container.decode(String.self, forKey: .rendererID)
        viewID = try container.decode(String.self, forKey: .viewID)
        eventID = try container.decode(String.self, forKey: .eventID)
        baseRevision = try container.decode(Int.self, forKey: .baseRevision)
        action = try UIAction(from: decoder)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(protocolName, forKey: .protocolName)
        try container.encode(protocolVersion, forKey: .protocolVersion)
        try container.encode(appInstanceID, forKey: .appInstanceID)
        try container.encode(participantID, forKey: .participantID)
        try container.encode(clientID, forKey: .clientID)
        try container.encode(rendererID, forKey: .rendererID)
        try container.encode(viewID, forKey: .viewID)
        try container.encode(eventID, forKey: .eventID)
        try container.encode(baseRevision, forKey: .baseRevision)
        try action.encode(to: encoder)
    }
}

public enum UIAckStatus: String, Codable, Equatable, Sendable {
    case pending
    case applied
    case rejected
    case stale
}

public struct UIAck: Codable, Equatable, Sendable {
    public let protocolName: String
    public let protocolVersion: Int
    public let appInstanceID: String
    public let clientID: String
    public let rendererID: String
    public let viewID: String
    public let eventID: String
    public let status: UIAckStatus
    public let revision: Int
    public let message: String?

    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case protocolVersion
        case appInstanceID = "appInstanceId"
        case clientID = "clientId"
        case rendererID = "rendererId"
        case viewID = "viewId"
        case eventID = "eventId"
        case status
        case revision
        case message
    }
}

public struct UILifecycle: Codable, Equatable, Sendable {
    public let protocolName: String
    public let protocolVersion: Int
    public let appInstanceID: String
    public let clientID: String
    public let rendererID: String
    public let viewID: String
    public let state: UIRendererState

    public init(snapshot: UISnapshot, rendererID: String, state: UIRendererState) {
        protocolName = UnpeelUIProtocol.name
        protocolVersion = snapshot.protocolVersion
        appInstanceID = snapshot.appInstanceID
        clientID = snapshot.clientID
        self.rendererID = rendererID
        viewID = snapshot.viewID
        self.state = state
    }

    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case protocolVersion
        case appInstanceID = "appInstanceId"
        case clientID = "clientId"
        case rendererID = "rendererId"
        case viewID = "viewId"
        case state
    }
}

public struct UIRequestSnapshot: Codable, Equatable, Sendable {
    public let protocolName: String
    public let protocolVersion: Int
    public let appInstanceID: String
    public let clientID: String
    public let rendererID: String
    public let viewID: String

    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case protocolVersion
        case appInstanceID = "appInstanceId"
        case clientID = "clientId"
        case rendererID = "rendererId"
        case viewID = "viewId"
    }
}

public struct UIPresenceMember: Codable, Equatable, Sendable {
    public let participant: UIParticipant
    public let clientID: String
    public let renderer: UIRendererMetadata
    public let state: UIRendererState

    enum CodingKeys: String, CodingKey {
        case participant
        case clientID = "clientId"
        case renderer
        case state
    }
}

public struct UIPresence: Codable, Equatable, Sendable {
    public let protocolName: String
    public let protocolVersion: Int
    public let appInstanceID: String
    public let viewID: String
    public let members: [UIPresenceMember]

    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case protocolVersion
        case appInstanceID = "appInstanceId"
        case viewID = "viewId"
        case members
    }
}

public struct UIErrorMessage: Codable, Equatable, Sendable {
    public let protocolName: String
    public let protocolVersion: Int
    public let code: String
    public let message: String

    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case protocolVersion
        case code
        case message
    }
}

public enum UIMessage: Equatable, Sendable {
    case attach(UIAttach)
    case attached(UIAttached)
    case snapshot(UISnapshot)
    case event(UIEvent)
    case ack(UIAck)
    case lifecycle(UILifecycle)
    case requestSnapshot(UIRequestSnapshot)
    case presence(UIPresence)
    case error(UIErrorMessage)
}

extension UIMessage: Codable {
    enum CodingKeys: String, CodingKey {
        case type
        case protocolName = "protocol"
        case protocolVersion
    }

    enum MessageType: String, Codable, Equatable {
        case attach
        case attached
        case snapshot
        case event
        case ack
        case lifecycle
        case requestSnapshot
        case presence
        case error
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let protocolName = try container.decode(String.self, forKey: .protocolName)
        guard protocolName == UnpeelUIProtocol.name else {
            throw DecodingError.dataCorruptedError(
                forKey: .protocolName,
                in: container,
                debugDescription: "Unsupported UI protocol \(protocolName)"
            )
        }
        let messageType = try container.decode(MessageType.self, forKey: .type)
        if messageType != .attach {
            let protocolVersion = try container.decode(Int.self, forKey: .protocolVersion)
            guard UnpeelUIProtocol.supports(protocolVersion) else {
                throw DecodingError.dataCorruptedError(
                    forKey: .protocolVersion,
                    in: container,
                    debugDescription: "Unsupported UI protocol version \(protocolVersion)"
                )
            }
        }
        switch messageType {
        case .attach:
            self = .attach(try UIAttach(from: decoder))
        case .attached:
            self = .attached(try UIAttached(from: decoder))
        case .snapshot:
            self = .snapshot(try UISnapshot(from: decoder))
        case .event:
            self = .event(try UIEvent(from: decoder))
        case .ack:
            self = .ack(try UIAck(from: decoder))
        case .lifecycle:
            self = .lifecycle(try UILifecycle(from: decoder))
        case .requestSnapshot:
            self = .requestSnapshot(try UIRequestSnapshot(from: decoder))
        case .presence:
            self = .presence(try UIPresence(from: decoder))
        case .error:
            self = .error(try UIErrorMessage(from: decoder))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case let .attach(message):
            try container.encode(MessageType.attach, forKey: .type)
            try message.encode(to: encoder)
        case let .attached(message):
            try container.encode(MessageType.attached, forKey: .type)
            try message.encode(to: encoder)
        case let .snapshot(message):
            try container.encode(MessageType.snapshot, forKey: .type)
            try message.encode(to: encoder)
        case let .event(message):
            try container.encode(MessageType.event, forKey: .type)
            try message.encode(to: encoder)
        case let .ack(message):
            try container.encode(MessageType.ack, forKey: .type)
            try message.encode(to: encoder)
        case let .lifecycle(message):
            try container.encode(MessageType.lifecycle, forKey: .type)
            try message.encode(to: encoder)
        case let .requestSnapshot(message):
            try container.encode(MessageType.requestSnapshot, forKey: .type)
            try message.encode(to: encoder)
        case let .presence(message):
            try container.encode(MessageType.presence, forKey: .type)
            try message.encode(to: encoder)
        case let .error(message):
            try container.encode(MessageType.error, forKey: .type)
            try message.encode(to: encoder)
        }
    }
}

public extension UIMessage {
    /// The selected connection version, or `nil` for the range-bearing attach.
    var protocolVersion: Int? {
        switch self {
        case .attach:
            nil
        case let .attached(message):
            message.protocolVersion
        case let .snapshot(message):
            message.protocolVersion
        case let .event(message):
            message.protocolVersion
        case let .ack(message):
            message.protocolVersion
        case let .lifecycle(message):
            message.protocolVersion
        case let .requestSnapshot(message):
            message.protocolVersion
        case let .presence(message):
            message.protocolVersion
        case let .error(message):
            message.protocolVersion
        }
    }
}
