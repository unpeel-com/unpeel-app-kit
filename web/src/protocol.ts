export const UI_PROTOCOL_NAME = "unpeel.ui" as const;
export const UI_PROTOCOL_MIN_VERSION = 1 as const;
export const UI_PROTOCOL_MAX_VERSION = 1 as const;
export const UI_PROTOCOL_VERSION = UI_PROTOCOL_MAX_VERSION;

export interface AppMetadata {
  id: string;
  name: string;
  version: string;
  description?: string;
}

export interface UiParticipant {
  id: string;
  displayName?: string;
  color?: string;
  grants?: string[];
}

export interface UiRendererMetadata {
  id: string;
  kind: string;
  capabilities?: string[];
}

export interface UiRendererState {
  rendererVisible: boolean;
  terminalVisible: boolean;
}

/** Trusted local-broker message. Never expose its authToken to browser code. */
export interface UiAttach {
  type: "attach";
  protocol: typeof UI_PROTOCOL_NAME;
  minProtocolVersion: number;
  maxProtocolVersion: number;
  authToken: string;
  participant: UiParticipant;
  clientId: string;
  renderer: UiRendererMetadata;
  viewId: string;
  expectedAppInstanceId?: string;
  lastSeenRevision?: number;
  state?: UiRendererState;
}

export interface UiAttached {
  type: "attached";
  protocol: typeof UI_PROTOCOL_NAME;
  protocolVersion: number;
  minProtocolVersion: number;
  maxProtocolVersion: number;
  app: AppMetadata;
  appInstanceId: string;
  participantId: string;
  clientId: string;
  rendererId: string;
  viewId: string;
  resumed: boolean;
  currentRevision?: number;
}

export interface TextPosition {
  line: number;
  utf16Column: number;
}

export interface TextRange {
  start: TextPosition;
  end: TextPosition;
}

export interface TextSelection {
  anchor: TextPosition;
  head: TextPosition;
}

export interface TextEdit {
  range: TextRange;
  text: string;
}

export type MarkdownPresentation = "source" | "preview" | "split";

export interface MarkdownEditorActions {
  replaceRange?: string;
  setSelection?: string;
  save?: string;
  undo?: string;
  redo?: string;
  setPresentation?: string;
}

export interface MarkdownEditorNode {
  id: string;
  type: "markdownEditor";
  text: string;
  selection: TextSelection;
  presentation?: MarkdownPresentation;
  readOnly?: boolean;
  dirty?: boolean;
  placeholder?: string;
  title?: string;
  actions?: MarkdownEditorActions;
}

export type UiNode = MarkdownEditorNode;

export interface UiSnapshot {
  type: "snapshot";
  protocol: typeof UI_PROTOCOL_NAME;
  protocolVersion: number;
  appInstanceId: string;
  clientId: string;
  viewId: string;
  revision: number;
  root: UiNode;
}

export type UiEventKind =
  | "activate"
  | "select"
  | "change"
  | "submit"
  | "cancel"
  | "command";

export type UiEventValue =
  | { type: "none" }
  | { type: "bool"; value: boolean }
  | { type: "index"; value: number }
  | { type: "integer"; value: number }
  | { type: "number"; value: number }
  | { type: "text"; value: string }
  | { type: "textList"; value: string[] }
  | { type: "textEdit"; value: TextEdit }
  | { type: "textSelection"; value: TextSelection };

/** Renderer-local action before authenticated session context is applied. */
export interface UiAction {
  nodeId: string;
  action: string;
  kind: UiEventKind;
  value: UiEventValue;
}

export interface UiEvent extends UiAction {
  type: "event";
  protocol: typeof UI_PROTOCOL_NAME;
  protocolVersion: number;
  appInstanceId: string;
  participantId: string;
  clientId: string;
  rendererId: string;
  viewId: string;
  eventId: string;
  baseRevision: number;
}

export type UiAckStatus = "pending" | "applied" | "rejected" | "stale";

export interface UiAck {
  type: "ack";
  protocol: typeof UI_PROTOCOL_NAME;
  protocolVersion: number;
  appInstanceId: string;
  clientId: string;
  rendererId: string;
  viewId: string;
  eventId: string;
  status: UiAckStatus;
  revision: number;
  message?: string;
}

export interface UiLifecycle {
  type: "lifecycle";
  protocol: typeof UI_PROTOCOL_NAME;
  protocolVersion: number;
  appInstanceId: string;
  clientId: string;
  rendererId: string;
  viewId: string;
  state: UiRendererState;
}

export interface UiRequestSnapshot {
  type: "requestSnapshot";
  protocol: typeof UI_PROTOCOL_NAME;
  protocolVersion: number;
  appInstanceId: string;
  clientId: string;
  rendererId: string;
  viewId: string;
}

export interface UiPresenceMember {
  participant: UiParticipant;
  clientId: string;
  renderer: UiRendererMetadata;
  state: UiRendererState;
}

export interface UiPresence {
  type: "presence";
  protocol: typeof UI_PROTOCOL_NAME;
  protocolVersion: number;
  appInstanceId: string;
  viewId: string;
  members: UiPresenceMember[];
}

export interface UiErrorMessage {
  type: "error";
  protocol: typeof UI_PROTOCOL_NAME;
  protocolVersion: number;
  code: string;
  message: string;
}

export type UiMessage =
  | UiAttach
  | UiAttached
  | UiSnapshot
  | UiEvent
  | UiAck
  | UiLifecycle
  | UiRequestSnapshot
  | UiPresence
  | UiErrorMessage;

/** Selects the highest protocol version shared with this wrapper. */
export function negotiateUiProtocolVersion(minimum: number, maximum: number): number | undefined {
  if (!Number.isSafeInteger(minimum) || !Number.isSafeInteger(maximum)
    || minimum < 1 || minimum > maximum || maximum > 4_294_967_295) return undefined;
  const sharedMinimum = Math.max(minimum, UI_PROTOCOL_MIN_VERSION);
  const sharedMaximum = Math.min(maximum, UI_PROTOCOL_MAX_VERSION);
  return sharedMinimum <= sharedMaximum ? sharedMaximum : undefined;
}

/**
 * Decodes a known message and intentionally ignores unknown object fields for
 * forward compatibility. Unknown message, component, action, and value kinds
 * remain errors because their semantics cannot be inferred safely.
 */
export function decodeUiMessage(input: string | unknown): UiMessage {
  const value: unknown = typeof input === "string" ? JSON.parse(input) : input;
  const message = record(value, "UI message");
  if (message.protocol !== UI_PROTOCOL_NAME) {
    throw new Error(`Unexpected UI protocol ${String(message.protocol)}`);
  }
  const messageType = message.type;
  if (messageType !== "attach") requireSupportedProtocolVersion(message.protocolVersion);

  switch (messageType) {
    case "attach":
      validateProtocolRange(
        message.minProtocolVersion,
        message.maxProtocolVersion,
        "attach",
      );
      requireString(message.authToken, "attach.authToken");
      if (message.authToken.length > 4_096) {
        throw new Error("attach.authToken must contain at most 4096 characters");
      }
      validateParticipant(message.participant, "attach.participant");
      requireIdentifier(message.clientId, "attach.clientId");
      validateRenderer(message.renderer, "attach.renderer");
      requireIdentifier(message.viewId, "attach.viewId");
      if (message.expectedAppInstanceId !== undefined) {
        requireIdentifier(message.expectedAppInstanceId, "attach.expectedAppInstanceId");
      }
      if (message.lastSeenRevision !== undefined) {
        requireSafeInteger(message.lastSeenRevision, "attach.lastSeenRevision");
      }
      if (message.state !== undefined) {
        validateRendererState(message.state, "attach.state");
      }
      break;
    case "attached": {
      validateProtocolRange(
        message.minProtocolVersion,
        message.maxProtocolVersion,
        "attached",
      );
      const selectedVersion = requireSupportedProtocolVersion(message.protocolVersion);
      if (selectedVersion < Number(message.minProtocolVersion)
        || selectedVersion > Number(message.maxProtocolVersion)) {
        throw new Error("attached.protocolVersion is outside the advertised server range");
      }
      const app = record(message.app, "attached.app");
      requireString(app.id, "attached.app.id");
      requireString(app.name, "attached.app.name");
      requireString(app.version, "attached.app.version");
      if (app.description !== undefined) {
        requireString(app.description, "attached.app.description", true);
      }
      validateRoute(message, "attached", true);
      requireIdentifier(message.participantId, "attached.participantId");
      if (typeof message.resumed !== "boolean") {
        throw new Error("attached.resumed must be a boolean");
      }
      if (message.currentRevision !== undefined) {
        requireSafeInteger(message.currentRevision, "attached.currentRevision");
      }
      break;
    }
    case "snapshot": {
      requireIdentifier(message.appInstanceId, "snapshot.appInstanceId");
      requireIdentifier(message.clientId, "snapshot.clientId");
      requireIdentifier(message.viewId, "snapshot.viewId");
      requireSafeInteger(message.revision, "snapshot.revision");
      const root = record(message.root, "snapshot.root");
      requireIdentifier(root.id, "snapshot.root.id");
      if (root.type !== "markdownEditor") {
        throw new Error(`Unsupported UI component ${String(root.type)}`);
      }
      requireString(root.text, "markdownEditor.text", true);
      validateSelection(root.selection, "markdownEditor.selection");
      validateSelectionInText(root.text, root.selection, "markdownEditor.selection");
      if (root.presentation !== undefined
        && !["source", "preview", "split"].includes(String(root.presentation))) {
        throw new Error(`Unsupported Markdown presentation ${String(root.presentation)}`);
      }
      for (const field of ["readOnly", "dirty"] as const) {
        if (root[field] !== undefined && typeof root[field] !== "boolean") {
          throw new Error(`markdownEditor.${field} must be a boolean`);
        }
      }
      for (const field of ["placeholder", "title"] as const) {
        if (root[field] !== undefined) {
          requireString(root[field], `markdownEditor.${field}`, true);
        }
      }
      if (root.actions !== undefined) {
        const actions = record(root.actions, "markdownEditor.actions");
        for (const field of [
          "replaceRange",
          "setSelection",
          "save",
          "undo",
          "redo",
          "setPresentation",
        ]) {
          if (actions[field] !== undefined) {
            requireIdentifier(actions[field], `markdownEditor.actions.${field}`);
          }
        }
      }
      break;
    }
    case "event":
      validateRoute(message, "event", true);
      requireIdentifier(message.participantId, "event.participantId");
      requireIdentifier(message.eventId, "event.eventId");
      requireSafeInteger(message.baseRevision, "event.baseRevision");
      validateAction(message, "event");
      break;
    case "ack":
      validateRoute(message, "ack", true);
      requireIdentifier(message.eventId, "ack.eventId");
      requireSafeInteger(message.revision, "ack.revision");
      if (!["pending", "applied", "rejected", "stale"].includes(String(message.status))) {
        throw new Error(`Unsupported ack status ${String(message.status)}`);
      }
      if (message.message !== undefined) {
        requireString(message.message, "ack.message", true);
      }
      break;
    case "lifecycle":
      validateRoute(message, "lifecycle", true);
      validateRendererState(message.state, "lifecycle.state");
      break;
    case "requestSnapshot":
      validateRoute(message, "requestSnapshot", true);
      break;
    case "presence": {
      requireIdentifier(message.appInstanceId, "presence.appInstanceId");
      requireIdentifier(message.viewId, "presence.viewId");
      if (!Array.isArray(message.members)) {
        throw new Error("presence.members must be an array");
      }
      for (const [index, memberValue] of message.members.entries()) {
        const member = record(memberValue, `presence.members[${index}]`);
        validateParticipant(member.participant, `presence.members[${index}].participant`);
        requireIdentifier(member.clientId, `presence.members[${index}].clientId`);
        validateRenderer(member.renderer, `presence.members[${index}].renderer`);
        validateRendererState(member.state, `presence.members[${index}].state`);
      }
      break;
    }
    case "error":
      requireIdentifier(message.code, "error.code");
      requireString(message.message, "error.message");
      break;
    default:
      throw new Error(`Unsupported UI message ${String(message.type)}`);
  }
  return value as UiMessage;
}

export function uiAction(
  nodeId: string,
  action: string,
  kind: UiEventKind,
  value: UiEventValue = { type: "none" },
): UiAction {
  return { nodeId, action, kind, value };
}

export function uiEvent(
  snapshot: UiSnapshot,
  participantId: string,
  rendererId: string,
  action: UiAction,
  eventId = newEventId(),
): UiEvent {
  return {
    type: "event",
    protocol: UI_PROTOCOL_NAME,
    protocolVersion: snapshot.protocolVersion,
    appInstanceId: snapshot.appInstanceId,
    participantId,
    clientId: snapshot.clientId,
    rendererId,
    viewId: snapshot.viewId,
    eventId,
    baseRevision: snapshot.revision,
    ...action,
  };
}

export function newEventId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `event-${Date.now().toString(16)}-${Math.random().toString(16).slice(2)}`;
}

function validateAction(value: Record<string, unknown>, path: string): void {
  requireIdentifier(value.nodeId, `${path}.nodeId`);
  requireIdentifier(value.action, `${path}.action`);
  if (!["activate", "select", "change", "submit", "cancel", "command"].includes(String(value.kind))) {
    throw new Error(`Unsupported event kind ${String(value.kind)}`);
  }
  validateEventValue(value.value, `${path}.value`);
}

function validateRoute(
  value: Record<string, unknown>,
  path: string,
  includeRenderer: boolean,
): void {
  requireIdentifier(value.appInstanceId, `${path}.appInstanceId`);
  requireIdentifier(value.clientId, `${path}.clientId`);
  if (includeRenderer) requireIdentifier(value.rendererId, `${path}.rendererId`);
  requireIdentifier(value.viewId, `${path}.viewId`);
}

function validateParticipant(value: unknown, path: string): void {
  const participant = record(value, path);
  requireIdentifier(participant.id, `${path}.id`);
  if (participant.displayName !== undefined) {
    requireString(participant.displayName, `${path}.displayName`);
  }
  if (participant.color !== undefined) {
    requireString(participant.color, `${path}.color`, true);
  }
  if (participant.grants !== undefined) {
    if (!Array.isArray(participant.grants)) {
      throw new Error(`${path}.grants must be an array`);
    }
    for (const [index, grant] of participant.grants.entries()) {
      if (grant !== "*") requireIdentifier(grant, `${path}.grants[${index}]`);
    }
  }
}

function validateRenderer(value: unknown, path: string): void {
  const renderer = record(value, path);
  requireIdentifier(renderer.id, `${path}.id`);
  requireIdentifier(renderer.kind, `${path}.kind`);
  if (renderer.capabilities !== undefined) {
    if (!Array.isArray(renderer.capabilities)) {
      throw new Error(`${path}.capabilities must be an array`);
    }
    for (const [index, capability] of renderer.capabilities.entries()) {
      requireIdentifier(capability, `${path}.capabilities[${index}]`);
    }
  }
}

function validateRendererState(value: unknown, path: string): void {
  const state = record(value, path);
  if (typeof state.rendererVisible !== "boolean" || typeof state.terminalVisible !== "boolean") {
    throw new Error(`${path} must contain boolean visibility fields`);
  }
}

function validateSelection(value: unknown, path: string): void {
  const selection = record(value, path);
  validatePosition(selection.anchor, `${path}.anchor`);
  validatePosition(selection.head, `${path}.head`);
}

function validatePosition(value: unknown, path: string): void {
  const position = record(value, path);
  requireSafeInteger(position.line, `${path}.line`);
  requireSafeInteger(position.utf16Column, `${path}.utf16Column`);
  if (position.line > 4_294_967_295 || position.utf16Column > 4_294_967_295) {
    throw new Error(`${path} exceeds the cross-renderer text position range`);
  }
}

function validateSelectionInText(text: string, value: unknown, path: string): void {
  const selection = record(value, path);
  validatePositionInText(text, selection.anchor, `${path}.anchor`);
  validatePositionInText(text, selection.head, `${path}.head`);
}

function validatePositionInText(text: string, value: unknown, path: string): void {
  const position = record(value, path);
  const line = position.line as number;
  const column = position.utf16Column as number;
  const textLine = text.split("\n")[line];
  if (textLine === undefined || column > textLine.length) {
    throw new Error(`${path} is outside the Markdown document`);
  }
  if (column > 0 && column < textLine.length) {
    const previous = textLine.charCodeAt(column - 1);
    const next = textLine.charCodeAt(column);
    if (previous >= 0xD800 && previous <= 0xDBFF
      && next >= 0xDC00 && next <= 0xDFFF) {
      throw new Error(`${path} splits a UTF-16 surrogate pair`);
    }
  }
}

function validateEventValue(value: unknown, path: string): void {
  const eventValue = record(value, path);
  switch (eventValue.type) {
    case "none":
      return;
    case "bool":
      if (typeof eventValue.value !== "boolean") {
        throw new Error(`${path}.value must be a boolean`);
      }
      return;
    case "index":
      requireSafeInteger(eventValue.value, `${path}.value`);
      return;
    case "integer":
      if (!Number.isSafeInteger(eventValue.value)) {
        throw new Error(`${path}.value must be a safe integer`);
      }
      return;
    case "number":
      if (typeof eventValue.value !== "number" || !Number.isFinite(eventValue.value)) {
        throw new Error(`${path}.value must be a finite number`);
      }
      return;
    case "text":
      requireString(eventValue.value, `${path}.value`, true);
      return;
    case "textList":
      if (!Array.isArray(eventValue.value)
        || eventValue.value.some((item) => typeof item !== "string")) {
        throw new Error(`${path}.value must be an array of strings`);
      }
      return;
    case "textEdit": {
      const edit = record(eventValue.value, `${path}.value`);
      const range = record(edit.range, `${path}.value.range`);
      validatePosition(range.start, `${path}.value.range.start`);
      validatePosition(range.end, `${path}.value.range.end`);
      requireString(edit.text, `${path}.value.text`, true);
      const start = record(range.start, `${path}.value.range.start`);
      const end = record(range.end, `${path}.value.range.end`);
      if ((start.line as number) > (end.line as number)
        || (start.line === end.line
          && (start.utf16Column as number) > (end.utf16Column as number))) {
        throw new Error(`${path}.value.range must not be reversed`);
      }
      return;
    }
    case "textSelection":
      validateSelection(eventValue.value, `${path}.value`);
      return;
    default:
      throw new Error(`Unsupported event value ${String(eventValue.type)}`);
  }
}

function record(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${path} must be an object`);
  }
  return value as Record<string, unknown>;
}

function requireSupportedProtocolVersion(value: unknown): number {
  requireSafeInteger(value, "protocolVersion");
  if (value < UI_PROTOCOL_MIN_VERSION || value > UI_PROTOCOL_MAX_VERSION) {
    throw new Error(`Unsupported UI protocol version ${String(value)}`);
  }
  return value;
}

function validateProtocolRange(minimum: unknown, maximum: unknown, path: string): void {
  requireSafeInteger(minimum, `${path}.minProtocolVersion`);
  requireSafeInteger(maximum, `${path}.maxProtocolVersion`);
  if (minimum < 1 || minimum > maximum || maximum > 4_294_967_295) {
    throw new Error(`${path} contains an invalid UI protocol version range`);
  }
}

function requireString(
  value: unknown,
  path: string,
  allowEmpty = false,
): asserts value is string {
  if (typeof value !== "string" || (!allowEmpty && value.length === 0)) {
    throw new Error(`${path} must be ${allowEmpty ? "a string" : "a non-empty string"}`);
  }
}

function requireSafeInteger(value: unknown, path: string): asserts value is number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new Error(`${path} must be a non-negative safe integer`);
  }
}

function requireIdentifier(value: unknown, path: string): asserts value is string {
  requireString(value, path);
  if (value.length > 256 || !/^[A-Za-z0-9._:/-]+$/.test(value)) {
    throw new Error(`${path} must be a portable identifier`);
  }
}
