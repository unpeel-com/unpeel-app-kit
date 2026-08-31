import AppKit
import SwiftUI

/// Native renderer for App Kit's first opinionated component.
///
/// The terminal-backed Rust App remains authoritative. This view applies its
/// latest snapshot and returns range edits, selections, save commands, and
/// presentation changes through `onAction`. The session transport adds the
/// authenticated participant, client, renderer, event, and revision envelope.
@MainActor
public struct MarkdownEditorView: View {
    public let snapshot: UISnapshot
    public let onAction: (UIAction) -> Void

    public init(snapshot: UISnapshot, onAction: @escaping (UIAction) -> Void) {
        self.snapshot = snapshot
        self.onAction = onAction
    }

    public var body: some View {
        switch snapshot.root.component {
        case let .markdownEditor(editor):
            VStack(spacing: 0) {
                toolbar(editor)
                Divider()
                editorContent(editor)
            }
        }
    }

    private func toolbar(_ editor: MarkdownEditorSpec) -> some View {
        HStack(spacing: 10) {
            Text(editor.title ?? "Markdown")
                .font(.system(size: 12, weight: .medium))
                .lineLimit(1)
            if editor.dirty {
                Circle()
                    .fill(.secondary)
                    .frame(width: 5, height: 5)
                    .accessibilityLabel("Unsaved changes")
            }
            Spacer(minLength: 8)
            if let action = editor.actions.setPresentation {
                Picker(
                    "Presentation",
                    selection: Binding(
                        get: { editor.presentation },
                        set: { presentation in
                            onAction(UIAction(
                                nodeID: snapshot.root.id,
                                action: action,
                                kind: .change,
                                value: .text(presentation.rawValue)
                            ))
                        }
                    )
                ) {
                    Text("Source").tag(MarkdownPresentation.source)
                    Text("Preview").tag(MarkdownPresentation.preview)
                    Text("Split").tag(MarkdownPresentation.split)
                }
                .labelsHidden()
                .pickerStyle(.segmented)
                .frame(width: 190)
            }
            if let action = editor.actions.save, !editor.readOnly {
                Button("Save") {
                    onAction(UIAction(
                        nodeID: snapshot.root.id,
                        action: action,
                        kind: .command
                    ))
                }
                .keyboardShortcut("s", modifiers: .command)
            }
        }
        .padding(.horizontal, 10)
        .frame(height: 38)
    }

    @ViewBuilder
    private func editorContent(_ editor: MarkdownEditorSpec) -> some View {
        switch editor.presentation {
        case .source:
            sourceEditor(editor)
        case .preview:
            MarkdownPreview(text: editor.text)
        case .split:
            HSplitView {
                sourceEditor(editor)
                MarkdownPreview(text: editor.text)
            }
        }
    }

    private func sourceEditor(_ editor: MarkdownEditorSpec) -> some View {
        ZStack(alignment: .topLeading) {
            MarkdownTextView(
                nodeID: snapshot.root.id,
                editor: editor,
                onAction: onAction
            )
            if editor.text.isEmpty, !editor.placeholder.isEmpty {
                Text(editor.placeholder)
                    .font(.system(.body, design: .monospaced))
                    .foregroundStyle(.tertiary)
                    .padding(.leading, 7)
                    .padding(.top, 8)
                    .allowsHitTesting(false)
            }
        }
    }
}

@MainActor
private struct MarkdownPreview: View {
    let text: String

    var body: some View {
        ScrollView {
            Text(rendered)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .topLeading)
                .padding(16)
        }
    }

    private var rendered: AttributedString {
        (try? AttributedString(markdown: text)) ?? AttributedString(text)
    }
}

@MainActor
private struct MarkdownTextView: NSViewRepresentable {
    let nodeID: String
    let editor: MarkdownEditorSpec
    let onAction: (UIAction) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(
            nodeID: nodeID,
            editor: editor,
            onAction: onAction
        )
    }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true
        scrollView.borderType = .noBorder
        scrollView.drawsBackground = false

        let textView = NSTextView()
        textView.delegate = context.coordinator
        textView.isRichText = false
        textView.importsGraphics = false
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticTextReplacementEnabled = false
        textView.isAutomaticSpellingCorrectionEnabled = false
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = false
        textView.autoresizingMask = [.width]
        textView.textContainer?.widthTracksTextView = true
        textView.textContainer?.containerSize = NSSize(
            width: 0,
            height: CGFloat.greatestFiniteMagnitude
        )
        textView.font = .monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)
        textView.drawsBackground = false
        textView.textContainerInset = NSSize(width: 5, height: 6)
        scrollView.documentView = textView
        context.coordinator.apply(editor, to: textView)
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let textView = scrollView.documentView as? NSTextView else { return }
        context.coordinator.update(
            nodeID: nodeID,
            editor: editor,
            onAction: onAction
        )
        context.coordinator.apply(editor, to: textView)
    }

    @MainActor
    final class Coordinator: NSObject, NSTextViewDelegate {
        private var nodeID: String
        private var editor: MarkdownEditorSpec
        private var onAction: (UIAction) -> Void
        private var applyingSnapshot = false
        private var suppressSelectionFromEdit = false

        init(
            nodeID: String,
            editor: MarkdownEditorSpec,
            onAction: @escaping (UIAction) -> Void
        ) {
            self.nodeID = nodeID
            self.editor = editor
            self.onAction = onAction
        }

        func update(
            nodeID: String,
            editor: MarkdownEditorSpec,
            onAction: @escaping (UIAction) -> Void
        ) {
            self.nodeID = nodeID
            self.editor = editor
            self.onAction = onAction
        }

        func apply(_ editor: MarkdownEditorSpec, to textView: NSTextView) {
            applyingSnapshot = true
            defer { applyingSnapshot = false }
            textView.isEditable = !editor.readOnly && editor.actions.replaceRange != nil
            textView.isSelectable = true
            if textView.string != editor.text {
                textView.string = editor.text
            }
            if let range = Self.nsRange(for: editor.selection, in: editor.text),
               textView.selectedRange() != range
            {
                textView.setSelectedRange(range)
            }
        }

        func textView(
            _ textView: NSTextView,
            shouldChangeTextIn affectedCharRange: NSRange,
            replacementString: String?
        ) -> Bool {
            guard !applyingSnapshot, !editor.readOnly,
                  let action = editor.actions.replaceRange,
                  let range = Self.textRange(for: affectedCharRange, in: textView.string)
            else { return false }

            suppressSelectionFromEdit = true
            onAction(UIAction(
                nodeID: nodeID,
                action: action,
                kind: .change,
                value: .textEdit(UITextEdit(
                    range: range,
                    text: replacementString ?? ""
                ))
            ))
            return true
        }

        func textViewDidChangeSelection(_ notification: Notification) {
            guard !applyingSnapshot,
                  let textView = notification.object as? NSTextView
            else { return }
            if suppressSelectionFromEdit {
                suppressSelectionFromEdit = false
                return
            }
            guard let action = editor.actions.setSelection,
                  let selection = Self.textSelection(
                      for: textView.selectedRange(),
                      in: textView.string
                  )
            else { return }
            onAction(UIAction(
                nodeID: nodeID,
                action: action,
                kind: .select,
                value: .textSelection(selection)
            ))
        }

        private static func textRange(for range: NSRange, in text: String) -> UITextRange? {
            guard range.location != NSNotFound,
                  range.location >= 0,
                  NSMaxRange(range) <= (text as NSString).length
            else { return nil }
            return UITextRange(
                start: position(atUTF16Offset: range.location, in: text),
                end: position(atUTF16Offset: NSMaxRange(range), in: text)
            )
        }

        private static func textSelection(
            for range: NSRange,
            in text: String
        ) -> UITextSelection? {
            guard let range = textRange(for: range, in: text) else { return nil }
            return UITextSelection(anchor: range.start, head: range.end)
        }

        private static func nsRange(for selection: UITextSelection, in text: String) -> NSRange? {
            guard let anchor = utf16Offset(for: selection.anchor, in: text),
                  let head = utf16Offset(for: selection.head, in: text)
            else { return nil }
            return NSRange(location: min(anchor, head), length: abs(head - anchor))
        }

        private static func position(atUTF16Offset target: Int, in text: String) -> UITextPosition {
            let utf16 = text.utf16
            let clamped = max(0, min(target, utf16.count))
            var line = 0
            var lineStart = 0
            for (offset, unit) in utf16.prefix(clamped).enumerated() where unit == 10 {
                line += 1
                lineStart = offset + 1
            }
            return UITextPosition(line: line, utf16Column: clamped - lineStart)
        }

        private static func utf16Offset(for position: UITextPosition, in text: String) -> Int? {
            guard position.line >= 0, position.utf16Column >= 0 else { return nil }
            var line = 0
            var lineStart = 0
            let utf16 = Array(text.utf16)
            if position.line > 0 {
                for (offset, unit) in utf16.enumerated() where unit == 10 {
                    line += 1
                    lineStart = offset + 1
                    if line == position.line { break }
                }
                guard line == position.line else { return nil }
            }
            let lineEnd = utf16[lineStart...].firstIndex(of: 10) ?? utf16.endIndex
            let target = lineStart + position.utf16Column
            guard target <= lineEnd else { return nil }
            return target
        }
    }
}
