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
            PageContent(page: page, onAction: onAction)
        case .markdownEditor, .media, .unsupported:
            EmptyView()
        }
    }
}

@MainActor
private struct PageContent: View {
    let page: PageSpec
    let onAction: (UIAction) -> Void
    @State private var draft = ""

    init(page: PageSpec, onAction: @escaping (UIAction) -> Void) {
        self.page = page
        self.onAction = onAction
        if case let .input(input) = page.header {
            _draft = State(initialValue: input.value)
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(page.title)
                .font(.title2.weight(.semibold))
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
            TextField(input.placeholder, text: value)
                .accessibilityLabel(input.label)
                .onSubmit { submit(input) }
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
            Text(item.label)
                .strikethrough(item.done)
                .foregroundStyle(item.done ? .secondary : .primary)
            Spacer(minLength: 12)
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
