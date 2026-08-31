import {
  type MarkdownEditorNode,
  type MarkdownPresentation,
  type TextEdit,
  type TextPosition,
  type TextSelection,
  type UiAction,
  type UiSnapshot,
  isMarkdownEditorNode,
  uiAction,
} from "./protocol";

export interface MarkdownEditorRendererOptions {
  renderMarkdown?: (source: string) => Node | string;
}

/// DOM renderer for the same terminal-backed MarkdownEditor component.
export class MarkdownEditorRenderer {
  readonly element: HTMLElement;

  private readonly toolbar: HTMLElement;
  private readonly body: HTMLElement;
  private readonly textarea: HTMLTextAreaElement;
  private readonly preview: HTMLElement;
  private readonly onAction: (action: UiAction) => void;
  private readonly options: MarkdownEditorRendererOptions;
  private snapshot?: UiSnapshot;
  private editor?: MarkdownEditorNode;
  private applyingSnapshot = false;
  private keyEdited = false;

  constructor(
    container: HTMLElement,
    onAction: (action: UiAction) => void,
    options: MarkdownEditorRendererOptions = {},
  ) {
    this.onAction = onAction;
    this.options = options;
    this.element = document.createElement("section");
    this.element.className = "unpeel-markdown-editor";
    this.toolbar = document.createElement("header");
    this.toolbar.className = "unpeel-markdown-editor__toolbar";
    this.body = document.createElement("div");
    this.body.className = "unpeel-markdown-editor__body";
    this.textarea = document.createElement("textarea");
    this.textarea.className = "unpeel-markdown-editor__source";
    this.textarea.spellcheck = false;
    this.preview = document.createElement("article");
    this.preview.className = "unpeel-markdown-editor__preview";
    this.body.append(this.textarea, this.preview);
    this.element.append(this.toolbar, this.body);
    container.replaceChildren(this.element);

    this.textarea.addEventListener("keydown", () => {
      this.keyEdited = false;
    });
    this.textarea.addEventListener("input", () => this.textChanged());
    this.textarea.addEventListener("keyup", () => {
      if (!this.keyEdited) this.selectionChanged();
      this.keyEdited = false;
    });
    this.textarea.addEventListener("mouseup", () => this.selectionChanged());
    this.textarea.addEventListener("blur", () => this.selectionChanged());
  }

  render(snapshot: UiSnapshot): void {
    if (!isMarkdownEditorNode(snapshot.root)) {
      throw new Error(`MarkdownEditorRenderer cannot render ${snapshot.root.type}`);
    }
    const editor = snapshot.root;
    this.snapshot = snapshot;
    this.editor = editor;
    this.applyingSnapshot = true;
    try {
      if (this.textarea.value !== editor.text) {
        this.textarea.value = editor.text;
      }
      this.textarea.readOnly = editor.readOnly ?? false;
      this.textarea.placeholder = editor.placeholder ?? "";
      const selection = selectionOffsets(editor.text, editor.selection);
      if (selection) {
        this.textarea.setSelectionRange(
          selection.start,
          selection.end,
          selection.direction,
        );
      }
      this.renderToolbar(editor);
      this.renderPreview(editor.text);
      this.applyPresentation(editor.presentation ?? "source");
    } finally {
      this.applyingSnapshot = false;
    }
  }

  destroy(): void {
    this.element.remove();
  }

  private textChanged(): void {
    if (this.applyingSnapshot || !this.snapshot || !this.editor) return;
    const action = componentAction(
      this.editor.actions,
      "replaceRange",
      "replace-range",
    );
    if (this.editor.readOnly || !action) return;
    const edit = diffText(this.editor.text, this.textarea.value);
    if (!edit) return;
    this.keyEdited = true;
    this.onAction(uiAction(
      this.editor.id,
      action,
      "change",
      { type: "textEdit", value: edit },
    ));
    this.editor = { ...this.editor, text: this.textarea.value };
  }

  private selectionChanged(): void {
    if (this.applyingSnapshot || !this.snapshot || !this.editor) return;
    const action = componentAction(
      this.editor.actions,
      "setSelection",
      "set-selection",
    );
    if (!action) return;
    const start = positionAtUtf16Offset(
      this.textarea.value,
      this.textarea.selectionStart,
    );
    const end = positionAtUtf16Offset(
      this.textarea.value,
      this.textarea.selectionEnd,
    );
    const backwards = this.textarea.selectionDirection === "backward";
    const selection: TextSelection = {
      anchor: backwards ? end : start,
      head: backwards ? start : end,
    };
    this.onAction(uiAction(
      this.editor.id,
      action,
      "select",
      { type: "textSelection", value: selection },
    ));
  }

  private renderToolbar(editor: MarkdownEditorNode): void {
    this.toolbar.replaceChildren();
    const title = document.createElement("strong");
    title.textContent = `${editor.title ?? "Markdown"}${editor.dirty ? " •" : ""}`;
    this.toolbar.append(title);

    const presentationAction = componentAction(
      editor.actions,
      "setPresentation",
      "set-presentation",
    );
    if (presentationAction) {
      const picker = document.createElement("select");
      picker.setAttribute("aria-label", "Presentation");
      for (const presentation of ["source", "preview", "split"] as const) {
        const option = document.createElement("option");
        option.value = presentation;
        option.textContent = presentation[0]!.toUpperCase() + presentation.slice(1);
        option.selected = presentation === (editor.presentation ?? "source");
        picker.append(option);
      }
      picker.addEventListener("change", () => {
        if (!this.snapshot) return;
        this.onAction(uiAction(
          editor.id,
          presentationAction,
          "change",
          { type: "text", value: picker.value },
        ));
      });
      this.toolbar.append(picker);
    }

    const saveAction = componentAction(editor.actions, "save", "save");
    if (saveAction && !editor.readOnly) {
      const save = document.createElement("button");
      save.type = "button";
      save.textContent = "Save";
      save.addEventListener("click", () => {
        if (!this.snapshot) return;
        this.onAction(uiAction(
          editor.id,
          saveAction,
          "command",
        ));
      });
      this.toolbar.append(save);
    }
  }

  private renderPreview(source: string): void {
    this.preview.replaceChildren();
    const rendered = this.options.renderMarkdown?.(source) ?? source;
    if (typeof rendered === "string") {
      this.preview.textContent = rendered;
    } else {
      this.preview.append(rendered);
    }
  }

  private applyPresentation(presentation: MarkdownPresentation): void {
    this.element.dataset.presentation = presentation;
    this.textarea.hidden = presentation === "preview";
    this.preview.hidden = presentation === "source";
    this.body.style.display = presentation === "split" ? "grid" : "block";
    this.body.style.gridTemplateColumns = presentation === "split" ? "1fr 1fr" : "";
  }
}

export function diffText(before: string, after: string): TextEdit | undefined {
  if (before === after) return undefined;
  const beforePoints = Array.from(before);
  const afterPoints = Array.from(after);
  let prefix = 0;
  while (
    prefix < beforePoints.length
    && prefix < afterPoints.length
    && beforePoints[prefix] === afterPoints[prefix]
  ) {
    prefix += 1;
  }
  let suffix = 0;
  while (
    suffix < beforePoints.length - prefix
    && suffix < afterPoints.length - prefix
    && beforePoints[beforePoints.length - suffix - 1]
      === afterPoints[afterPoints.length - suffix - 1]
  ) {
    suffix += 1;
  }
  const startOffset = beforePoints.slice(0, prefix).join("").length;
  const removedEnd = beforePoints.slice(0, beforePoints.length - suffix).join("").length;
  const replacement = afterPoints.slice(prefix, afterPoints.length - suffix).join("");
  return {
    range: {
      start: positionAtUtf16Offset(before, startOffset),
      end: positionAtUtf16Offset(before, removedEnd),
    },
    text: replacement,
  };
}

export function positionAtUtf16Offset(text: string, target: number): TextPosition {
  const clamped = Math.max(0, Math.min(target, text.length));
  const before = text.slice(0, clamped);
  const lines = before.split("\n");
  return {
    line: lines.length - 1,
    utf16Column: lines.at(-1)?.length ?? 0,
  };
}

export function utf16OffsetAtPosition(
  text: string,
  position: TextPosition,
): number | undefined {
  if (position.line < 0 || position.utf16Column < 0) return undefined;
  const lines = text.split("\n");
  const line = lines[position.line];
  if (line === undefined || position.utf16Column > line.length) return undefined;
  let offset = position.utf16Column;
  for (let index = 0; index < position.line; index += 1) {
    offset += lines[index]!.length + 1;
  }
  return offset;
}

function selectionOffsets(
  text: string,
  selection: TextSelection,
): {
  start: number;
  end: number;
  direction: "forward" | "backward";
} | undefined {
  const anchor = utf16OffsetAtPosition(text, selection.anchor);
  const head = utf16OffsetAtPosition(text, selection.head);
  if (anchor === undefined || head === undefined) return undefined;
  return {
    start: Math.min(anchor, head),
    end: Math.max(anchor, head),
    direction: anchor > head ? "backward" : "forward",
  };
}

function componentAction<K extends keyof NonNullable<MarkdownEditorNode["actions"]>>(
  actions: MarkdownEditorNode["actions"],
  key: K,
  defaultAction: string,
): string | undefined {
  return actions === undefined ? defaultAction : actions[key];
}
