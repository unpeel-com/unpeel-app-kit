import {
  type InputSpec,
  type ListItemSlot,
  type ListItemSpec,
  type ListSpec,
  type PageNode,
  type ToggleSpec,
  type UiAction,
  type UiSnapshot,
  isRenderablePageNode,
  isToggleSlot,
  uiAction,
} from "./protocol";

/** Native DOM interpretation of Page, List, ListItem, Toggle, and Input. */
export class PageRenderer {
  readonly element: HTMLElement;

  private readonly onAction: (action: UiAction) => void;
  private readonly drafts = new Map<string, string>();
  private readonly serverInputValues = new Map<string, string>();

  constructor(container: HTMLElement, onAction: (action: UiAction) => void) {
    this.onAction = onAction;
    this.element = document.createElement("section");
    this.element.className = "unpeel-page";
    container.replaceChildren(this.element);
  }

  render(snapshot: UiSnapshot): void {
    if (!isRenderablePageNode(snapshot.root)) {
      throw new Error(`PageRenderer cannot render ${snapshot.root.type}`);
    }
    this.renderPage(snapshot.root);
  }

  destroy(): void {
    this.drafts.clear();
    this.serverInputValues.clear();
    this.element.remove();
  }

  private renderPage(page: PageNode & {
    header?: InputSpec;
    body: ListSpec;
  }): void {
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

    const list = document.createElement("ul");
    list.className = "unpeel-list";
    if (page.body.items.length === 0) {
      const empty = document.createElement("li");
      empty.className = "unpeel-list__empty";
      empty.textContent = page.body.emptyMessage ?? "";
      list.append(empty);
    } else {
      for (const item of page.body.items) list.append(this.item(item));
    }
    this.element.append(list);
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

  private item(item: ListItemSpec): HTMLLIElement {
    const row = document.createElement("li");
    row.className = "unpeel-list-item";
    row.dataset.id = item.id;
    row.dataset.done = String(item.done ?? false);
    this.appendSlot(row, item.leading);
    const labelContent = document.createElement("span");
    labelContent.className = "unpeel-list-item__content";
    const label = document.createElement("span");
    label.className = "unpeel-list-item__label";
    label.textContent = item.label;
    labelContent.append(label);
    if (item.detail !== undefined) {
      const detail = document.createElement("span");
      detail.className = "unpeel-list-item__detail";
      detail.textContent = item.detail;
      labelContent.append(detail);
    }
    if (item.activate !== undefined) {
      const activate = document.createElement("button");
      activate.type = "button";
      activate.className = "unpeel-list-item__activate";
      activate.append(labelContent);
      activate.addEventListener("click", () => {
        this.onAction(uiAction(item.id, item.activate!, "activate"));
      });
      row.append(activate);
    } else {
      row.append(labelContent);
    }
    if (item.value !== undefined) {
      const value = document.createElement("span");
      value.className = "unpeel-list-item__value";
      value.textContent = item.value;
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
    if (slot === undefined || !isToggleSlot(slot)) return;
    row.append(this.toggle(slot));
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
}
