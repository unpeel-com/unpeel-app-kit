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
        case .media, .page, .unsupported:
            EmptyView()
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
                revision: snapshot.revision,
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
    let revision: Int
    let editor: MarkdownEditorSpec
    let onAction: (UIAction) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(
            nodeID: nodeID,
            revision: revision,
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
        context.coordinator.attach(textView)
        context.coordinator.apply(editor, revision: revision, to: textView)
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let textView = scrollView.documentView as? NSTextView else { return }
        context.coordinator.update(
            nodeID: nodeID,
            revision: revision,
            editor: editor,
            onAction: onAction
        )
        context.coordinator.apply(editor, revision: revision, to: textView)
    }

    @MainActor
    final class Coordinator: NSObject, NSTextViewDelegate {
        private var nodeID: String
        private var editor: MarkdownEditorSpec
        private var onAction: (UIAction) -> Void
        private var applyingSnapshot = false
        private weak var textView: NSTextView?
        private var hasAppliedSnapshot = false
        private var authoritativeRevision: Int
        private var authoritativeText: String
        private var inFlightText: String?
        private var flushWorkItem: DispatchWorkItem?
        private var insertPopover: NSPopover?
        private var insertContext: MarkdownSlashContext?
        private var visibleInsertItems: [MarkdownInsertItem] = []
        private var selectedInsertIndex = 0

        init(
            nodeID: String,
            revision: Int,
            editor: MarkdownEditorSpec,
            onAction: @escaping (UIAction) -> Void
        ) {
            self.nodeID = nodeID
            self.editor = editor
            self.onAction = onAction
            authoritativeRevision = revision
            authoritativeText = editor.text
        }

        func attach(_ textView: NSTextView) {
            self.textView = textView
        }

        func update(
            nodeID: String,
            revision _: Int,
            editor: MarkdownEditorSpec,
            onAction: @escaping (UIAction) -> Void
        ) {
            self.nodeID = nodeID
            self.editor = editor
            self.onAction = onAction
        }

        func apply(
            _ editor: MarkdownEditorSpec,
            revision: Int,
            to textView: NSTextView
        ) {
            applyingSnapshot = true
            defer {
                applyingSnapshot = false
                refreshInsertMenu(in: textView)
            }
            textView.isEditable = !editor.readOnly && editor.actions.replaceRange != nil
            textView.isSelectable = true
            var shouldApplyAuthoritativeSelection = false

            if !hasAppliedSnapshot {
                hasAppliedSnapshot = true
                authoritativeRevision = revision
                authoritativeText = editor.text
                textView.string = editor.text
                shouldApplyAuthoritativeSelection = true
            } else {
                let previousRevision = authoritativeRevision
                let previousAuthority = authoritativeText
                let localText = textView.string
                let hadLocalChanges = localText != previousAuthority
                let incomingChanged = revision != previousRevision
                    || editor.text != previousAuthority

                if incomingChanged {
                    authoritativeRevision = revision
                    authoritativeText = editor.text
                    if inFlightText != nil,
                       revision > previousRevision || editor.text == inFlightText
                    {
                        inFlightText = nil
                    }
                    if !hadLocalChanges || localText == editor.text {
                        if textView.string != editor.text {
                            textView.string = editor.text
                        }
                        shouldApplyAuthoritativeSelection = !hadLocalChanges
                            && editor.text != previousAuthority
                    }
                }
            }

            if shouldApplyAuthoritativeSelection,
               let range = Self.nsRange(for: editor.selection, in: editor.text),
               textView.selectedRange() != range
            {
                textView.setSelectedRange(range)
            }
            if textView.string != authoritativeText, inFlightText == nil {
                scheduleFlush(from: textView)
            }
        }

        func textView(
            _ textView: NSTextView,
            shouldChangeTextIn affectedCharRange: NSRange,
            replacementString: String?
        ) -> Bool {
            guard !applyingSnapshot, !editor.readOnly,
                  editor.actions.replaceRange != nil,
                  affectedCharRange.location != NSNotFound,
                  NSMaxRange(affectedCharRange) <= (textView.string as NSString).length
            else { return false }

            return true
        }

        func textDidChange(_ notification: Notification) {
            guard !applyingSnapshot,
                  let textView = notification.object as? NSTextView
            else { return }
            scheduleFlush(from: textView)
            refreshInsertMenu(in: textView)
        }

        func textViewDidChangeSelection(_ notification: Notification) {
            guard !applyingSnapshot,
                  let textView = notification.object as? NSTextView
            else { return }
            refreshInsertMenu(in: textView)
            guard textView.string == authoritativeText, inFlightText == nil else { return }
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

        func textView(
            _ textView: NSTextView,
            doCommandBy commandSelector: Selector
        ) -> Bool {
            let command = NSStringFromSelector(commandSelector)
            if insertContext != nil {
                switch command {
                case "moveUp:":
                    moveInsertSelection(-1, in: textView)
                    return true
                case "moveDown:":
                    moveInsertSelection(1, in: textView)
                    return true
                case "insertNewline:", "insertNewlineIgnoringFieldEditor:", "insertTab:":
                    applySelectedInsert(in: textView)
                    return true
                case "cancelOperation:":
                    clearSlashCommand(in: textView)
                    return true
                default:
                    break
                }
            }
            if command == "deleteBackward:",
               let edit = markdownBackspaceEdit(
                   text: textView.string,
                   selection: textView.selectedRange()
               )
            {
                replaceText(
                    in: textView,
                    range: edit.lineRange,
                    with: edit.replacement,
                    caretUTF16Offset: edit.caretUTF16Offset
                )
                return true
            }
            return false
        }

        private func scheduleFlush(from textView: NSTextView) {
            flushWorkItem?.cancel()
            flushWorkItem = nil
            guard inFlightText == nil, textView.string != authoritativeText else { return }
            let work = DispatchWorkItem { [weak self, weak textView] in
                guard let self, let textView else { return }
                self.flushWorkItem = nil
                self.flushLocalEdit(from: textView)
            }
            flushWorkItem = work
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.09, execute: work)
        }

        private func flushLocalEdit(from textView: NSTextView) {
            guard !applyingSnapshot, inFlightText == nil, !editor.readOnly,
                  let action = editor.actions.replaceRange,
                  let edit = Self.diffText(before: authoritativeText, after: textView.string)
            else { return }
            inFlightText = textView.string
            onAction(UIAction(
                nodeID: nodeID,
                action: action,
                kind: .change,
                value: .textEdit(edit)
            ))
        }

        private func refreshInsertMenu(in textView: NSTextView) {
            guard !editor.readOnly,
                  let context = markdownSlashContext(
                      text: textView.string,
                      selection: textView.selectedRange()
                  )
            else {
                closeInsertMenu()
                return
            }
            insertContext = context
            visibleInsertItems = visibleMarkdownInsertItems(query: context.query)
            selectedInsertIndex = min(
                selectedInsertIndex,
                max(0, visibleInsertItems.count - 1)
            )
            showInsertPopover(relativeTo: textView)
        }

        private func showInsertPopover(relativeTo textView: NSTextView) {
            let popover: NSPopover
            if let insertPopover {
                popover = insertPopover
            } else {
                popover = NSPopover()
                popover.behavior = .applicationDefined
                popover.animates = false
                insertPopover = popover
            }
            let menu = MarkdownInsertMenuView(
                items: visibleInsertItems,
                selectedIndex: selectedInsertIndex,
                onSelect: { [weak self, weak textView] kind in
                    guard let self, let textView else { return }
                    self.applyInsert(kind, in: textView)
                }
            )
            popover.contentViewController = NSHostingController(rootView: menu)
            popover.contentSize = NSSize(
                width: 270,
                height: min(330, CGFloat(max(1, visibleInsertItems.count) * 30 + 12))
            )
            guard !popover.isShown, let window = textView.window else { return }
            let screenRect = textView.firstRect(
                forCharacterRange: textView.selectedRange(),
                actualRange: nil
            )
            let localRect = textView.convert(window.convertFromScreen(screenRect), from: nil)
            let anchor = NSRect(
                x: localRect.minX,
                y: localRect.minY,
                width: max(1, localRect.width),
                height: max(16, localRect.height)
            )
            let wasFirstResponder = window.firstResponder === textView
            popover.show(relativeTo: anchor, of: textView, preferredEdge: .maxY)
            if wasFirstResponder {
                window.makeFirstResponder(textView)
            }
        }

        private func moveInsertSelection(_ delta: Int, in textView: NSTextView) {
            guard !visibleInsertItems.isEmpty else { return }
            selectedInsertIndex = (
                selectedInsertIndex + delta + visibleInsertItems.count
            ) % visibleInsertItems.count
            showInsertPopover(relativeTo: textView)
        }

        private func applySelectedInsert(in textView: NSTextView) {
            guard visibleInsertItems.indices.contains(selectedInsertIndex) else { return }
            applyInsert(visibleInsertItems[selectedInsertIndex].kind, in: textView)
        }

        private func applyInsert(_ kind: MarkdownBlockKind, in textView: NSTextView) {
            guard let context = markdownSlashContext(
                text: textView.string,
                selection: textView.selectedRange()
            ) else {
                closeInsertMenu()
                return
            }
            let replacement = markdownBlockReplacement(kind: kind, indent: context.indent)
            closeInsertMenu()
            replaceText(
                in: textView,
                range: context.lineRange,
                with: replacement.text,
                caretUTF16Offset: replacement.caretUTF16Offset
            )
            textView.window?.makeFirstResponder(textView)
        }

        private func clearSlashCommand(in textView: NSTextView) {
            guard let context = insertContext else {
                closeInsertMenu()
                return
            }
            closeInsertMenu()
            replaceText(
                in: textView,
                range: context.lineRange,
                with: context.indent,
                caretUTF16Offset: context.indent.utf16.count
            )
        }

        private func replaceText(
            in textView: NSTextView,
            range: NSRange,
            with replacement: String,
            caretUTF16Offset: Int
        ) {
            guard textView.shouldChangeText(in: range, replacementString: replacement) else {
                return
            }
            textView.textStorage?.replaceCharacters(in: range, with: replacement)
            textView.didChangeText()
            textView.setSelectedRange(NSRange(
                location: range.location + caretUTF16Offset,
                length: 0
            ))
        }

        private func closeInsertMenu() {
            insertPopover?.performClose(nil)
            insertPopover = nil
            insertContext = nil
            visibleInsertItems = []
            selectedInsertIndex = 0
        }

        private static func diffText(before: String, after: String) -> UITextEdit? {
            guard before != after else { return nil }
            let beforeCharacters = Array(before)
            let afterCharacters = Array(after)
            var prefix = 0
            while prefix < beforeCharacters.count,
                  prefix < afterCharacters.count,
                  beforeCharacters[prefix] == afterCharacters[prefix]
            {
                prefix += 1
            }
            var suffix = 0
            while suffix < beforeCharacters.count - prefix,
                  suffix < afterCharacters.count - prefix,
                  beforeCharacters[beforeCharacters.count - suffix - 1]
                    == afterCharacters[afterCharacters.count - suffix - 1]
            {
                suffix += 1
            }
            let start = String(beforeCharacters[..<prefix]).utf16.count
            let removedEnd = String(
                beforeCharacters[..<(beforeCharacters.count - suffix)]
            ).utf16.count
            let replacement = String(
                afterCharacters[prefix..<(afterCharacters.count - suffix)]
            )
            return UITextEdit(
                range: UITextRange(
                    start: position(atUTF16Offset: start, in: before),
                    end: position(atUTF16Offset: removedEnd, in: before)
                ),
                text: replacement
            )
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
