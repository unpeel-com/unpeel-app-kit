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
                .id(snapshot.root.id)
                .transition(.asymmetric(
                    insertion: .move(edge: .trailing).combined(with: .opacity),
                    removal: .move(edge: .leading).combined(with: .opacity)
                ))
                .animation(.snappy, value: snapshot.root.id)
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
    @State private var selectedID: String?
    @State private var listHeight: CGFloat = 0
    @FocusState private var listFocused: Bool

    init(nodeID: String, page: PageSpec, onAction: @escaping (UIAction) -> Void) {
        self.nodeID = nodeID
        self.page = page
        self.onAction = onAction
        if case let .input(input) = page.header {
            _draft = State(initialValue: input.value)
        }
        if case let .list(list) = page.body {
            _selectedID = State(initialValue: list.selectedID)
        } else {
            _selectedID = State(initialValue: nil)
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
                    ScrollViewReader { proxy in
                        List(list.items, selection: selectionBinding(list)) { item in
                            itemRow(item, list: list)
                                .tag(item.id)
                                .id(item.id)
                        }
                        .listStyle(.inset)
                        .focusable()
                        .focused($listFocused)
                        .onKeyPress(phases: [.down, .repeat]) { press in
                            handleKeyPress(press, list: list, proxy: proxy)
                        }
                        .background {
                            GeometryReader { geometry in
                                Color.clear
                                    .onAppear { listHeight = geometry.size.height }
                                    .onChange(of: geometry.size.height) { _, height in
                                        listHeight = height
                                    }
                            }
                        }
                        .onChange(of: list.selectedID) { _, selected in
                            guard selectedID != selected else { return }
                            selectedID = selected
                            if let selected {
                                proxy.scrollTo(selected, anchor: .center)
                            }
                        }
                    }
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

    private func selectionBinding(_ list: UIListSpec) -> Binding<String?> {
        Binding(
            get: { selectedID },
            set: { itemID in
                guard let itemID else { return }
                select(itemID, in: list)
            }
        )
    }

    private func select(_ itemID: String, in list: UIListSpec) {
        guard list.items.contains(where: { $0.id == itemID }) else { return }
        let changed = selectedID != itemID
        selectedID = itemID
        guard changed, let action = list.select else { return }
        onAction(UIAction(
            nodeID: list.id,
            action: action,
            kind: .change,
            value: .text(itemID)
        ))
    }

    private func handleKeyPress(
        _ press: KeyPress,
        list: UIListSpec,
        proxy: ScrollViewProxy
    ) -> KeyPress.Result {
        guard press.modifiers.intersection([.command, .control, .option]).isEmpty else {
            return .ignored
        }
        guard !list.items.isEmpty else { return .ignored }
        let current = list.items.firstIndex(where: { $0.id == selectedID })
            ?? list.items.firstIndex(where: { $0.id == list.selectedID })
            ?? 0
        guard let key = navigationKey(press),
              let decision = uiListNavigationDecision(
                key: key,
                primaryRole: list.items[current].primaryRole
              )
        else { return .ignored }
        if decision == .back {
            guard let back = page.back else { return .ignored }
            onAction(UIAction(nodeID: nodeID, action: back, kind: .cancel))
            return .handled
        }
        if decision == .invokePrimary {
            return invokePrimary(list.items[current], in: list) ? .handled : .ignored
        }
        if list.pageBehavior == .scroll, decision == .pageDown || decision == .pageUp {
            return .ignored
        }

        let visibleRows = max(Int(listHeight / 28), 1)
        let pageRows = max(visibleRows - max(list.pageOverlap, 0), 1)
        let last = list.items.count - 1
        let next: Int
        switch decision {
        case .down:
            next = min(current + 1, last)
        case .up:
            next = max(current - 1, 0)
        case .first:
            next = 0
        case .last:
            next = last
        case .pageDown:
            next = min(current + pageRows, last)
        case .pageUp:
            next = max(current - pageRows, 0)
        case .invokePrimary, .back:
            return .ignored
        }
        select(list.items[next].id, in: list)
        proxy.scrollTo(list.items[next].id, anchor: list.scrollPadding > 0 ? .center : nil)
        return .handled
    }

    private func navigationKey(_ press: KeyPress) -> UIListNavigationKey? {
        switch (press.key, press.characters) {
        case (.downArrow, _), (_, "j"): .down
        case (.upArrow, _), (_, "k"): .up
        case (.home, _), (_, "g"): .first
        case (.end, _), (_, "G"): .last
        case (.pageDown, _): .pageDown
        case (.pageUp, _): .pageUp
        case (.return, _): .enter
        case (.space, _): .space
        case (.escape, _), (_, "q"): .back
        default: nil
        }
    }

    @discardableResult
    private func invokePrimary(_ item: UIListItemSpec, in list: UIListSpec) -> Bool {
        select(item.id, in: list)
        switch item.primaryRole {
        case .toggle:
            guard let toggle = item.primaryToggle else { return false }
            onAction(UIAction(
                nodeID: toggle.id,
                action: toggle.setValue,
                kind: .change,
                value: .bool(!toggle.value)
            ))
        case .checkmark:
            guard let checkmark = item.primaryCheckmark else { return false }
            onAction(UIAction(
                nodeID: checkmark.id,
                action: checkmark.setValue,
                kind: .change,
                value: .bool(!checkmark.value)
            ))
        case .disclosure:
            guard let activate = item.activate else { return false }
            withAnimation(.snappy) {
                onAction(UIAction(nodeID: item.id, action: activate, kind: .activate))
            }
        case .command, .destructive:
            guard let activate = item.activate else { return false }
            onAction(UIAction(nodeID: item.id, action: activate, kind: .activate))
        case .static:
            return false
        }
        return true
    }

    private func color(for tone: UIListItemTone) -> Color {
        switch tone {
        case .default: .primary
        case .muted: .secondary
        case .accent: .accentColor
        case .info: .blue
        case .success: .green
        case .warning: .orange
        case .danger: .red
        }
    }

    private func itemRow(_ item: UIListItemSpec, list: UIListSpec) -> some View {
        Group {
            if let value = item.value {
                ViewThatFits(in: .horizontal) {
                    itemRowContent(item, list: list, value: value)
                        .frame(minWidth: CGFloat(item.valueMinWidth ?? value.count + 11) * 8)
                    itemRowContent(item, list: list, value: nil)
                }
            } else {
                itemRowContent(item, list: list, value: nil)
            }
        }
    }

    private func itemRowContent(
        _ item: UIListItemSpec,
        list: UIListSpec,
        value: String?
    ) -> some View {
        HStack {
            if item.busy {
                ProgressView()
                    .controlSize(.small)
                    .accessibilityLabel("Loading")
            }
            slot(item.leading, itemID: item.id, list: list)
            if item.primaryRole == .static {
                itemLabel(item)
                    .frame(maxWidth: .infinity, alignment: .leading)
            } else {
                Button {
                    listFocused = true
                    _ = invokePrimary(item, in: list)
                } label: {
                    itemLabel(item)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
            Spacer(minLength: 4)
            if let value {
                Text(value)
                    .foregroundStyle(color(for: item.valueTone))
                    .multilineTextAlignment(.trailing)
                    .fixedSize(horizontal: true, vertical: false)
            }
            slot(item.trailing, itemID: item.id, list: list)
            slot(item.accessory, itemID: item.id, list: list)
            if let action = item.delete {
                Button {
                    select(item.id, in: list)
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
                .fontWeight(item.emphasis == .strong ? .semibold : .regular)
                .foregroundStyle(
                    item.actionRole == .destructive
                        ? Color.red
                        : (item.done ? Color.secondary : color(for: item.labelTone))
                )
            if let detail = item.detail {
                Text(detail)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .contentShape(Rectangle())
    }

    @ViewBuilder
    private func slot(
        _ slot: UIListItemSlot?,
        itemID: String,
        list: UIListSpec
    ) -> some View {
        switch slot {
        case let .toggle(toggle):
            Toggle(
                isOn: Binding(
                    get: { toggle.value },
                    set: { value in
                        select(itemID, in: list)
                        listFocused = true
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
            .toggleStyle(.checkbox)
        case let .status(status):
            Text(status.symbol)
                .foregroundStyle(color(for: status.tone))
                .fontWeight(status.emphasis == .strong ? .semibold : .regular)
                .accessibilityLabel(status.label)
        case let .badge(badge):
            Text(badge.text)
                .font(.caption.weight(.medium))
                .foregroundStyle(color(for: badge.tone))
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(.quaternary, in: Capsule())
        case .disclosure:
            Image(systemName: "chevron.right")
                .foregroundStyle(.tertiary)
                .accessibilityHidden(true)
        case let .checkmark(checkmark):
            Image(systemName: "checkmark")
                .foregroundStyle(Color.accentColor)
                .opacity(checkmark.value ? 1 : 0)
                .accessibilityLabel(checkmark.label)
                .accessibilityValue(checkmark.value ? "Selected" : "Not selected")
        case .unsupported, .none:
            EmptyView()
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
