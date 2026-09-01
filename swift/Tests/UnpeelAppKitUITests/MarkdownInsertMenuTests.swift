import Foundation
import Testing

@testable import UnpeelAppKitUI

@Test
func markdownInsertMenuFiltersAndBuildsClosedBlockReplacements() {
    #expect(visibleMarkdownInsertItems(query: "todo").map(\.kind) == [.todo])
    #expect(
        markdownBlockReplacement(kind: .heading2, indent: "  ").text == "  ## "
    )
    let code = markdownBlockReplacement(kind: .codeBlock, indent: "")
    #expect(code.text == "```\n\n```")
    #expect(code.caretUTF16Offset == 4)
}

@Test
func markdownTaskMarkersToggleOnlyAtTheCheckbox() {
    let text = "- [ ] first\n10. [x] second"
    #expect(markdownTaskToggleEdit(text: text, utf16Offset: 3) == MarkdownTaskToggleEdit(
        range: NSRange(location: 3, length: 1),
        replacement: "x"
    ))
    #expect(markdownTaskToggleEdit(text: text, utf16Offset: 17) == MarkdownTaskToggleEdit(
        range: NSRange(location: 17, length: 1),
        replacement: " "
    ))
    #expect(markdownTaskToggleEdit(text: text, utf16Offset: 7) == nil)
}

@Test
func markdownSlashMenuIgnoresCodeFencesAndBackspaceRemovesMarkers() {
    let source = "title\n/todo"
    let context = markdownSlashContext(
        text: source,
        selection: NSRange(location: (source as NSString).length, length: 0)
    )
    #expect(context?.query == "todo")

    let fenced = "```\n/"
    #expect(markdownSlashContext(
        text: fenced,
        selection: NSRange(location: (fenced as NSString).length, length: 0)
    ) == nil)

    let task = "- [ ] write tests"
    let edit = markdownBackspaceEdit(
        text: task,
        selection: NSRange(location: 4, length: 0)
    )
    #expect(edit?.replacement == "write tests")
    #expect(edit?.caretUTF16Offset == 0)

    #expect(canOpenMarkdownMenu(
        text: "title\n  ",
        selection: NSRange(location: 8, length: 0)
    ))
    #expect(!canOpenMarkdownMenu(
        text: "```\n",
        selection: NSRange(location: 4, length: 0)
    ))
}
