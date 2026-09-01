import Foundation
import Testing

@testable import UnpeelAppKitUI

@Test
func markdownCommandHintAndMenuTriggersComeFromTheSpec() {
    let editor = MarkdownEditorSpec(
        text: "title\n\nbody",
        selection: .caret(UITextPosition(line: 1, utf16Column: 0)),
        commandHint: MarkdownCommandHint(text: "Type '/' for commands"),
        actions: MarkdownEditorActions(openMenu: "open-menu")
    )
    #expect(editor.commandHintVisible)
    #expect(editor.menuTrigger(forTextInput: "/") == .slash)
    #expect(editor.menuTrigger(forTextInput: "\\") == .palette)
    let preview = MarkdownEditorSpec(
        text: editor.text,
        selection: editor.selection,
        presentation: .preview,
        commandHint: editor.commandHint,
        actions: editor.actions
    )
    #expect(!preview.commandHintVisible)

    let ordinaryLine = MarkdownEditorSpec(
        text: "not blank",
        selection: .caret(UITextPosition(line: 0, utf16Column: 9)),
        actions: MarkdownEditorActions(openMenu: "open-menu")
    )
    #expect(
        ordinaryLine.menuTrigger(forTextInput: "/") == .slash,
        "the Rust reducer, not the renderer, decides whether slash opens a Menu"
    )
}

@Test
func markdownTaskMarkersToggleOnlyAtTheCheckbox() {
    let text = "- [ ] first\n10. [x] second"
    #expect(
        markdownTaskToggleEdit(text: text, utf16Offset: 3)
            == MarkdownTaskToggleEdit(
                range: NSRange(location: 3, length: 1),
                replacement: "x"
            ))
    #expect(
        markdownTaskToggleEdit(text: text, utf16Offset: 17)
            == MarkdownTaskToggleEdit(
                range: NSRange(location: 17, length: 1),
                replacement: " "
            ))
    #expect(markdownTaskToggleEdit(text: text, utf16Offset: 7) == nil)
}

@Test
func markdownBackspaceRemovesMarkers() {
    let task = "- [ ] write tests"
    let edit = markdownBackspaceEdit(
        text: task,
        selection: NSRange(location: 4, length: 0)
    )
    #expect(edit?.replacement == "write tests")
    #expect(edit?.caretUTF16Offset == 0)
}
