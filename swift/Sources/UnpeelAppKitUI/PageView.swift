import AppKit
import SwiftUI

/// Native SwiftUI interpretation of the closed Page component family.
///
/// Page owns named header/body slots, List owns only ListItem rows, and each
/// row slot accepts only controls enumerated by `UIListItemSlot`.
@MainActor
public struct PageView: View {
    public let snapshot: UISnapshot
    public let onAction: (UIAction) -> Void

    public init(snapshot: UISnapshot, onAction: @escaping (UIAction) -> Void) {
        self.snapshot = snapshot
        self.onAction = onAction
    }

    public var body: some View {
        switch snapshot.root.component {
        case let .page(page):
            PageContent(nodeID: snapshot.root.id, page: page, onAction: onAction)
        case .canvasPage, .markdownEditor, .media, .surface, .unsupported:
            EmptyView()
        }
    }
}

@MainActor
private struct PageContent: View {
    let nodeID: String
    let page: PageSpec
    let onAction: (UIAction) -> Void
    @State private var draft = ""

    init(nodeID: String, page: PageSpec, onAction: @escaping (UIAction) -> Void) {
        self.nodeID = nodeID
        self.page = page
        self.onAction = onAction
        if case let .input(input) = page.header {
            _draft = State(initialValue: input.value)
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                if let back = page.back {
                    Button {
                        onAction(UIAction(
                            nodeID: nodeID,
                            action: back,
                            kind: .cancel
                        ))
                    } label: {
                        Image(systemName: "chevron.left")
                    }
                    .buttonStyle(.borderless)
                    .accessibilityLabel("Back")
                }
                Text(page.title)
                    .font(.title2.weight(.semibold))
            }
                .padding(.horizontal)
                .padding(.top)
            if case let .input(input) = page.header {
                inputRow(input)
                    .padding()
                    .onChange(of: input.value) { _, value in
                        if draft != value { draft = value }
                    }
            }
            if case let .list(list) = page.body {
                if list.items.isEmpty {
                    ContentUnavailableView(
                        list.emptyMessage,
                        systemImage: "checklist"
                    )
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    List(list.items) { item in
                        itemRow(item)
                    }
                    .listStyle(.inset)
                }
            }
        }
    }

    private func inputRow(_ input: UIInputSpec) -> some View {
        let value = Binding(
            get: { draft },
            set: { value in
                draft = value
                guard let action = input.setValue else { return }
                onAction(UIAction(
                    nodeID: input.id,
                    action: action,
                    kind: .change,
                    value: .text(value)
                ))
            }
        )
        return HStack {
            StableInputField(
                text: value,
                placeholder: input.placeholder,
                onSubmit: { submit(input) }
            )
                .accessibilityLabel(input.label)
                .frame(minHeight: 24)
            if input.submit != nil {
                Button("Add") { submit(input) }
                    .disabled(draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
    }

    private func submit(_ input: UIInputSpec) {
        guard let action = input.submit else { return }
        onAction(UIAction(
            nodeID: input.id,
            action: action,
            kind: .submit,
            value: .text(draft)
        ))
        draft = ""
    }

    private func itemRow(_ item: UIListItemSpec) -> some View {
        HStack {
            slot(item.leading)
            if let activate = item.activate {
                Button {
                    onAction(UIAction(
                        nodeID: item.id,
                        action: activate,
                        kind: .activate
                    ))
                } label: {
                    itemLabel(item)
                }
                .buttonStyle(.plain)
            } else {
                itemLabel(item)
            }
            Spacer(minLength: 12)
            if let value = item.value {
                Text(value)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.trailing)
            }
            slot(item.trailing)
            slot(item.accessory)
            if let action = item.delete {
                Button {
                    onAction(UIAction(
                        nodeID: item.id,
                        action: action,
                        kind: .change
                    ))
                } label: {
                    Image(systemName: "trash")
                }
                .buttonStyle(.borderless)
                .accessibilityLabel("Delete \(item.label)")
            }
        }
    }

    private func itemLabel(_ item: UIListItemSpec) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(item.label)
                .strikethrough(item.done)
                .foregroundStyle(item.done ? .secondary : .primary)
            if let detail = item.detail {
                Text(detail)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .contentShape(Rectangle())
    }

    @ViewBuilder
    private func slot(_ slot: UIListItemSlot?) -> some View {
        if case let .toggle(toggle) = slot {
            Toggle(
                isOn: Binding(
                    get: { toggle.value },
                    set: { value in
                        onAction(UIAction(
                            nodeID: toggle.id,
                            action: toggle.setValue,
                            kind: .change,
                            value: .bool(value)
                        ))
                    }
                )
            ) {
                Text(toggle.label)
            }
            .labelsHidden()
            .accessibilityLabel(toggle.label)
            .toggleStyle(.switch)
        }
    }
}

/// An AppKit field whose editor survives unrelated snapshot/presence redraws.
/// SwiftUI's stock TextField can resign its field editor when a whole semantic
/// projection value is replaced, even though the Input node itself is stable.
@MainActor
private struct StableInputField: NSViewRepresentable {
    @Binding var text: String
    let placeholder: String
    let onSubmit: () -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(parent: self)
    }

    func makeNSView(context: Context) -> NSTextField {
        let field = NSTextField(string: text)
        field.delegate = context.coordinator
        field.placeholderString = placeholder
        field.isEditable = true
        field.isSelectable = true
        field.isBordered = true
        field.bezelStyle = .roundedBezel
        field.focusRingType = .default
        return field
    }

    func updateNSView(_ field: NSTextField, context: Context) {
        context.coordinator.parent = self
        field.placeholderString = placeholder
        // Never replace the active field editor underneath the user's caret.
        if field.currentEditor() == nil, field.stringValue != text {
            field.stringValue = text
        }
    }

    final class Coordinator: NSObject, NSTextFieldDelegate {
        var parent: StableInputField

        init(parent: StableInputField) {
            self.parent = parent
        }

        func controlTextDidChange(_ notification: Notification) {
            guard let field = notification.object as? NSTextField else { return }
            if parent.text != field.stringValue {
                parent.text = field.stringValue
            }
        }

        func controlTextDidEndEditing(_ notification: Notification) {
            guard let field = notification.object as? NSTextField else { return }
            if parent.text != field.stringValue {
                parent.text = field.stringValue
            }
        }

        func control(
            _ control: NSControl,
            textView _: NSTextView,
            doCommandBy commandSelector: Selector
        ) -> Bool {
            guard NSStringFromSelector(commandSelector) == "insertNewline:",
                  let field = control as? NSTextField
            else { return false }
            if parent.text != field.stringValue {
                parent.text = field.stringValue
            }
            parent.onSubmit()
            field.stringValue = parent.text
            return true
        }
    }
}
