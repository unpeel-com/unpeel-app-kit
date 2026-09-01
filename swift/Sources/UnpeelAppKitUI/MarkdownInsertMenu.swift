import AppKit
import SwiftUI

enum MarkdownBlockKind: String, CaseIterable, Identifiable, Sendable {
    case heading1
    case heading2
    case heading3
    case heading4
    case heading5
    case heading6
    case paragraph
    case bulletList
    case numberedList
    case todo
    case quote
    case codeBlock
    case divider

    var id: String { rawValue }
}

struct MarkdownInsertItem: Identifiable, Equatable, Sendable {
    let kind: MarkdownBlockKind
    let shortcut: Character
    let label: String
    let sample: String
    let aliases: [String]
    let primary: Bool

    var id: String { kind.id }
}

let markdownInsertItems: [MarkdownInsertItem] = [
    .init(kind: .heading1, shortcut: "1", label: "Heading 1", sample: "#", aliases: ["h1", "1", "#", "heading 1", "heading1"], primary: true),
    .init(kind: .heading2, shortcut: "2", label: "Heading 2", sample: "##", aliases: ["h2", "2", "##", "heading 2", "heading2"], primary: true),
    .init(kind: .heading3, shortcut: "3", label: "Heading 3", sample: "###", aliases: ["h3", "3", "###", "heading 3", "heading3"], primary: true),
    .init(kind: .heading4, shortcut: "4", label: "Heading 4", sample: "####", aliases: ["h4", "4", "####", "heading 4", "heading4"], primary: false),
    .init(kind: .heading5, shortcut: "5", label: "Heading 5", sample: "#####", aliases: ["h5", "5", "#####", "heading 5", "heading5"], primary: false),
    .init(kind: .heading6, shortcut: "6", label: "Heading 6", sample: "######", aliases: ["h6", "6", "######", "heading 6", "heading6"], primary: false),
    .init(kind: .paragraph, shortcut: "0", label: "Text", sample: "paragraph", aliases: ["p", "0", "text", "body", "paragraph"], primary: true),
    .init(kind: .bulletList, shortcut: "b", label: "Bulleted list", sample: "-", aliases: ["bullet", "bulleted", "ul", "list", "-"], primary: true),
    .init(kind: .numberedList, shortcut: "n", label: "Numbered list", sample: "1.", aliases: ["numbered", "ol", "number", "1"], primary: true),
    .init(kind: .todo, shortcut: "t", label: "To-do", sample: "[]", aliases: ["todo", "to-do", "task", "check", "checkbox"], primary: true),
    .init(kind: .quote, shortcut: "q", label: "Quote", sample: ">", aliases: ["quote", "blockquote", ">"], primary: true),
    .init(kind: .codeBlock, shortcut: "c", label: "Code", sample: "```", aliases: ["code", "fence", "pre"], primary: true),
    .init(kind: .divider, shortcut: "-", label: "Divider", sample: "---", aliases: ["divider", "hr", "line", "---"], primary: true),
]

func visibleMarkdownInsertItems(query: String) -> [MarkdownInsertItem] {
    let query = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    return markdownInsertItems.filter { item in
        if query.isEmpty { return item.primary }
        if item.label.lowercased().contains(query) { return true }
        return item.aliases.contains { alias in
            alias == query || (!query.hasPrefix("#") && alias.hasPrefix(query))
        }
    }
}

struct MarkdownSlashContext {
    let lineRange: NSRange
    let indent: String
    let query: String
}

struct MarkdownBackspaceEdit {
    let lineRange: NSRange
    let replacement: String
    let caretUTF16Offset: Int
}

/// Matches the App's rule for `/` and `\`: a collapsed caret on an otherwise
/// blank, unfenced line. The renderer only requests a Menu; the App rechecks
/// this against authoritative state before opening it.
func canOpenMarkdownMenu(text: String, selection: NSRange) -> Bool {
    guard selection.location != NSNotFound, selection.length == 0 else { return false }
    let source = text as NSString
    guard selection.location <= source.length else { return false }
    let lineRange = source.lineRange(for: NSRange(location: selection.location, length: 0))
    var contentEnd = NSMaxRange(lineRange)
    while contentEnd > lineRange.location,
          [10, 13].contains(source.character(at: contentEnd - 1))
    {
        contentEnd -= 1
    }
    let content = source.substring(with: NSRange(
        location: lineRange.location,
        length: contentEnd - lineRange.location
    ))
    guard content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return false }
    let prefix = source.substring(to: lineRange.location)
    let fenceCount = prefix
        .components(separatedBy: .newlines)
        .filter { $0.trimmingCharacters(in: .whitespaces).hasPrefix("```") }
        .count
    return fenceCount.isMultiple(of: 2)
}

func markdownBackspaceEdit(text: String, selection: NSRange) -> MarkdownBackspaceEdit? {
    guard selection.location != NSNotFound, selection.length == 0 else { return nil }
    let source = text as NSString
    guard selection.location <= source.length else { return nil }
    let lineRangeWithEnding = source.lineRange(
        for: NSRange(location: selection.location, length: 0)
    )
    var contentEnd = NSMaxRange(lineRangeWithEnding)
    while contentEnd > lineRangeWithEnding.location,
          [10, 13].contains(source.character(at: contentEnd - 1))
    {
        contentEnd -= 1
    }
    let lineRange = NSRange(
        location: lineRangeWithEnding.location,
        length: contentEnd - lineRangeWithEnding.location
    )
    let line = source.substring(with: lineRange)
    let indent = String(line.prefix { $0 == " " || $0 == "\t" })
    let rest = String(line.dropFirst(indent.count))
    let markerLength: Int
    if let marker = ["- [ ] ", "- [x] ", "- [X] "].first(where: rest.hasPrefix) {
        markerLength = marker.utf16.count
    } else if let marker = ["- ", "* ", "+ ", "> "].first(where: rest.hasPrefix) {
        markerLength = marker.utf16.count
    } else {
        let hashes = rest.prefix { $0 == "#" }.count
        if (1...6).contains(hashes), rest.dropFirst(hashes).first?.isWhitespace == true {
            markerLength = hashes + 1
        } else {
            let digits = rest.prefix { $0.isNumber }
            let numberedMarker = "\(digits). "
            guard !digits.isEmpty, rest.hasPrefix(numberedMarker) else { return nil }
            markerLength = numberedMarker.utf16.count
        }
    }
    let column = selection.location - lineRange.location
    let prefixLength = indent.utf16.count + markerLength
    guard column > indent.utf16.count, column <= prefixLength else { return nil }
    let body = String(rest.dropFirst(markerLength))
    return MarkdownBackspaceEdit(
        lineRange: lineRange,
        replacement: indent + body,
        caretUTF16Offset: indent.utf16.count
    )
}

func markdownSlashContext(text: String, selection: NSRange) -> MarkdownSlashContext? {
    guard selection.location != NSNotFound, selection.length == 0 else { return nil }
    let source = text as NSString
    guard selection.location <= source.length else { return nil }
    let lineRange = source.lineRange(for: NSRange(location: selection.location, length: 0))
    var contentEnd = NSMaxRange(lineRange)
    while contentEnd > lineRange.location {
        let scalar = source.character(at: contentEnd - 1)
        if scalar == 10 || scalar == 13 {
            contentEnd -= 1
        } else {
            break
        }
    }
    let contentRange = NSRange(
        location: lineRange.location,
        length: contentEnd - lineRange.location
    )
    let line = source.substring(with: contentRange)
    let indent = String(line.prefix { $0 == " " || $0 == "\t" })
    guard line.dropFirst(indent.utf16.count).hasPrefix("/") else { return nil }
    let slashOffset = contentRange.location + indent.utf16.count
    guard selection.location >= slashOffset + 1 else { return nil }

    let prefix = source.substring(to: contentRange.location)
    let fenceCount = prefix
        .components(separatedBy: .newlines)
        .filter { $0.trimmingCharacters(in: .whitespaces).hasPrefix("```") }
        .count
    guard fenceCount.isMultiple(of: 2) else { return nil }

    let queryRange = NSRange(
        location: slashOffset + 1,
        length: max(0, selection.location - slashOffset - 1)
    )
    return MarkdownSlashContext(
        lineRange: contentRange,
        indent: indent,
        query: source.substring(with: queryRange)
    )
}

func markdownBlockReplacement(
    kind: MarkdownBlockKind,
    indent: String
) -> (text: String, caretUTF16Offset: Int) {
    let text: String
    let caret: Int
    switch kind {
    case .heading1: text = "\(indent)# "; caret = text.utf16.count
    case .heading2: text = "\(indent)## "; caret = text.utf16.count
    case .heading3: text = "\(indent)### "; caret = text.utf16.count
    case .heading4: text = "\(indent)#### "; caret = text.utf16.count
    case .heading5: text = "\(indent)##### "; caret = text.utf16.count
    case .heading6: text = "\(indent)###### "; caret = text.utf16.count
    case .paragraph: text = indent; caret = text.utf16.count
    case .bulletList: text = "\(indent)- "; caret = text.utf16.count
    case .numberedList: text = "\(indent)1. "; caret = text.utf16.count
    case .todo: text = "\(indent)- [ ] "; caret = text.utf16.count
    case .quote: text = "\(indent)> "; caret = text.utf16.count
    case .codeBlock:
        text = "\(indent)```\n\n\(indent)```"
        caret = "\(indent)```\n".utf16.count
    case .divider: text = "\(indent)---"; caret = text.utf16.count
    }
    return (text, caret)
}

@MainActor
struct MarkdownInsertMenuView: View {
    let items: [MarkdownInsertItem]
    let selectedIndex: Int
    let onSelect: (MarkdownBlockKind) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            if items.isEmpty {
                Text("No matching blocks")
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 10)
                    .frame(height: 30)
            } else {
                ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                    HStack(spacing: 8) {
                        Text(String(item.shortcut))
                            .font(.system(.body, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .frame(width: 14, alignment: .leading)
                        Text(item.label)
                            .frame(width: 116, alignment: .leading)
                        Text(item.sample)
                            .font(.system(.body, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                        Spacer(minLength: 0)
                    }
                    .padding(.horizontal, 8)
                    .frame(maxWidth: .infinity, minHeight: 28, alignment: .leading)
                    .contentShape(Rectangle())
                    .background(
                        index == selectedIndex ? Color.accentColor.opacity(0.18) : .clear,
                        in: RoundedRectangle(cornerRadius: 5)
                    )
                    .onTapGesture {
                        onSelect(item.kind)
                    }
                    .accessibilityElement(children: .combine)
                    .accessibilityAddTraits(.isButton)
                    .accessibilityAddTraits(index == selectedIndex ? .isSelected : [])
                }
            }
        }
        .padding(6)
        .frame(width: 238, alignment: .leading)
        .background(.regularMaterial)
    }
}
