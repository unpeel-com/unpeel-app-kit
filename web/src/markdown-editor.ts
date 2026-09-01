import {
  type MarkdownEditorNode,
  type MarkdownPresentation,
  type MenuSpec,
  type TextEdit,
  type TextPosition,
  type TextSelection,
  type UiAction,
  type UiSnapshot,
  isMarkdownEditorNode,
  uiAction,
} from "./protocol";
import { renderSemanticMenu } from "./menu";

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
  private readonly insertMenu: HTMLElement;
  private readonly contextMenu: HTMLElement;
  private readonly onAction: (action: UiAction) => void;
  private readonly options: MarkdownEditorRendererOptions;
  private readonly dismissContextMenuOnPointerDown: (event: PointerEvent) => void;
  private snapshot?: UiSnapshot;
  private editor?: MarkdownEditorNode;
  private applyingSnapshot = false;
  private keyEdited = false;
  private hasRendered = false;
  private authoritativeText = "";
  private authoritativeSelection: TextSelection | undefined;
  private authoritativeRevision = -1;
  private inFlightText: string | undefined;
  private flushTimer: number | undefined;
  private visibleInsertItems: MarkdownInsertItem[] = [];
  private selectedInsertIndex = 0;
  private semanticSelectedId: string | undefined;

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
    this.insertMenu = document.createElement("div");
    this.insertMenu.className = "unpeel-markdown-editor__insert-menu";
    this.insertMenu.setAttribute("role", "listbox");
    this.insertMenu.hidden = true;
    this.insertMenu.style.position = "absolute";
    this.insertMenu.style.zIndex = "20";
    this.insertMenu.style.insetInlineStart = "12px";
    this.insertMenu.style.insetBlockStart = "12px";
    this.insertMenu.style.minWidth = "250px";
    this.insertMenu.style.padding = "6px";
    this.insertMenu.style.border = "1px solid color-mix(in srgb, CanvasText 14%, transparent)";
    this.insertMenu.style.borderRadius = "8px";
    this.insertMenu.style.background = "Canvas";
    this.insertMenu.style.boxShadow = "0 10px 30px rgb(0 0 0 / 20%)";
    this.contextMenu = document.createElement("div");
    this.contextMenu.className = "unpeel-markdown-editor__context-menu";
    this.contextMenu.hidden = true;
    this.contextMenu.style.position = "absolute";
    this.contextMenu.style.zIndex = "30";
    this.dismissContextMenuOnPointerDown = (event) => {
      const target = event.target;
      if (this.contextMenu.hidden
        || !(target instanceof Node)
        || this.contextMenu.contains(target)) return;
      this.contextMenu.hidden = true;
    };
    document.addEventListener("pointerdown", this.dismissContextMenuOnPointerDown);
    this.contextMenu.addEventListener("keydown", (event) => {
      if (event.key !== "Escape") return;
      this.contextMenu.hidden = true;
      this.textarea.focus();
    }, { capture: true });
    this.body.style.position = "relative";
    this.body.append(this.textarea, this.preview);
    this.body.append(this.insertMenu, this.contextMenu);
    this.element.append(this.toolbar, this.body);
    container.replaceChildren(this.element);

    this.textarea.addEventListener("keydown", (event) => {
      this.keyEdited = false;
      this.handleEditorKey(event);
    });
    this.textarea.addEventListener("input", () => this.textChanged());
    this.textarea.addEventListener("keyup", () => {
      if (!this.keyEdited) this.selectionChanged();
      this.keyEdited = false;
    });
    this.textarea.addEventListener("mouseup", () => this.selectionChanged());
    this.textarea.addEventListener("blur", () => this.selectionChanged());
    this.textarea.addEventListener("contextmenu", (event) => {
      if (this.editor?.contextMenu === undefined) return;
      event.preventDefault();
      this.showContextMenu(event, this.editor.contextMenu);
    });
  }

  render(snapshot: UiSnapshot): void {
    if (!isMarkdownEditorNode(snapshot.root)) {
      throw new Error(`MarkdownEditorRenderer cannot render ${snapshot.root.type}`);
    }
    const editor = snapshot.root;
    this.applyingSnapshot = true;
    try {
      const previousSelection = this.authoritativeSelection;
      let shouldApplyAuthoritativeSelection = !this.hasRendered;
      if (!this.hasRendered) {
        this.hasRendered = true;
        this.authoritativeText = editor.text;
        this.authoritativeRevision = snapshot.revision;
        this.textarea.value = editor.text;
      } else {
        const previousText = this.authoritativeText;
        const previousRevision = this.authoritativeRevision;
        const localText = this.textarea.value;
        const hadLocalChanges = localText !== previousText;
        const incomingChanged = snapshot.revision !== previousRevision
          || editor.text !== previousText;
        if (incomingChanged) {
          this.authoritativeText = editor.text;
          this.authoritativeRevision = snapshot.revision;
          if (
            this.inFlightText !== undefined
            && (snapshot.revision > previousRevision || editor.text === this.inFlightText)
          ) {
            this.inFlightText = undefined;
          }
          if (!hadLocalChanges || localText === editor.text) {
            this.textarea.value = editor.text;
            shouldApplyAuthoritativeSelection = !hadLocalChanges
              && editor.text !== previousText;
          }
        }
      }
      if (
        !shouldApplyAuthoritativeSelection
        && previousSelection !== undefined
        && !textSelectionsEqual(previousSelection, editor.selection)
        && this.textarea.value === editor.text
        && this.inFlightText === undefined
      ) {
        const previousOffsets = selectionOffsets(editor.text, previousSelection);
        const incomingOffsets = selectionOffsets(editor.text, editor.selection);
        if (incomingOffsets !== undefined) {
          const ownsFocus = document.activeElement === this.textarea;
          shouldApplyAuthoritativeSelection = !ownsFocus
            || (previousOffsets !== undefined
              && textareaSelectionMatches(this.textarea, previousOffsets))
            || textareaSelectionMatches(this.textarea, incomingOffsets);
        }
      }
      this.authoritativeSelection = editor.selection;
      this.snapshot = snapshot;
      this.editor = editor;
      if (editor.contextMenu === undefined) this.contextMenu.hidden = true;
      this.textarea.readOnly = editor.readOnly ?? false;
      this.textarea.placeholder = editor.placeholder ?? "";
      const selection = shouldApplyAuthoritativeSelection
        && this.textarea.value === this.authoritativeText
        && this.inFlightText === undefined
        ? selectionOffsets(editor.text, editor.selection)
        : undefined;
      if (selection !== undefined) {
        this.textarea.setSelectionRange(
          selection.start,
          selection.end,
          selection.direction,
        );
      }
      this.renderToolbar(editor);
      this.renderPreview(editor.text);
      this.applyPresentation(editor.presentation ?? "source");
      this.refreshInsertMenu();
    } finally {
      this.applyingSnapshot = false;
    }
    if (this.textarea.value !== this.authoritativeText && this.inFlightText === undefined) {
      this.scheduleFlush();
    }
  }

  destroy(): void {
    if (this.flushTimer !== undefined) window.clearTimeout(this.flushTimer);
    document.removeEventListener("pointerdown", this.dismissContextMenuOnPointerDown);
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
    this.keyEdited = true;
    this.refreshInsertMenu();
    this.scheduleFlush();
  }

  private selectionChanged(): void {
    if (
      this.applyingSnapshot || !this.snapshot || !this.editor
      || this.textarea.value !== this.authoritativeText
      || this.inFlightText !== undefined
    ) return;
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

  private scheduleFlush(): void {
    if (this.flushTimer !== undefined) window.clearTimeout(this.flushTimer);
    this.flushTimer = undefined;
    if (this.inFlightText !== undefined || this.textarea.value === this.authoritativeText) {
      return;
    }
    this.flushTimer = window.setTimeout(() => {
      this.flushTimer = undefined;
      this.flushLocalEdit();
    }, 90);
  }

  private flushLocalEdit(): void {
    if (!this.editor || this.inFlightText !== undefined || this.editor.readOnly) return;
    const action = componentAction(
      this.editor.actions,
      "replaceRange",
      "replace-range",
    );
    const edit = diffText(this.authoritativeText, this.textarea.value);
    if (!action || !edit) return;
    this.inFlightText = this.textarea.value;
    this.onAction(uiAction(
      this.editor.id,
      action,
      "change",
      { type: "textEdit", value: edit },
    ));
  }

  private handleEditorKey(event: KeyboardEvent): void {
    const openMenu = this.editor?.actions?.openMenu;
    if (this.editor?.insertMenu === undefined
      && openMenu !== undefined
      && (event.key === "/" || event.key === "\\")
      && !event.metaKey && !event.ctrlKey && !event.altKey
      && canOpenMarkdownMenu(
        this.textarea.value,
        this.textarea.selectionStart,
        this.textarea.selectionEnd,
      )) {
      event.preventDefault();
      this.onAction(uiAction(
        this.editor!.id,
        openMenu,
        "command",
        { type: "text", value: event.key === "/" ? "slash" : "palette" },
      ));
      return;
    }
    if (this.editor?.insertMenu !== undefined) {
      if (event.key === "ArrowUp" || event.key === "ArrowDown") {
        event.preventDefault();
        this.moveSemanticSelection(event.key === "ArrowUp" ? -1 : 1);
        return;
      }
      if (event.key === "Enter" || event.key === "Tab") {
        event.preventDefault();
        this.activateSemanticItem(this.editor.insertMenu);
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        if (this.editor.insertMenu.dismiss !== undefined) {
          this.onAction(uiAction(this.editor.id, this.editor.insertMenu.dismiss, "cancel"));
        }
        return;
      }
    }
    const context = markdownSlashContext(
      this.textarea.value,
      this.textarea.selectionStart,
      this.textarea.selectionEnd,
    );
    if (context) {
      if (event.key === "ArrowUp" || event.key === "ArrowDown") {
        event.preventDefault();
        this.moveInsertSelection(event.key === "ArrowUp" ? -1 : 1);
        return;
      }
      if (event.key === "Enter" || event.key === "Tab") {
        event.preventDefault();
        this.applySelectedInsert();
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        this.replaceLocalRange(context.lineStart, context.lineEnd, context.indent, context.indent.length);
        return;
      }
    }
    if (event.key === "Backspace") {
      const edit = markdownBackspaceEdit(
        this.textarea.value,
        this.textarea.selectionStart,
        this.textarea.selectionEnd,
      );
      if (edit) {
        event.preventDefault();
        this.replaceLocalRange(
          edit.lineStart,
          edit.lineEnd,
          edit.replacement,
          edit.caretOffset,
        );
      }
    }
  }

  private refreshInsertMenu(): void {
    if (this.textarea.hidden || this.textarea.readOnly) {
      this.closeInsertMenu();
      return;
    }
    if (this.editor?.insertMenu !== undefined) {
      const menu = this.editor.insertMenu;
      const enabled = menu.items.filter((item) => item.disabled !== true);
      if (this.semanticSelectedId === undefined
        || !enabled.some((item) => item.id === this.semanticSelectedId)) {
        this.semanticSelectedId = menu.selectedId ?? enabled[0]?.id;
      }
      const projection = { ...menu, selectedId: this.semanticSelectedId };
      this.insertMenu.hidden = false;
      renderSemanticMenu(this.insertMenu, projection, this.editor.id, (action) => {
        this.onAction(action);
        this.textarea.focus();
      });
      return;
    }
    const context = markdownSlashContext(
      this.textarea.value,
      this.textarea.selectionStart,
      this.textarea.selectionEnd,
    );
    if (!context) {
      this.closeInsertMenu();
      return;
    }
    this.visibleInsertItems = visibleMarkdownInsertItems(context.query);
    this.selectedInsertIndex = Math.min(
      this.selectedInsertIndex,
      Math.max(0, this.visibleInsertItems.length - 1),
    );
    this.insertMenu.hidden = false;
    this.insertMenu.replaceChildren();
    if (this.visibleInsertItems.length === 0) {
      const empty = document.createElement("div");
      empty.textContent = "No matching blocks";
      empty.style.padding = "6px 8px";
      empty.style.opacity = "0.65";
      this.insertMenu.append(empty);
      return;
    }
    this.visibleInsertItems.forEach((item, index) => {
      const button = document.createElement("button");
      button.type = "button";
      button.setAttribute("role", "option");
      button.setAttribute("aria-selected", String(index === this.selectedInsertIndex));
      button.style.display = "grid";
      button.style.gridTemplateColumns = "60px 1fr";
      button.style.width = "100%";
      button.style.padding = "6px 8px";
      button.style.border = "0";
      button.style.borderRadius = "5px";
      button.style.textAlign = "start";
      button.style.background = index === this.selectedInsertIndex
        ? "color-mix(in srgb, AccentColor 18%, transparent)"
        : "transparent";
      const sample = document.createElement("code");
      sample.textContent = item.sample;
      sample.style.opacity = "0.65";
      const label = document.createElement("span");
      label.textContent = item.label;
      button.append(sample, label);
      button.addEventListener("mousedown", (pointerEvent) => pointerEvent.preventDefault());
      button.addEventListener("click", () => this.applyInsert(item.kind));
      this.insertMenu.append(button);
    });
  }

  private moveInsertSelection(delta: number): void {
    if (this.visibleInsertItems.length === 0) return;
    this.selectedInsertIndex = (
      this.selectedInsertIndex + delta + this.visibleInsertItems.length
    ) % this.visibleInsertItems.length;
    this.refreshInsertMenu();
  }

  private applySelectedInsert(): void {
    const item = this.visibleInsertItems[this.selectedInsertIndex];
    if (item) this.applyInsert(item.kind);
  }

  private applyInsert(kind: MarkdownBlockKind): void {
    const context = markdownSlashContext(
      this.textarea.value,
      this.textarea.selectionStart,
      this.textarea.selectionEnd,
    );
    if (!context) return;
    const replacement = markdownBlockReplacement(kind, context.indent);
    this.replaceLocalRange(
      context.lineStart,
      context.lineEnd,
      replacement.text,
      replacement.caretOffset,
    );
    this.textarea.focus();
  }

  private replaceLocalRange(
    start: number,
    end: number,
    replacement: string,
    caretOffset: number,
  ): void {
    this.textarea.setRangeText(replacement, start, end, "end");
    const caret = start + caretOffset;
    this.textarea.setSelectionRange(caret, caret);
    this.keyEdited = true;
    this.textChanged();
  }

  private closeInsertMenu(): void {
    this.insertMenu.hidden = true;
    this.insertMenu.replaceChildren();
    this.visibleInsertItems = [];
    this.selectedInsertIndex = 0;
    this.semanticSelectedId = undefined;
  }

  private moveSemanticSelection(delta: number): void {
    const menu = this.editor?.insertMenu;
    if (menu === undefined) return;
    const enabled = menu.items.filter((item) => item.disabled !== true);
    if (enabled.length === 0) return;
    const current = Math.max(0, enabled.findIndex((item) => item.id === this.semanticSelectedId));
    this.semanticSelectedId = enabled[(current + delta + enabled.length) % enabled.length]!.id;
    this.refreshInsertMenu();
  }

  private activateSemanticItem(menu: MenuSpec): void {
    const item = menu.items.find((candidate) => candidate.id === this.semanticSelectedId);
    if (item === undefined || item.disabled === true) return;
    this.onAction(uiAction(item.id, item.action, "activate"));
  }

  private showContextMenu(event: MouseEvent, menu: MenuSpec): void {
    this.contextMenu.hidden = false;
    this.contextMenu.style.insetInlineStart = `${event.offsetX}px`;
    this.contextMenu.style.insetBlockStart = `${event.offsetY}px`;
    renderSemanticMenu(this.contextMenu, menu, this.editor?.id ?? "markdown-editor", (action) => {
      this.onAction(action);
      this.contextMenu.hidden = true;
      this.textarea.focus();
    });
    this.contextMenu.focus();
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

export type MarkdownBlockKind =
  | "heading1"
  | "heading2"
  | "heading3"
  | "heading4"
  | "heading5"
  | "heading6"
  | "paragraph"
  | "bulletList"
  | "numberedList"
  | "todo"
  | "quote"
  | "codeBlock"
  | "divider";

export interface MarkdownInsertItem {
  kind: MarkdownBlockKind;
  shortcut: string;
  label: string;
  sample: string;
  aliases: string[];
  primary: boolean;
}

export const MARKDOWN_INSERT_ITEMS: MarkdownInsertItem[] = [
  { kind: "heading1", shortcut: "1", label: "Heading 1", sample: "#", aliases: ["h1", "1", "#", "heading 1", "heading1"], primary: true },
  { kind: "heading2", shortcut: "2", label: "Heading 2", sample: "##", aliases: ["h2", "2", "##", "heading 2", "heading2"], primary: true },
  { kind: "heading3", shortcut: "3", label: "Heading 3", sample: "###", aliases: ["h3", "3", "###", "heading 3", "heading3"], primary: true },
  { kind: "heading4", shortcut: "4", label: "Heading 4", sample: "####", aliases: ["h4", "4", "####", "heading 4", "heading4"], primary: false },
  { kind: "heading5", shortcut: "5", label: "Heading 5", sample: "#####", aliases: ["h5", "5", "#####", "heading 5", "heading5"], primary: false },
  { kind: "heading6", shortcut: "6", label: "Heading 6", sample: "######", aliases: ["h6", "6", "######", "heading 6", "heading6"], primary: false },
  { kind: "paragraph", shortcut: "0", label: "Text", sample: "paragraph", aliases: ["p", "0", "text", "body", "paragraph"], primary: true },
  { kind: "bulletList", shortcut: "b", label: "Bulleted list", sample: "-", aliases: ["bullet", "bulleted", "ul", "list", "-"], primary: true },
  { kind: "numberedList", shortcut: "n", label: "Numbered list", sample: "1.", aliases: ["numbered", "ol", "number", "1"], primary: true },
  { kind: "todo", shortcut: "t", label: "To-do", sample: "[]", aliases: ["todo", "to-do", "task", "check", "checkbox"], primary: true },
  { kind: "quote", shortcut: "q", label: "Quote", sample: ">", aliases: ["quote", "blockquote", ">"], primary: true },
  { kind: "codeBlock", shortcut: "c", label: "Code", sample: "```", aliases: ["code", "fence", "pre"], primary: true },
  { kind: "divider", shortcut: "-", label: "Divider", sample: "---", aliases: ["divider", "hr", "line", "---"], primary: true },
];

export function visibleMarkdownInsertItems(query: string): MarkdownInsertItem[] {
  const normalized = query.trim().toLowerCase();
  return MARKDOWN_INSERT_ITEMS.filter((item) => {
    if (normalized.length === 0) return item.primary;
    if (item.label.toLowerCase().includes(normalized)) return true;
    return item.aliases.some((alias) => (
      alias === normalized
      || (!normalized.startsWith("#") && alias.startsWith(normalized))
    ));
  });
}

interface MarkdownSlashContext {
  lineStart: number;
  lineEnd: number;
  indent: string;
  query: string;
}

function canOpenMarkdownMenu(
  text: string,
  selectionStart: number,
  selectionEnd: number,
): boolean {
  if (selectionStart !== selectionEnd) return false;
  const lineStart = text.lastIndexOf("\n", Math.max(0, selectionStart - 1)) + 1;
  const newline = text.indexOf("\n", selectionStart);
  const lineEnd = newline === -1 ? text.length : newline;
  if (text.slice(lineStart, lineEnd).trim().length !== 0) return false;
  const fenceCount = text
    .slice(0, lineStart)
    .split("\n")
    .filter((candidate) => candidate.trimStart().startsWith("```"))
    .length;
  return fenceCount % 2 === 0;
}

function markdownSlashContext(
  text: string,
  selectionStart: number,
  selectionEnd: number,
): MarkdownSlashContext | undefined {
  if (selectionStart !== selectionEnd) return undefined;
  const lineStart = text.lastIndexOf("\n", Math.max(0, selectionStart - 1)) + 1;
  const newline = text.indexOf("\n", selectionStart);
  const lineEnd = newline === -1 ? text.length : newline;
  const line = text.slice(lineStart, lineEnd);
  const indent = line.match(/^[\t ]*/u)?.[0] ?? "";
  if (!line.slice(indent.length).startsWith("/")) return undefined;
  const slashOffset = lineStart + indent.length;
  if (selectionStart < slashOffset + 1) return undefined;
  const fenceCount = text
    .slice(0, lineStart)
    .split("\n")
    .filter((candidate) => candidate.trimStart().startsWith("```")).length;
  if (fenceCount % 2 !== 0) return undefined;
  return {
    lineStart,
    lineEnd,
    indent,
    query: text.slice(slashOffset + 1, selectionStart),
  };
}

export function markdownBlockReplacement(
  kind: MarkdownBlockKind,
  indent: string,
): { text: string; caretOffset: number } {
  let text: string;
  let caretOffset: number | undefined;
  switch (kind) {
    case "heading1": text = `${indent}# `; break;
    case "heading2": text = `${indent}## `; break;
    case "heading3": text = `${indent}### `; break;
    case "heading4": text = `${indent}#### `; break;
    case "heading5": text = `${indent}##### `; break;
    case "heading6": text = `${indent}###### `; break;
    case "paragraph": text = indent; break;
    case "bulletList": text = `${indent}- `; break;
    case "numberedList": text = `${indent}1. `; break;
    case "todo": text = `${indent}- [ ] `; break;
    case "quote": text = `${indent}> `; break;
    case "codeBlock":
      text = `${indent}\`\`\`\n\n${indent}\`\`\``;
      caretOffset = `${indent}\`\`\`\n`.length;
      break;
    case "divider": text = `${indent}---`; break;
  }
  return { text, caretOffset: caretOffset ?? text.length };
}

interface MarkdownBackspaceEdit {
  lineStart: number;
  lineEnd: number;
  replacement: string;
  caretOffset: number;
}

function markdownBackspaceEdit(
  text: string,
  selectionStart: number,
  selectionEnd: number,
): MarkdownBackspaceEdit | undefined {
  if (selectionStart !== selectionEnd) return undefined;
  const lineStart = text.lastIndexOf("\n", Math.max(0, selectionStart - 1)) + 1;
  const newline = text.indexOf("\n", selectionStart);
  const lineEnd = newline === -1 ? text.length : newline;
  const line = text.slice(lineStart, lineEnd);
  const indent = line.match(/^[\t ]*/u)?.[0] ?? "";
  const rest = line.slice(indent.length);
  const marker = ["- [ ] ", "- [x] ", "- [X] ", "- ", "* ", "+ ", "> "]
    .find((candidate) => rest.startsWith(candidate))
    ?? rest.match(/^#{1,6}\s/u)?.[0]
    ?? rest.match(/^\d+\. /u)?.[0];
  if (!marker) return undefined;
  const column = selectionStart - lineStart;
  const prefixLength = indent.length + marker.length;
  if (column <= indent.length || column > prefixLength) return undefined;
  return {
    lineStart,
    lineEnd,
    replacement: indent + rest.slice(marker.length),
    caretOffset: indent.length,
  };
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

function textSelectionsEqual(left: TextSelection, right: TextSelection): boolean {
  return left.anchor.line === right.anchor.line
    && left.anchor.utf16Column === right.anchor.utf16Column
    && left.head.line === right.head.line
    && left.head.utf16Column === right.head.utf16Column;
}

function textareaSelectionMatches(
  textarea: HTMLTextAreaElement,
  expected: NonNullable<ReturnType<typeof selectionOffsets>>,
): boolean {
  return textarea.selectionStart === expected.start
    && textarea.selectionEnd === expected.end
    && (
      expected.start === expected.end
      || textarea.selectionDirection === expected.direction
    );
}

function componentAction<K extends keyof NonNullable<MarkdownEditorNode["actions"]>>(
  actions: MarkdownEditorNode["actions"],
  key: K,
  defaultAction: string,
): string | undefined {
  return actions === undefined ? defaultAction : actions[key];
}
