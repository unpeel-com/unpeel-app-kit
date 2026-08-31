import Foundation

/// One ordered server-side mutation between immutable UI revisions.
public enum UIDeltaOperation: Equatable, Sendable {
    case replaceRoot(UINode)
    case markdownReplaceRange(nodeID: String, edit: UITextEdit)
    case markdownSetSelection(nodeID: String, selection: UITextSelection)
    case markdownSetPresentation(nodeID: String, presentation: MarkdownPresentation)
    case markdownSetDirty(nodeID: String, dirty: Bool)
    case markdownSetReadOnly(nodeID: String, readOnly: Bool)
    case markdownSetTitle(nodeID: String, title: String?)
    case markdownSetPlaceholder(nodeID: String, placeholder: String)
    case markdownSetActions(nodeID: String, actions: MarkdownEditorActions)
    case mediaSetSource(nodeID: String, source: MediaSource, intrinsic: MediaPixelSize)
    case surfaceSetReference(nodeID: String, reference: SurfaceReference)
    case toggleSetValue(nodeID: String, value: Bool)
    case inputSetValue(nodeID: String, value: String)
    case listInsertItem(listID: String, index: Int, item: UIListItemSpec)
    case listRemoveItem(listID: String, itemID: String)
}

extension UIDeltaOperation: Codable {
    enum CodingKeys: String, CodingKey {
        case op
        case root
        case nodeID = "nodeId"
        case edit
        case selection
        case presentation
        case dirty
        case readOnly
        case title
        case placeholder
        case actions
        case source
        case intrinsic
        case reference
        case value
        case listID = "listId"
        case index
        case item
        case itemID = "itemId"
    }

    enum Operation: String, Codable {
        case replaceRoot
        case markdownReplaceRange
        case markdownSetSelection
        case markdownSetPresentation
        case markdownSetDirty
        case markdownSetReadOnly
        case markdownSetTitle
        case markdownSetPlaceholder
        case markdownSetActions
        case mediaSetSource
        case surfaceSetReference
        case toggleSetValue
        case inputSetValue
        case listInsertItem
        case listRemoveItem
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Operation.self, forKey: .op) {
        case .replaceRoot:
            self = .replaceRoot(try container.decode(UINode.self, forKey: .root))
        case .markdownReplaceRange:
            self = .markdownReplaceRange(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                edit: try container.decode(UITextEdit.self, forKey: .edit)
            )
        case .markdownSetSelection:
            self = .markdownSetSelection(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                selection: try container.decode(UITextSelection.self, forKey: .selection)
            )
        case .markdownSetPresentation:
            self = .markdownSetPresentation(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                presentation: try container.decode(
                    MarkdownPresentation.self,
                    forKey: .presentation
                )
            )
        case .markdownSetDirty:
            self = .markdownSetDirty(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                dirty: try container.decode(Bool.self, forKey: .dirty)
            )
        case .markdownSetReadOnly:
            self = .markdownSetReadOnly(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                readOnly: try container.decode(Bool.self, forKey: .readOnly)
            )
        case .markdownSetTitle:
            self = .markdownSetTitle(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                title: try container.decodeIfPresent(String.self, forKey: .title)
            )
        case .markdownSetPlaceholder:
            self = .markdownSetPlaceholder(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                placeholder: try container.decode(String.self, forKey: .placeholder)
            )
        case .markdownSetActions:
            self = .markdownSetActions(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                actions: try container.decode(MarkdownEditorActions.self, forKey: .actions)
            )
        case .mediaSetSource:
            self = .mediaSetSource(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                source: try container.decode(MediaSource.self, forKey: .source),
                intrinsic: try container.decode(MediaPixelSize.self, forKey: .intrinsic)
            )
        case .surfaceSetReference:
            self = .surfaceSetReference(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                reference: try container.decode(SurfaceReference.self, forKey: .reference)
            )
        case .toggleSetValue:
            self = .toggleSetValue(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                value: try container.decode(Bool.self, forKey: .value)
            )
        case .inputSetValue:
            self = .inputSetValue(
                nodeID: try container.decode(String.self, forKey: .nodeID),
                value: try container.decode(String.self, forKey: .value)
            )
        case .listInsertItem:
            self = .listInsertItem(
                listID: try container.decode(String.self, forKey: .listID),
                index: try container.decode(Int.self, forKey: .index),
                item: try container.decode(UIListItemSpec.self, forKey: .item)
            )
        case .listRemoveItem:
            self = .listRemoveItem(
                listID: try container.decode(String.self, forKey: .listID),
                itemID: try container.decode(String.self, forKey: .itemID)
            )
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case let .replaceRoot(root):
            try container.encode(Operation.replaceRoot, forKey: .op)
            try container.encode(root, forKey: .root)
        case let .markdownReplaceRange(nodeID, edit):
            try container.encode(Operation.markdownReplaceRange, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(edit, forKey: .edit)
        case let .markdownSetSelection(nodeID, selection):
            try container.encode(Operation.markdownSetSelection, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(selection, forKey: .selection)
        case let .markdownSetPresentation(nodeID, presentation):
            try container.encode(Operation.markdownSetPresentation, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(presentation, forKey: .presentation)
        case let .markdownSetDirty(nodeID, dirty):
            try container.encode(Operation.markdownSetDirty, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(dirty, forKey: .dirty)
        case let .markdownSetReadOnly(nodeID, readOnly):
            try container.encode(Operation.markdownSetReadOnly, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(readOnly, forKey: .readOnly)
        case let .markdownSetTitle(nodeID, title):
            try container.encode(Operation.markdownSetTitle, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encodeIfPresent(title, forKey: .title)
            if title == nil {
                try container.encodeNil(forKey: .title)
            }
        case let .markdownSetPlaceholder(nodeID, placeholder):
            try container.encode(Operation.markdownSetPlaceholder, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(placeholder, forKey: .placeholder)
        case let .markdownSetActions(nodeID, actions):
            try container.encode(Operation.markdownSetActions, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(actions, forKey: .actions)
        case let .mediaSetSource(nodeID, source, intrinsic):
            try container.encode(Operation.mediaSetSource, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(source, forKey: .source)
            try container.encode(intrinsic, forKey: .intrinsic)
        case let .surfaceSetReference(nodeID, reference):
            try container.encode(Operation.surfaceSetReference, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(reference, forKey: .reference)
        case let .toggleSetValue(nodeID, value):
            try container.encode(Operation.toggleSetValue, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(value, forKey: .value)
        case let .inputSetValue(nodeID, value):
            try container.encode(Operation.inputSetValue, forKey: .op)
            try container.encode(nodeID, forKey: .nodeID)
            try container.encode(value, forKey: .value)
        case let .listInsertItem(listID, index, item):
            try container.encode(Operation.listInsertItem, forKey: .op)
            try container.encode(listID, forKey: .listID)
            try container.encode(index, forKey: .index)
            try container.encode(item, forKey: .item)
        case let .listRemoveItem(listID, itemID):
            try container.encode(Operation.listRemoveItem, forKey: .op)
            try container.encode(listID, forKey: .listID)
            try container.encode(itemID, forKey: .itemID)
        }
    }
}

/// Contiguous server-to-renderer change. A renderer applies it only when its
/// complete snapshot revision equals `baseRevision`.
public struct UIDelta: Codable, Equatable, Sendable {
    public let protocolName: String
    public let protocolVersion: Int
    public let appInstanceID: String
    public let clientID: String
    public let viewID: String
    public let baseRevision: Int
    public let revision: Int
    public let operations: [UIDeltaOperation]

    public init(
        protocolVersion: Int = UnpeelUIProtocol.version,
        appInstanceID: String,
        clientID: String,
        viewID: String,
        baseRevision: Int,
        revision: Int,
        operations: [UIDeltaOperation]
    ) {
        protocolName = UnpeelUIProtocol.name
        self.protocolVersion = protocolVersion
        self.appInstanceID = appInstanceID
        self.clientID = clientID
        self.viewID = viewID
        self.baseRevision = baseRevision
        self.revision = revision
        self.operations = operations
    }

    enum CodingKeys: String, CodingKey {
        case protocolName = "protocol"
        case protocolVersion
        case appInstanceID = "appInstanceId"
        case clientID = "clientId"
        case viewID = "viewId"
        case baseRevision
        case revision
        case operations
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        protocolName = try container.decode(String.self, forKey: .protocolName)
        protocolVersion = try container.decode(Int.self, forKey: .protocolVersion)
        appInstanceID = try container.decode(String.self, forKey: .appInstanceID)
        clientID = try container.decode(String.self, forKey: .clientID)
        viewID = try container.decode(String.self, forKey: .viewID)
        baseRevision = try container.decode(Int.self, forKey: .baseRevision)
        revision = try container.decode(Int.self, forKey: .revision)
        operations = try container.decode([UIDeltaOperation].self, forKey: .operations)
        guard baseRevision >= 0,
              revision > baseRevision,
              (1...4_096).contains(operations.count)
        else {
            throw DecodingError.dataCorruptedError(
                forKey: .revision,
                in: container,
                debugDescription: "Delta must advance its base with 1...4096 operations"
            )
        }
    }
}

public struct UIDeltaApplicationError: Error, Equatable, LocalizedError, Sendable {
    public let message: String

    public init(_ message: String) {
        self.message = message
    }

    public var errorDescription: String? { message }
}

public extension UISnapshot {
    /// Applies a route-matched contiguous delta and returns complete new state.
    func applying(_ delta: UIDelta) throws -> UISnapshot {
        guard protocolName == delta.protocolName,
              protocolVersion == delta.protocolVersion,
              appInstanceID == delta.appInstanceID,
              clientID == delta.clientID,
              viewID == delta.viewID
        else {
            throw UIDeltaApplicationError("Delta route does not match the current snapshot")
        }
        guard revision == delta.baseRevision, delta.revision > delta.baseRevision else {
            throw UIDeltaApplicationError("Delta is not contiguous with the current snapshot")
        }
        guard (1...4_096).contains(delta.operations.count) else {
            throw UIDeltaApplicationError("Delta must contain 1...4096 operations")
        }

        var root = root
        for operation in delta.operations {
            root = try root.applying(operation)
        }
        if case let .markdownEditor(editor) = root.component {
            _ = try utf16Offset(for: editor.selection.anchor, in: editor.text)
            _ = try utf16Offset(for: editor.selection.head, in: editor.text)
        }
        return UISnapshot(
            protocolVersion: delta.protocolVersion,
            appInstanceID: delta.appInstanceID,
            clientID: delta.clientID,
            viewID: delta.viewID,
            revision: delta.revision,
            root: root
        )
    }
}

private extension UINode {
    func applying(_ operation: UIDeltaOperation) throws -> UINode {
        switch operation {
        case let .replaceRoot(root):
            return root
        case let .markdownReplaceRange(nodeID, edit):
            var editor = try markdownEditor(nodeID: nodeID)
            editor = editor.copying(text: try replacing(edit, in: editor.text))
            return UINode(id: id, component: .markdownEditor(editor))
        case let .markdownSetSelection(nodeID, selection):
            let editor = try markdownEditor(nodeID: nodeID).copying(selection: selection)
            return UINode(id: id, component: .markdownEditor(editor))
        case let .markdownSetPresentation(nodeID, presentation):
            let editor = try markdownEditor(nodeID: nodeID).copying(presentation: presentation)
            return UINode(id: id, component: .markdownEditor(editor))
        case let .markdownSetDirty(nodeID, dirty):
            let editor = try markdownEditor(nodeID: nodeID).copying(dirty: dirty)
            return UINode(id: id, component: .markdownEditor(editor))
        case let .markdownSetReadOnly(nodeID, readOnly):
            let editor = try markdownEditor(nodeID: nodeID).copying(readOnly: readOnly)
            return UINode(id: id, component: .markdownEditor(editor))
        case let .markdownSetTitle(nodeID, title):
            let editor = try markdownEditor(nodeID: nodeID).copying(title: .set(title))
            return UINode(id: id, component: .markdownEditor(editor))
        case let .markdownSetPlaceholder(nodeID, placeholder):
            let editor = try markdownEditor(nodeID: nodeID).copying(placeholder: placeholder)
            return UINode(id: id, component: .markdownEditor(editor))
        case let .markdownSetActions(nodeID, actions):
            let editor = try markdownEditor(nodeID: nodeID).copying(actions: actions)
            return UINode(id: id, component: .markdownEditor(editor))
        case let .mediaSetSource(nodeID, source, intrinsic):
            let media = try media(nodeID: nodeID).copying(
                source: source,
                intrinsic: intrinsic
            )
            return UINode(id: id, component: .media(media))
        case let .surfaceSetReference(nodeID, reference):
            switch component {
            case let .surface(surface) where id == nodeID:
                return UINode(id: id, component: .surface(surface.copying(
                    reference: reference
                )))
            case let .canvasPage(page) where page.surface.id == nodeID:
                let nested = page.surface.surface.copying(reference: reference)
                return UINode(id: id, component: .canvasPage(CanvasPageSpec(
                    title: page.title,
                    surface: UICanvasSurfaceSpec(id: page.surface.id, surface: nested),
                    controls: page.controls
                )))
            default:
                throw UIDeltaApplicationError("Delta targets an unavailable Surface node")
            }
        case let .toggleSetValue(nodeID, value):
            var page = try page()
            guard case var .list(list) = page.body else {
                throw UIDeltaApplicationError("Delta targets an unavailable List")
            }
            var found = false
            for index in list.items.indices {
                var item = list.items[index]
                var itemFound = false
                item.leading = setToggle(item.leading, id: nodeID, value: value, found: &itemFound)
                item.trailing = setToggle(item.trailing, id: nodeID, value: value, found: &itemFound)
                item.accessory = setToggle(item.accessory, id: nodeID, value: value, found: &itemFound)
                if itemFound {
                    item.done = value
                    list.items[index] = item
                    found = true
                    break
                }
            }
            guard found else {
                throw UIDeltaApplicationError("Delta targets an unavailable Toggle")
            }
            page.body = .list(list)
            return UINode(id: id, component: .page(page))
        case let .inputSetValue(nodeID, value):
            var page = try page()
            guard case var .input(input) = page.header, input.id == nodeID else {
                throw UIDeltaApplicationError("Delta targets an unavailable Input")
            }
            input.value = value
            page.header = .input(input)
            return UINode(id: id, component: .page(page))
        case let .listInsertItem(listID, index, item):
            var page = try page()
            guard case var .list(list) = page.body,
                  list.id == listID,
                  index >= 0,
                  index <= list.items.count
            else {
                throw UIDeltaApplicationError("Delta targets an unavailable List insertion")
            }
            list.items.insert(item, at: index)
            page.body = .list(list)
            return UINode(id: id, component: .page(page))
        case let .listRemoveItem(listID, itemID):
            var page = try page()
            guard case var .list(list) = page.body,
                  list.id == listID,
                  let index = list.items.firstIndex(where: { $0.id == itemID })
            else {
                throw UIDeltaApplicationError("Delta targets an unavailable ListItem")
            }
            list.items.remove(at: index)
            page.body = .list(list)
            return UINode(id: id, component: .page(page))
        }
    }

    func markdownEditor(nodeID: String) throws -> MarkdownEditorSpec {
        guard id == nodeID, case let .markdownEditor(editor) = component else {
            throw UIDeltaApplicationError("Delta targets an unavailable Markdown node")
        }
        return editor
    }

    func media(nodeID: String) throws -> MediaSpec {
        guard id == nodeID, case let .media(media) = component else {
            throw UIDeltaApplicationError("Delta targets an unavailable Media node")
        }
        return media
    }

    func page() throws -> PageSpec {
        guard case let .page(page) = component else {
            throw UIDeltaApplicationError("Delta targets an unavailable Page")
        }
        return page
    }
}

private func setToggle(
    _ slot: UIListItemSlot?,
    id: String,
    value: Bool,
    found: inout Bool
) -> UIListItemSlot? {
    guard case let .toggle(toggle) = slot, toggle.id == id else { return slot }
    found = true
    return .toggle(UIToggleSpec(
        id: toggle.id,
        label: toggle.label,
        value: value,
        setValue: toggle.setValue
    ))
}

private enum OptionalStringChange {
    case unchanged
    case set(String?)
}

private extension MarkdownEditorSpec {
    func copying(
        text: String? = nil,
        selection: UITextSelection? = nil,
        presentation: MarkdownPresentation? = nil,
        readOnly: Bool? = nil,
        dirty: Bool? = nil,
        placeholder: String? = nil,
        title: OptionalStringChange = .unchanged,
        actions: MarkdownEditorActions? = nil
    ) -> Self {
        let nextTitle: String?
        switch title {
        case .unchanged:
            nextTitle = self.title
        case let .set(value):
            nextTitle = value
        }
        return Self(
            text: text ?? self.text,
            selection: selection ?? self.selection,
            presentation: presentation ?? self.presentation,
            readOnly: readOnly ?? self.readOnly,
            dirty: dirty ?? self.dirty,
            placeholder: placeholder ?? self.placeholder,
            title: nextTitle,
            actions: actions ?? self.actions
        )
    }
}

private extension MediaSpec {
    func copying(
        source: MediaSource? = nil,
        intrinsic: MediaPixelSize? = nil
    ) -> Self {
        Self(
            source: source ?? self.source,
            intrinsic: intrinsic ?? self.intrinsic,
            cells: cells,
            points: points,
            fit: fit,
            alt: alt,
            activate: activate
        )
    }
}

private extension SurfaceSpec {
    func copying(reference: SurfaceReference) -> Self {
        Self(
            reference: reference,
            cells: cells,
            points: points,
            background: background,
            inputPolicy: inputPolicy
        )
    }
}

private func replacing(_ edit: UITextEdit, in text: String) throws -> String {
    guard edit.range.start <= edit.range.end else {
        throw UIDeltaApplicationError("Markdown text edit range is reversed")
    }
    let start = try utf16Offset(for: edit.range.start, in: text)
    let end = try utf16Offset(for: edit.range.end, in: text)
    let mutable = NSMutableString(string: text)
    mutable.replaceCharacters(in: NSRange(location: start, length: end - start), with: edit.text)
    return mutable as String
}

private func utf16Offset(for position: UITextPosition, in text: String) throws -> Int {
    guard position.line >= 0, position.utf16Column >= 0 else {
        throw UIDeltaApplicationError("Negative Markdown text position")
    }
    let units = text.utf16
    var lineStart = units.startIndex
    for _ in 0..<position.line {
        guard let newline = units[lineStart...].firstIndex(of: 10) else {
            throw UIDeltaApplicationError("Markdown text line is outside the document")
        }
        lineStart = units.index(after: newline)
    }
    let lineEnd = units[lineStart...].firstIndex(of: 10) ?? units.endIndex
    guard let target = units.index(
        lineStart,
        offsetBy: position.utf16Column,
        limitedBy: lineEnd
    ), target <= lineEnd, String.Index(target, within: text) != nil else {
        throw UIDeltaApplicationError("Markdown UTF-16 column is outside a scalar boundary")
    }
    return units.distance(from: units.startIndex, to: target)
}
