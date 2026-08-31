import Foundation

public enum UnpeelUIProtocol {
    public static let name = "unpeel.ui"
    public static let minimumVersion = 1
    public static let maximumVersion = 1
    public static let version = maximumVersion
    public static let deltaCapability = "serverDelta"
    public static let markdownEditorCapability = "markdownEditor"
    public static let mediaCapability = "media"
    public static let pageCapability = "page"
    public static let listCapability = "list"
    public static let listItemCapability = "listItem"
    public static let toggleCapability = "toggle"
    public static let inputCapability = "input"
    public static let supportedComponentCapabilities = [
        markdownEditorCapability,
        mediaCapability,
        pageCapability,
        listCapability,
        listItemCapability,
        toggleCapability,
        inputCapability,
    ]
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

public enum UIParticipantKind: String, Codable, Equatable, Sendable {
    case human
    case agent
    case service
}

/// Opaque Host identity and signed access grants. Agents use this same type.
public struct UIParticipant: Codable, Equatable, Sendable {
    public let id: String
    public let kind: UIParticipantKind
    public let sourceSessionID: String?
    public let displayName: String?
    public let color: String?
    public let grants: [String]

    public init(
        id: String,
        kind: UIParticipantKind = .human,
        sourceSessionID: String? = nil,
        displayName: String? = nil,
        color: String? = nil,
        grants: [String] = []
    ) {
        self.id = id
        self.kind = kind
        self.sourceSessionID = sourceSessionID
        self.displayName = displayName
        self.color = color
        self.grants = grants
    }

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case sourceSessionID = "sourceSessionId"
        case displayName
        case color
        case grants
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        kind = try container.decodeIfPresent(UIParticipantKind.self, forKey: .kind) ?? .human
        sourceSessionID = try container.decodeIfPresent(String.self, forKey: .sourceSessionID)
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

/// Scoped local attachment. Never expose `participantToken` to web code.
public struct UIAttach: Codable, Equatable, Sendable, CustomDebugStringConvertible {
    public let protocolName: String
    public let minProtocolVersion: Int
    public let maxProtocolVersion: Int
    public let participantToken: String
    public let clientID: String
    public let renderer: UIRendererMetadata
    public let viewID: String
    public let expectedAppInstanceID: String?
    public let lastSeenRevision: Int?
    public let state: UIRendererState

    public init(
        participantToken: String,
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
        self.participantToken = participantToken
        self.clientID = clientID
        self.renderer = renderer
        self.viewID = viewID
        self.expectedAppInstanceID = expectedAppInstanceID
        self.lastSeenRevision = lastSeenRevision
        self.state = state
    }

    public var debugDescription: String {
        "UIAttach(client: \(clientID), participantToken: [REDACTED])"
    }

    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case minProtocolVersion
        case maxProtocolVersion
        case participantToken
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
        participantToken = try container.decode(String.self, forKey: .participantToken)
        guard !participantToken.isEmpty, participantToken.utf8.count <= 16_384 else {
            throw DecodingError.dataCorruptedError(
                forKey: .participantToken,
                in: container,
                debugDescription: "participantToken must contain 1...16384 bytes"
            )
        }
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

public enum MediaFit: String, Codable, Equatable, Hashable, Sendable {
    case contain
    case cover
    case fill
}

public struct MediaPixelSize: Codable, Equatable, Hashable, Sendable {
    public let w: Int
    public let h: Int

    public init(w: Int, h: Int) {
        self.w = w
        self.h = h
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        w = try container.decode(Int.self, forKey: .w)
        h = try container.decode(Int.self, forKey: .h)
        guard (1...Int(UInt32.max)).contains(w), (1...Int(UInt32.max)).contains(h) else {
            throw DecodingError.dataCorruptedError(
                forKey: .w,
                in: container,
                debugDescription: "Media intrinsic dimensions must be positive UInt32 values"
            )
        }
    }

    enum CodingKeys: String, CodingKey {
        case w
        case h
    }
}

public struct MediaCellSize: Codable, Equatable, Hashable, Sendable {
    public let w: Int?
    public let h: Int?

    public init(w: Int? = nil, h: Int? = nil) {
        self.w = w
        self.h = h
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        w = try container.decodeIfPresent(Int.self, forKey: .w)
        h = try container.decodeIfPresent(Int.self, forKey: .h)
        guard w != nil || h != nil,
              w.map({ (1...Int(UInt16.max)).contains($0) }) ?? true,
              h.map({ (1...Int(UInt16.max)).contains($0) }) ?? true
        else {
            throw DecodingError.dataCorruptedError(
                forKey: .w,
                in: container,
                debugDescription: "Media cell size needs at least one positive UInt16 axis"
            )
        }
    }

    enum CodingKeys: String, CodingKey {
        case w
        case h
    }
}

public struct MediaPointSize: Codable, Equatable, Hashable, Sendable {
    public let w: Int?
    public let h: Int?

    public init(w: Int? = nil, h: Int? = nil) {
        self.w = w
        self.h = h
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        w = try container.decodeIfPresent(Int.self, forKey: .w)
        h = try container.decodeIfPresent(Int.self, forKey: .h)
        guard w != nil || h != nil,
              w.map({ (1...Int(UInt32.max)).contains($0) }) ?? true,
              h.map({ (1...Int(UInt32.max)).contains($0) }) ?? true
        else {
            throw DecodingError.dataCorruptedError(
                forKey: .w,
                in: container,
                debugDescription: "Media point size needs at least one positive UInt32 axis"
            )
        }
    }

    enum CodingKeys: String, CodingKey {
        case w
        case h
    }
}

public struct MediaBlobReference: Codable, Equatable, Hashable, Sendable {
    public let sha256: String
    public let mediaType: String
    public let byteLength: Int

    public init(sha256: String, mediaType: String, byteLength: Int) {
        self.sha256 = sha256
        self.mediaType = mediaType
        self.byteLength = byteLength
    }
}

public enum MediaSource: Equatable, Hashable, Sendable {
    case path(String)
    case inline(mediaType: String, base64: String)
    case blob(MediaBlobReference)
}

extension MediaSource: Codable {
    enum CodingKeys: String, CodingKey {
        case kind
        case path
        case mediaType
        case base64
        case sha256
        case byteLength
    }

    enum Kind: String, Codable {
        case path
        case inline
        case blob
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .path:
            let path = try container.decode(String.self, forKey: .path)
            guard !path.isEmpty, path.utf8.count <= 4_096, !path.contains("\0") else {
                throw DecodingError.dataCorruptedError(
                    forKey: .path,
                    in: container,
                    debugDescription: "Media path must contain 1...4096 non-NUL bytes"
                )
            }
            self = .path(path)
        case .inline:
            let mediaType = try container.decode(String.self, forKey: .mediaType)
            let base64 = try container.decode(String.self, forKey: .base64)
            try Self.validateMediaType(mediaType, key: .mediaType, in: container)
            guard base64.utf8.count <= 349_528,
                  let data = Data(base64Encoded: base64),
                  data.base64EncodedString() == base64,
                  (1...262_144).contains(data.count)
            else {
                throw DecodingError.dataCorruptedError(
                    forKey: .base64,
                    in: container,
                    debugDescription: "Inline Media must be valid base64 containing at most 256 KiB"
                )
            }
            self = .inline(mediaType: mediaType, base64: base64)
        case .blob:
            let reference = MediaBlobReference(
                sha256: try container.decode(String.self, forKey: .sha256),
                mediaType: try container.decode(String.self, forKey: .mediaType),
                byteLength: try container.decode(Int.self, forKey: .byteLength)
            )
            try Self.validateMediaType(reference.mediaType, key: .mediaType, in: container)
            guard reference.sha256.count == 64,
                  reference.sha256.utf8.allSatisfy({
                      (48...57).contains($0) || (97...102).contains($0)
                  }),
                  (1...9_007_199_254_740_991).contains(reference.byteLength)
            else {
                throw DecodingError.dataCorruptedError(
                    forKey: .sha256,
                    in: container,
                    debugDescription: "Media blob metadata is invalid"
                )
            }
            self = .blob(reference)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case let .path(path):
            try container.encode(Kind.path, forKey: .kind)
            try container.encode(path, forKey: .path)
        case let .inline(mediaType, base64):
            try container.encode(Kind.inline, forKey: .kind)
            try container.encode(mediaType, forKey: .mediaType)
            try container.encode(base64, forKey: .base64)
        case let .blob(reference):
            try container.encode(Kind.blob, forKey: .kind)
            try container.encode(reference.sha256, forKey: .sha256)
            try container.encode(reference.mediaType, forKey: .mediaType)
            try container.encode(reference.byteLength, forKey: .byteLength)
        }
    }

    private static func validateMediaType<Key: CodingKey>(
        _ value: String,
        key: Key,
        in container: KeyedDecodingContainer<Key>
    ) throws {
        let punctuation = Set("!#$&^_.+-/".utf8)
        guard value.hasPrefix("image/"),
              value.utf8.count <= 127,
              value.utf8.allSatisfy({
                  (48...57).contains($0)
                      || (65...90).contains($0)
                      || (97...122).contains($0)
                      || punctuation.contains($0)
              })
        else {
            throw DecodingError.dataCorruptedError(
                forKey: key,
                in: container,
                debugDescription: "Media mediaType must be an image MIME type"
            )
        }
    }
}

public struct MediaSpec: Codable, Equatable, Hashable, Sendable {
    public let source: MediaSource
    public let intrinsic: MediaPixelSize
    public let cells: MediaCellSize?
    public let points: MediaPointSize?
    public let fit: MediaFit
    public let alt: String
    public let activate: String?

    public init(
        source: MediaSource,
        intrinsic: MediaPixelSize,
        cells: MediaCellSize? = nil,
        points: MediaPointSize? = nil,
        fit: MediaFit = .contain,
        alt: String,
        activate: String? = nil
    ) {
        self.source = source
        self.intrinsic = intrinsic
        self.cells = cells
        self.points = points
        self.fit = fit
        self.alt = alt
        self.activate = activate
    }

    enum CodingKeys: String, CodingKey {
        case source
        case intrinsic
        case cells
        case points
        case fit
        case alt
        case activate
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        source = try container.decode(MediaSource.self, forKey: .source)
        intrinsic = try container.decode(MediaPixelSize.self, forKey: .intrinsic)
        cells = try container.decodeIfPresent(MediaCellSize.self, forKey: .cells)
        points = try container.decodeIfPresent(MediaPointSize.self, forKey: .points)
        fit = try container.decodeIfPresent(MediaFit.self, forKey: .fit) ?? .contain
        alt = try container.decode(String.self, forKey: .alt)
        activate = try container.decodeIfPresent(String.self, forKey: .activate)
        guard alt.utf8.count <= 16_384 else {
            throw DecodingError.dataCorruptedError(
                forKey: .alt,
                in: container,
                debugDescription: "Media alt text must contain at most 16384 bytes"
            )
        }
        if let activate {
            guard !activate.isEmpty,
                  activate.utf8.count <= 256,
                  activate.utf8.allSatisfy({
                      (48...57).contains($0)
                          || (65...90).contains($0)
                          || (97...122).contains($0)
                          || [46, 95, 58, 47, 45].contains($0)
                  })
            else {
                throw DecodingError.dataCorruptedError(
                    forKey: .activate,
                    in: container,
                    debugDescription: "Media activate must be a portable action identifier"
                )
            }
        }
    }

    public var resolvedPointSize: (w: Int, h: Int) {
        guard let points else { return (intrinsic.w, intrinsic.h) }
        switch (points.w, points.h) {
        case let (w?, h?):
            return (w, h)
        case let (w?, nil):
            return (w, ratioCeil(w, intrinsic.h, intrinsic.w))
        case let (nil, h?):
            return (ratioCeil(h, intrinsic.w, intrinsic.h), h)
        case (nil, nil):
            return (intrinsic.w, intrinsic.h)
        }
    }
}

private func ratioCeil(_ value: Int, _ numerator: Int, _ denominator: Int) -> Int {
    let scaled = UInt64(value) * UInt64(numerator)
    let divisor = UInt64(max(denominator, 1))
    let resolved = scaled / divisor + (scaled % divisor == 0 ? 0 : 1)
    return Int(min(resolved, UInt64(UInt32.max)))
}

public struct UIToggleSpec: Codable, Equatable, Hashable, Sendable {
    public let id: String
    public let label: String
    public let value: Bool
    public let setValue: String

    public init(id: String, label: String, value: Bool, setValue: String) {
        self.id = id
        self.label = label
        self.value = value
        self.setValue = setValue
    }
}

public enum UIListItemSlot: Equatable, Hashable, Sendable {
    case toggle(UIToggleSpec)
    case unsupported(kind: String)

    public var kind: String {
        switch self {
        case .toggle: "toggle"
        case let .unsupported(kind): kind
        }
    }
}

extension UIListItemSlot: Codable {
    enum CodingKeys: String, CodingKey { case type }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let kind = try container.decode(String.self, forKey: .type)
        switch kind {
        case "toggle": self = .toggle(try UIToggleSpec(from: decoder))
        default: self = .unsupported(kind: kind)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case let .toggle(toggle):
            try container.encode("toggle", forKey: .type)
            try toggle.encode(to: encoder)
        case let .unsupported(kind):
            try container.encode(kind, forKey: .type)
        }
    }
}

public struct UIListItemSpec: Codable, Equatable, Hashable, Identifiable, Sendable {
    public let id: String
    public let label: String
    public var done: Bool
    public var leading: UIListItemSlot?
    public var trailing: UIListItemSlot?
    public var accessory: UIListItemSlot?
    public let delete: String?

    public init(
        id: String,
        label: String,
        done: Bool = false,
        leading: UIListItemSlot? = nil,
        trailing: UIListItemSlot? = nil,
        accessory: UIListItemSlot? = nil,
        delete: String? = nil
    ) {
        self.id = id
        self.label = label
        self.done = done
        self.leading = leading
        self.trailing = trailing
        self.accessory = accessory
        self.delete = delete
    }

    enum CodingKeys: String, CodingKey {
        case id, label, done, leading, trailing, accessory, delete
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        label = try container.decode(String.self, forKey: .label)
        done = try container.decodeIfPresent(Bool.self, forKey: .done) ?? false
        leading = try container.decodeIfPresent(UIListItemSlot.self, forKey: .leading)
        trailing = try container.decodeIfPresent(UIListItemSlot.self, forKey: .trailing)
        accessory = try container.decodeIfPresent(UIListItemSlot.self, forKey: .accessory)
        delete = try container.decodeIfPresent(String.self, forKey: .delete)
        let toggles = [leading, trailing, accessory].compactMap { slot -> UIToggleSpec? in
            guard case let .toggle(toggle) = slot else { return nil }
            return toggle
        }
        guard toggles.count <= 1, toggles.first?.value ?? done == done else {
            throw DecodingError.dataCorruptedError(
                forKey: .done,
                in: container,
                debugDescription: "ListItem accepts one completion Toggle whose value matches done"
            )
        }
    }
}

public struct UIListSpec: Codable, Equatable, Hashable, Sendable {
    public let id: String
    public var items: [UIListItemSpec]
    public let emptyMessage: String

    public init(id: String, items: [UIListItemSpec], emptyMessage: String = "") {
        self.id = id
        self.items = items
        self.emptyMessage = emptyMessage
    }

    enum CodingKeys: String, CodingKey { case id, items, emptyMessage }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        items = try container.decode([UIListItemSpec].self, forKey: .items)
        emptyMessage = try container.decodeIfPresent(String.self, forKey: .emptyMessage) ?? ""
    }
}

public struct UIInputSpec: Codable, Equatable, Hashable, Sendable {
    public let id: String
    public let label: String
    public var value: String
    public let placeholder: String
    public let setValue: String?
    public let submit: String?

    public init(
        id: String,
        label: String,
        value: String = "",
        placeholder: String = "",
        setValue: String? = nil,
        submit: String? = nil
    ) {
        self.id = id
        self.label = label
        self.value = value
        self.placeholder = placeholder
        self.setValue = setValue
        self.submit = submit
    }

    enum CodingKeys: String, CodingKey { case id, label, value, placeholder, setValue, submit }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        label = try container.decode(String.self, forKey: .label)
        value = try container.decodeIfPresent(String.self, forKey: .value) ?? ""
        placeholder = try container.decodeIfPresent(String.self, forKey: .placeholder) ?? ""
        setValue = try container.decodeIfPresent(String.self, forKey: .setValue)
        submit = try container.decodeIfPresent(String.self, forKey: .submit)
    }
}

public enum UIPageHeaderSlot: Equatable, Hashable, Sendable {
    case input(UIInputSpec)
    case unsupported(kind: String)
}

extension UIPageHeaderSlot: Codable {
    enum CodingKeys: String, CodingKey { case type }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let kind = try container.decode(String.self, forKey: .type)
        switch kind {
        case "input": self = .input(try UIInputSpec(from: decoder))
        default: self = .unsupported(kind: kind)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case let .input(input):
            try container.encode("input", forKey: .type)
            try input.encode(to: encoder)
        case let .unsupported(kind):
            try container.encode(kind, forKey: .type)
        }
    }
}

public enum UIPageBodySlot: Equatable, Hashable, Sendable {
    case list(UIListSpec)
    case unsupported(kind: String)
}

extension UIPageBodySlot: Codable {
    enum CodingKeys: String, CodingKey { case type }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let kind = try container.decode(String.self, forKey: .type)
        switch kind {
        case "list": self = .list(try UIListSpec(from: decoder))
        default: self = .unsupported(kind: kind)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case let .list(list):
            try container.encode("list", forKey: .type)
            try list.encode(to: encoder)
        case let .unsupported(kind):
            try container.encode(kind, forKey: .type)
        }
    }
}

public struct PageSpec: Codable, Equatable, Hashable, Sendable {
    public let title: String
    public var header: UIPageHeaderSlot?
    public var body: UIPageBodySlot

    public init(title: String, header: UIPageHeaderSlot? = nil, body: UIPageBodySlot) {
        self.title = title
        self.header = header
        self.body = body
    }

    public var requiredCapabilities: [String]? {
        var capabilities = [
            UnpeelUIProtocol.pageCapability,
            UnpeelUIProtocol.listCapability,
            UnpeelUIProtocol.listItemCapability,
        ]
        if let header {
            guard case .input = header else { return nil }
            capabilities.append(UnpeelUIProtocol.inputCapability)
        }
        guard case let .list(list) = body else { return nil }
        var hasToggle = false
        for item in list.items {
            for slot in [item.leading, item.trailing, item.accessory].compactMap({ $0 }) {
                guard case .toggle = slot else { return nil }
                hasToggle = true
            }
        }
        if hasToggle { capabilities.append(UnpeelUIProtocol.toggleCapability) }
        return capabilities
    }
}

public enum UIComponent: Equatable, Sendable {
    case markdownEditor(MarkdownEditorSpec)
    case media(MediaSpec)
    case page(PageSpec)
    case unsupported(kind: String)

    public var kind: String {
        switch self {
        case .markdownEditor:
            "markdownEditor"
        case .media:
            "media"
        case .page:
            "page"
        case let .unsupported(kind):
            kind
        }
    }

    public var requiredCapability: String? {
        requiredCapabilities?.first
    }

    public var requiredCapabilities: [String]? {
        switch self {
        case .markdownEditor:
            [UnpeelUIProtocol.markdownEditorCapability]
        case .media:
            [UnpeelUIProtocol.mediaCapability]
        case let .page(page):
            page.requiredCapabilities
        case .unsupported:
            nil
        }
    }
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

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        let kind = try container.decode(String.self, forKey: .type)
        switch kind {
        case "markdownEditor":
            component = .markdownEditor(try MarkdownEditorSpec(from: decoder))
        case "media":
            component = .media(try MediaSpec(from: decoder))
        case "page":
            component = .page(try PageSpec(from: decoder))
        default:
            component = .unsupported(kind: kind)
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(id, forKey: .id)
        switch component {
        case let .markdownEditor(editor):
            try container.encode("markdownEditor", forKey: .type)
            try editor.encode(to: encoder)
        case let .media(media):
            try container.encode("media", forKey: .type)
            try media.encode(to: encoder)
        case let .page(page):
            try container.encode("page", forKey: .type)
            try page.encode(to: encoder)
        case let .unsupported(kind):
            try container.encode(kind, forKey: .type)
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
        protocolVersion: Int = UnpeelUIProtocol.version,
        appInstanceID: String,
        clientID: String,
        viewID: String,
        revision: Int,
        root: UINode
    ) {
        protocolName = UnpeelUIProtocol.name
        self.protocolVersion = protocolVersion
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
    case delta(UIDelta)
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
        case delta
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
        case .delta:
            self = .delta(try UIDelta(from: decoder))
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
        case let .delta(message):
            try container.encode(MessageType.delta, forKey: .type)
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
        case let .delta(message):
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
