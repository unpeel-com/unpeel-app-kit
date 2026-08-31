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
}
