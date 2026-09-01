import {
  type BadgeSpec,
  type CheckmarkSpec,
  type ContentLine,
  type ContentSelection,
  type ContentSpec,
  type InputSpec,
  type ListItemSlot,
  type ListItemSpec,
  type ListSpec,
  type PageNode,
  type StatusSymbolSpec,
  type ToggleSpec,
  type UiAction,
  type UiSnapshot,
  isBadgeSlot,
  isCheckmarkSlot,
  isContentSpec,
  isListSpec,
  isDisclosureSlot,
  isRenderablePageNode,
  isRenderableContentPageNode,
  isStatusSlot,
  isToggleSlot,
  listItemPrimaryRole,
  uiAction,
} from "./protocol";
import { listNavigationDecision } from "./list_navigation";
import { renderSemanticMenu } from "./menu";

/** Native DOM interpretation of Page, List, ListItem, Toggle, and Input. */
export class PageRenderer {
  readonly element: HTMLElement;

  private readonly onAction: (action: UiAction) => void;
  private readonly drafts = new Map<string, string>();
  private readonly serverInputValues = new Map<string, string>();
  private readonly selections = new Map<string, string | undefined>();
  private readonly serverSelections = new Map<string, string | undefined>();
  private readonly resizeObservers: ResizeObserver[] = [];
  private contentSelection: ContentSelection | undefined;
  private contentAnchor: string | undefined;

  constructor(container: HTMLElement, onAction: (action: UiAction) => void) {
    this.onAction = onAction;
    this.element = document.createElement("section");
    this.element.className = "unpeel-page";
    container.replaceChildren(this.element);
  }

  render(snapshot: UiSnapshot): void {
    if (!isRenderablePageNode(snapshot.root) && !isRenderableContentPageNode(snapshot.root)) {
      throw new Error(`PageRenderer cannot render ${snapshot.root.type}`);
    }
    this.renderPage(snapshot.root);
  }

  destroy(): void {
    this.disconnectResizeObservers();
    this.drafts.clear();
    this.serverInputValues.clear();
    this.selections.clear();
    this.serverSelections.clear();
    this.element.remove();
  }

  private renderPage(page: PageNode & {
    header?: InputSpec;
    body: ListSpec | ContentSpec;
  }): void {
    const focusedID = document.activeElement instanceof HTMLElement
      ? document.activeElement.id
      : "";
    this.disconnectResizeObservers();
    this.element.replaceChildren();
    const pageHeader = document.createElement("header");
    pageHeader.className = "unpeel-page__header";
    if (page.back !== undefined) {
      const back = document.createElement("button");
      back.type = "button";
      back.className = "unpeel-page__back";
      back.textContent = "Back";
      back.addEventListener("click", () => {
        this.onAction(uiAction(page.id, page.back!, "cancel"));
      });
      pageHeader.append(back);
    }
    const heading = document.createElement("h1");
    heading.textContent = page.title;
    pageHeader.append(heading);
    this.element.append(pageHeader);

    if (page.header !== undefined) this.element.append(this.input(page.header));

    if (isContentSpec(page.body)) {
      this.element.append(this.content(page.body));
      this.restoreFocus(focusedID);
      return;
    }

    const body = page.body;
    if (!isListSpec(body)) return;
    const list = document.createElement("ul");
    list.className = "unpeel-list";
    list.id = `unpeel-list-${body.id}`;
    list.tabIndex = 0;
    list.setAttribute("role", "listbox");
    list.setAttribute("aria-label", page.title);
    this.reconcileSelection(body);
    list.addEventListener("keydown", (event) => this.handleListKey(event, page, body, list));
    if (body.items.length === 0) {
      const empty = document.createElement("li");
      empty.className = "unpeel-list__empty";
      empty.textContent = body.emptyMessage ?? "";
      list.append(empty);
    } else {
      for (const item of body.items) list.append(this.item(item, body));
    }
    this.element.append(list);
    this.configureValueVisibility(list, body);
    this.restoreFocus(focusedID);
  }

  private content(content: ContentSpec): HTMLElement {
    const viewport = document.createElement("div");
    viewport.className = "unpeel-content";
    viewport.dataset.wrap = String(content.wrap ?? true);
    viewport.dataset.font = content.font ?? "body";
    viewport.tabIndex = 0;
    viewport.setAttribute("role", "document");
    viewport.setAttribute("aria-label", content.label);
    this.contentSelection = content.selection;
    if (content.lines.length === 0) {
      const empty = document.createElement("p");
      empty.className = "unpeel-content__empty";
      empty.textContent = content.emptyMessage ?? "";
      viewport.append(empty);
      return viewport;
    }
    for (const line of content.lines) viewport.append(this.contentLine(content, line));
    return viewport;
  }

  private contentLine(content: ContentSpec, line: ContentLine): HTMLElement {
    const row = document.createElement("div");
    row.className = "unpeel-content__line";
    row.dataset.id = line.id;
    row.dataset.tone = line.tone ?? "default";
    row.dataset.selected = String(this.contentLineSelected(content, line.id));
    for (const run of line.runs) {
      const span = document.createElement("span");
      span.textContent = run.text;
      span.dataset.tone = run.tone ?? "default";
      span.dataset.emphasis = run.emphasis ?? "regular";
      row.append(span);
    }
    row.addEventListener("pointerdown", () => {
      this.contentAnchor = line.id;
      this.setContentSelection(content, line.id, line.id, false);
    });
    row.addEventListener("pointerenter", (event) => {
      if ((event.buttons & 1) === 0 || this.contentAnchor === undefined) return;
      this.setContentSelection(content, this.contentAnchor, line.id, false);
    });
    row.addEventListener("pointerup", () => {
      const selection = this.contentSelection;
      if (selection !== undefined) {
        this.setContentSelection(content, selection.anchorId, selection.headId, true);
      }
      this.contentAnchor = undefined;
    });
    if (content.contextMenu !== undefined) {
      row.addEventListener("contextmenu", (event) => {
        event.preventDefault();
        this.setContentSelection(content, line.id, line.id, true);
        this.showContextMenu(event, content.contextMenu!, line.id);
      });
    }
    return row;
  }

  private contentLineSelected(content: ContentSpec, id: string): boolean {
    const selection = this.contentSelection;
    if (selection === undefined) return false;
    const anchor = content.lines.findIndex((line) => line.id === selection.anchorId);
    const head = content.lines.findIndex((line) => line.id === selection.headId);
    const index = content.lines.findIndex((line) => line.id === id);
    return index >= Math.min(anchor, head) && index <= Math.max(anchor, head);
  }

  private setContentSelection(
    content: ContentSpec,
    anchorId: string,
    headId: string,
    publish: boolean,
  ): void {
    const next = { anchorId, headId };
    this.contentSelection = next;
    for (const row of this.element.querySelectorAll<HTMLElement>(".unpeel-content__line")) {
      row.dataset.selected = String(this.contentLineSelected(content, row.dataset.id ?? ""));
    }
    if (publish && content.select !== undefined) {
      this.onAction(uiAction(
        content.id,
        content.select,
        "select",
        { type: "textList", value: [anchorId, headId] },
      ));
    }
  }

  private showContextMenu(event: MouseEvent, menu: NonNullable<ContentSpec["contextMenu"]>, target: string): void {
    const host = document.createElement("div");
    host.className = "unpeel-context-menu";
    host.style.position = "fixed";
    host.style.left = `${event.clientX}px`;
    host.style.top = `${event.clientY}px`;
    host.style.zIndex = "50";
    renderSemanticMenu(host, menu, target, (action) => {
      this.onAction({ ...action, value: { type: "text", value: target } });
      host.remove();
    });
    document.body.append(host);
    host.focus();
    const dismiss = (pointer: PointerEvent): void => {
      if (!host.contains(pointer.target as Node)) host.remove();
      document.removeEventListener("pointerdown", dismiss, true);
    };
    queueMicrotask(() => document.addEventListener("pointerdown", dismiss, true));
  }

  private restoreFocus(focusedID: string): void {
    if (focusedID === "") return;
    const escaped = typeof CSS !== "undefined" && typeof CSS.escape === "function"
      ? CSS.escape(focusedID)
      : focusedID.replace(/[^A-Za-z0-9_-]/g, "\\$&");
    this.element.querySelector<HTMLElement>(`#${escaped}`)?.focus();
  }

  private input(input: InputSpec): HTMLFormElement {
    const form = document.createElement("form");
    form.className = "unpeel-input";
    const label = document.createElement("label");
    label.htmlFor = input.id;
    label.textContent = input.label;
    const field = document.createElement("input");
    field.id = input.id;
    field.type = "text";
    const serverValue = input.value ?? "";
    if (this.serverInputValues.get(input.id) !== serverValue || !this.drafts.has(input.id)) {
      this.drafts.set(input.id, serverValue);
    }
    this.serverInputValues.set(input.id, serverValue);
    field.value = this.drafts.get(input.id) ?? serverValue;
    field.placeholder = input.placeholder ?? "";
    field.addEventListener("input", () => this.drafts.set(input.id, field.value));
    if (input.setValue !== undefined) {
      field.addEventListener("change", () => {
        this.onAction(uiAction(
          input.id,
          input.setValue!,
          "change",
          { type: "text", value: field.value },
        ));
      });
    }
    form.append(label, field);
    if (input.submit !== undefined) {
      const add = document.createElement("button");
      add.type = "submit";
      add.textContent = "Add";
      form.append(add);
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        this.onAction(uiAction(
          input.id,
          input.submit!,
          "submit",
          { type: "text", value: field.value },
        ));
        field.value = "";
        this.drafts.set(input.id, "");
        field.focus();
      });
    }
    return form;
  }

  private item(item: ListItemSpec, list: ListSpec): HTMLLIElement {
    const row = document.createElement("li");
    row.className = "unpeel-list-item";
    row.dataset.id = item.id;
    row.dataset.done = String(item.done ?? false);
    row.dataset.role = listItemPrimaryRole(item);
    row.dataset.actionRole = item.actionRole ?? "default";
    row.dataset.selected = String(this.selections.get(list.id) === item.id);
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", row.dataset.selected);
    row.addEventListener("click", (event) => {
      this.select(list, item.id);
      const control = event.target instanceof Element
        ? event.target.closest("button, input, label")
        : null;
      if (control !== null) return;
      row.parentElement?.focus();
      this.invokePrimary(item);
    });
    if (list.contextMenu !== undefined) {
      row.addEventListener("contextmenu", (event) => {
        event.preventDefault();
        this.select(list, item.id);
        this.showContextMenu(event, list.contextMenu!, item.id);
      });
    }
    if (item.busy === true) {
      const busy = document.createElement("span");
      busy.className = "unpeel-list-item__busy";
      busy.textContent = "◌";
      busy.setAttribute("role", "progressbar");
      busy.setAttribute("aria-label", "Loading");
      row.append(busy);
    }
    this.appendSlot(row, item.leading);
    const labelContent = document.createElement("span");
    labelContent.className = "unpeel-list-item__content";
    const label = document.createElement("span");
    label.className = "unpeel-list-item__label";
    label.textContent = item.label;
    label.dataset.tone = item.labelTone ?? "default";
    label.dataset.emphasis = item.emphasis ?? "regular";
    labelContent.append(label);
    if (item.detail !== undefined) {
      const detail = document.createElement("span");
      detail.className = "unpeel-list-item__detail";
      detail.textContent = item.detail;
      labelContent.append(detail);
    }
    row.append(labelContent);
    if (item.value !== undefined) {
      const value = document.createElement("span");
      value.className = "unpeel-list-item__value";
      value.textContent = item.value;
      value.dataset.tone = item.valueTone ?? "muted";
      const minimum = item.valueMinWidth
        ?? Math.min(item.value.length + 11, 65_535);
      value.dataset.minColumns = String(minimum);
      row.append(value);
    }
    this.appendSlot(row, item.trailing);
    this.appendSlot(row, item.accessory);
    if (item.delete !== undefined) {
      const remove = document.createElement("button");
      remove.type = "button";
      remove.className = "unpeel-list-item__delete";
      remove.textContent = "Delete";
      remove.setAttribute("aria-label", `Delete ${item.label}`);
      remove.addEventListener("click", () => {
        this.onAction(uiAction(item.id, item.delete!, "change"));
      });
      row.append(remove);
    }
    return row;
  }

  private appendSlot(row: HTMLElement, slot: ListItemSlot | undefined): void {
    if (slot === undefined) return;
    if (isToggleSlot(slot)) row.append(this.toggle(slot));
    else if (isStatusSlot(slot)) row.append(this.status(slot));
    else if (isBadgeSlot(slot)) row.append(this.badge(slot));
    else if (isDisclosureSlot(slot)) row.append(this.disclosure());
    else if (isCheckmarkSlot(slot)) row.append(this.checkmark(slot));
  }

  private status(status: StatusSymbolSpec): HTMLSpanElement {
    const element = document.createElement("span");
    element.className = "unpeel-status-symbol";
    element.textContent = status.symbol;
    element.dataset.tone = status.tone ?? "default";
    element.dataset.emphasis = status.emphasis ?? "regular";
    element.setAttribute("aria-label", status.label);
    return element;
  }

  private badge(badge: BadgeSpec): HTMLSpanElement {
    const element = document.createElement("span");
    element.className = "unpeel-badge";
    element.textContent = badge.text;
    element.dataset.tone = badge.tone ?? "default";
    return element;
  }

  private toggle(toggle: ToggleSpec): HTMLLabelElement {
    const label = document.createElement("label");
    label.className = "unpeel-toggle";
    const input = document.createElement("input");
    input.type = "checkbox";
    input.checked = toggle.value;
    input.setAttribute("aria-label", toggle.label);
    input.addEventListener("change", () => {
      this.onAction(uiAction(
        toggle.id,
        toggle.setValue,
        "change",
        { type: "bool", value: input.checked },
      ));
    });
    const text = document.createElement("span");
    text.textContent = toggle.label;
    text.className = "unpeel-toggle__label";
    label.append(input, text);
    return label;
  }

  private disclosure(): HTMLSpanElement {
    const element = document.createElement("span");
    element.className = "unpeel-list-item__disclosure";
    element.textContent = "›";
    element.setAttribute("aria-hidden", "true");
    return element;
  }

  private checkmark(checkmark: CheckmarkSpec): HTMLSpanElement {
    const element = document.createElement("span");
    element.className = "unpeel-list-item__checkmark";
    element.textContent = checkmark.value ? "✓" : "";
    element.setAttribute(
      "aria-label",
      `${checkmark.label}: ${checkmark.value ? "selected" : "not selected"}`,
    );
    return element;
  }

  private invokePrimary(item: ListItemSpec): boolean {
    const role = listItemPrimaryRole(item);
    if (role === "toggle") {
      const toggle = [item.leading, item.trailing, item.accessory]
        .find((slot): slot is ToggleSpec => slot !== undefined && isToggleSlot(slot));
      if (toggle === undefined) return false;
      this.onAction(uiAction(
        toggle.id,
        toggle.setValue,
        "change",
        { type: "bool", value: !toggle.value },
      ));
      return true;
    }
    if (role === "checkmark") {
      const checkmark = [item.leading, item.trailing, item.accessory]
        .find((slot): slot is CheckmarkSpec => slot !== undefined && isCheckmarkSlot(slot));
      if (checkmark === undefined) return false;
      this.onAction(uiAction(
        checkmark.id,
        checkmark.setValue,
        "change",
        { type: "bool", value: !checkmark.value },
      ));
      return true;
    }
    if ((role === "disclosure" || role === "command" || role === "destructive")
      && item.activate !== undefined) {
      this.onAction(uiAction(item.id, item.activate, "activate"));
      return true;
    }
    return false;
  }

  private reconcileSelection(list: ListSpec): void {
    const previousServer = this.serverSelections.get(list.id);
    if (!this.selections.has(list.id) || previousServer !== list.selectedId) {
      this.selections.set(list.id, list.selectedId);
    }
    this.serverSelections.set(list.id, list.selectedId);
  }

  private select(list: ListSpec, itemID: string): void {
    if (!list.items.some((item) => item.id === itemID)) return;
    const changed = this.selections.get(list.id) !== itemID;
    this.selections.set(list.id, itemID);
    for (const row of this.element.querySelectorAll<HTMLElement>(".unpeel-list-item")) {
      const selected = row.dataset.id === itemID;
      row.dataset.selected = String(selected);
      row.setAttribute("aria-selected", String(selected));
    }
    if (changed && list.select !== undefined) {
      this.onAction(uiAction(
        list.id,
        list.select,
        "change",
        { type: "text", value: itemID },
      ));
    }
  }

  private handleListKey(
    event: KeyboardEvent,
    page: PageNode,
    list: ListSpec,
    element: HTMLElement,
  ): void {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    if (list.items.length === 0) return;
    const selectedID = this.selections.get(list.id);
    const selectedIndex = list.items.findIndex((item) => item.id === selectedID);
    const current = selectedIndex >= 0 ? selectedIndex : 0;
    const item = list.items[current];
    const decision = listNavigationDecision(event.key, listItemPrimaryRole(item));
    if (decision === "back") {
      if (page.back === undefined) return;
      event.preventDefault();
      this.onAction(uiAction(page.id, page.back, "cancel"));
      return;
    }
    if (decision === "invokePrimary") {
      event.preventDefault();
      this.invokePrimary(item);
      return;
    }
    if ((decision === "pageDown" || decision === "pageUp")
      && (list.pageBehavior ?? "selection") === "scroll") {
      return;
    }
    const firstRow = element.querySelector<HTMLElement>(".unpeel-list-item");
    const rowHeight = Math.max(firstRow?.getBoundingClientRect().height ?? 28, 1);
    const visibleRows = Math.max(Math.floor(element.clientHeight / rowHeight), 1);
    const pageRows = Math.max(visibleRows - (list.pageOverlap ?? 1), 1);
    const last = list.items.length - 1;
    let next: number | undefined;
    switch (decision) {
      case "down": next = Math.min(current + 1, last); break;
      case "up": next = Math.max(current - 1, 0); break;
      case "first": next = 0; break;
      case "last": next = last; break;
      case "pageDown": next = Math.min(current + pageRows, last); break;
      case "pageUp": next = Math.max(current - pageRows, 0); break;
      default: break;
    }
    if (next === undefined) return;
    event.preventDefault();
    const itemID = list.items[next].id;
    this.select(list, itemID);
    element.querySelector<HTMLElement>(`.unpeel-list-item[data-id="${this.escapeAttribute(itemID)}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }

  private configureValueVisibility(element: HTMLElement, list: ListSpec): void {
    if (typeof ResizeObserver === "undefined") return;
    for (const row of element.querySelectorAll<HTMLElement>(".unpeel-list-item")) {
      const item = list.items.find((candidate) => candidate.id === row.dataset.id);
      const value = row.querySelector<HTMLElement>(".unpeel-list-item__value");
      if (item === undefined || value === null) continue;
      const minimum = item.valueMinWidth ?? Math.min((item.value?.length ?? 0) + 11, 65_535);
      const update = (): void => {
        const width = row.getBoundingClientRect().width;
        if (width > 0) value.hidden = width < minimum * 8;
      };
      const observer = new ResizeObserver(update);
      observer.observe(row);
      this.resizeObservers.push(observer);
      update();
    }
  }

  private disconnectResizeObservers(): void {
    for (const observer of this.resizeObservers) observer.disconnect();
    this.resizeObservers.length = 0;
  }

  private escapeAttribute(value: string): string {
    return value.replace(/(["\\])/g, "\\$1");
  }
}
