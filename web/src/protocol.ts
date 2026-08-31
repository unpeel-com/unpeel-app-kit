export const UI_PROTOCOL_NAME = "unpeel.ui" as const;
export const UI_PROTOCOL_MIN_VERSION = 1 as const;
export const UI_PROTOCOL_MAX_VERSION = 1 as const;
export const UI_PROTOCOL_VERSION = UI_PROTOCOL_MAX_VERSION;
export const UI_DELTA_CAPABILITY = "serverDelta" as const;
export const UI_MARKDOWN_EDITOR_CAPABILITY = "markdownEditor" as const;
export const UI_MEDIA_CAPABILITY = "media" as const;
export const UI_COMPONENT_CAPABILITIES = [
  UI_MARKDOWN_EDITOR_CAPABILITY,
  UI_MEDIA_CAPABILITY,
] as const;
export const MAX_INLINE_MEDIA_BYTES = 256 * 1024;

export interface AppMetadata {
  id: string;
  name: string;
  version: string;
  description?: string;
}

export interface UiParticipant {
  id: string;
  kind?: "human" | "agent" | "service";
  sourceSessionId?: string;
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

/** Scoped local attachment. Never expose its participantToken to browser code. */
export interface UiAttach {
  type: "attach";
  protocol: typeof UI_PROTOCOL_NAME;
  minProtocolVersion: number;
  maxProtocolVersion: number;
  participantToken: string;
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

export type MediaFit = "contain" | "cover" | "fill";

export interface MediaPathSource {
  kind: "path";
  path: string;
}

export interface MediaInlineSource {
  kind: "inline";
  mediaType: string;
  base64: string;
}

export interface MediaBlobSource {
  kind: "blob";
  sha256: string;
  mediaType: string;
  byteLength: number;
}

export type MediaSource = MediaPathSource | MediaInlineSource | MediaBlobSource;

export interface MediaPixelSize {
  w: number;
  h: number;
}

export interface MediaCellSize {
  w?: number;
  h?: number;
}

export interface MediaPointSize {
  w?: number;
  h?: number;
}

export interface MediaNode {
  id: string;
  type: "media";
  source: MediaSource;
  intrinsic: MediaPixelSize;
  cells?: MediaCellSize;
  points?: MediaPointSize;
  fit?: MediaFit;
  alt: string;
  activate?: string;
}

/** Opaque root retained only so the session can request terminal fallback. */
export interface UnsupportedUiNode {
  id: string;
  type: string;
  [field: string]: unknown;
}

export type UiNode = MarkdownEditorNode | MediaNode | UnsupportedUiNode;

export function isMarkdownEditorNode(node: UiNode): node is MarkdownEditorNode {
  return node.type === "markdownEditor";
}

export function isMediaNode(node: UiNode): node is MediaNode {
  return node.type === "media";
}

/** Capability required for a known root, or undefined for an unknown kind. */
export function uiNodeCapability(node: UiNode): string | undefined {
  if (isMarkdownEditorNode(node)) return UI_MARKDOWN_EDITOR_CAPABILITY;
  if (isMediaNode(node)) return UI_MEDIA_CAPABILITY;
  return undefined;
}

/** Filesystem paths must be translated by the Host before entering a browser. */
export function isBrowserSafeUiNode(node: UiNode): boolean {
  return !isMediaNode(node) || node.source.kind !== "path";
}

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

export type UiDeltaOperation =
  | { op: "replaceRoot"; root: UiNode }
  | { op: "markdownReplaceRange"; nodeId: string; edit: TextEdit }
  | { op: "markdownSetSelection"; nodeId: string; selection: TextSelection }
  | { op: "markdownSetPresentation"; nodeId: string; presentation: MarkdownPresentation }
  | { op: "markdownSetDirty"; nodeId: string; dirty: boolean }
  | { op: "markdownSetReadOnly"; nodeId: string; readOnly: boolean }
  | { op: "markdownSetTitle"; nodeId: string; title: string | null }
  | { op: "markdownSetPlaceholder"; nodeId: string; placeholder: string }
  | { op: "markdownSetActions"; nodeId: string; actions: MarkdownEditorActions }
  | {
    op: "mediaSetSource";
    nodeId: string;
    source: MediaSource;
    intrinsic: MediaPixelSize;
  };

export interface UiDelta {
  type: "delta";
  protocol: typeof UI_PROTOCOL_NAME;
  protocolVersion: number;
  appInstanceId: string;
  clientId: string;
  viewId: string;
  baseRevision: number;
  revision: number;
  operations: UiDeltaOperation[];
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
  | UiDelta
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
 * forward compatibility. Unknown component roots remain opaque so a session
 * can keep its attachment and expose the complete terminal pane. Unknown
 * message, action, and value kinds remain errors.
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
      requireString(message.participantToken, "attach.participantToken");
      if (message.participantToken.length > 16_384) {
        throw new Error("attach.participantToken must contain at most 16384 characters");
      }
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
      validateNode(message.root, "snapshot.root");
      break;
    }
    case "delta": {
      requireIdentifier(message.appInstanceId, "delta.appInstanceId");
      requireIdentifier(message.clientId, "delta.clientId");
      requireIdentifier(message.viewId, "delta.viewId");
      requireSafeInteger(message.baseRevision, "delta.baseRevision");
      requireSafeInteger(message.revision, "delta.revision");
      if (message.revision <= message.baseRevision) {
        throw new Error("delta.revision must be greater than baseRevision");
      }
      if (!Array.isArray(message.operations)
        || message.operations.length === 0
        || message.operations.length > 4_096) {
        throw new Error("delta.operations must contain 1..=4096 entries");
      }
      for (const [index, operation] of message.operations.entries()) {
        validateDeltaOperation(operation, `delta.operations[${index}]`);
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

/** Applies a contiguous server delta and returns the next complete snapshot. */
export function applyUiDelta(snapshot: UiSnapshot, delta: UiDelta): UiSnapshot {
  if (snapshot.protocol !== delta.protocol
    || snapshot.protocolVersion !== delta.protocolVersion
    || snapshot.appInstanceId !== delta.appInstanceId
    || snapshot.clientId !== delta.clientId
    || snapshot.viewId !== delta.viewId) {
    throw new Error("Delta route does not match the current snapshot");
  }
  if (snapshot.revision !== delta.baseRevision || delta.revision <= delta.baseRevision) {
    throw new Error("Delta is not contiguous with the current snapshot");
  }
  if (delta.operations.length === 0 || delta.operations.length > 4_096) {
    throw new Error("Delta must contain 1..=4096 operations");
  }

  let root = snapshot.root;
  for (const operation of delta.operations) {
    root = applyDeltaOperation(root, operation);
  }
  validateNode(root, "delta.result.root");
  return {
    type: "snapshot",
    protocol: delta.protocol,
    protocolVersion: delta.protocolVersion,
    appInstanceId: delta.appInstanceId,
    clientId: delta.clientId,
    viewId: delta.viewId,
    revision: delta.revision,
    root,
  };
}

function applyDeltaOperation(root: UiNode, operation: UiDeltaOperation): UiNode {
  if (operation.op === "replaceRoot") return operation.root;
  if (operation.op === "mediaSetSource") {
    if (root.id !== operation.nodeId || !isMediaNode(root)) {
      throw new Error("Delta targets an unavailable Media node");
    }
    return {
      ...root,
      source: operation.source,
      intrinsic: operation.intrinsic,
    };
  }
  if (root.id !== operation.nodeId || !isMarkdownEditorNode(root)) {
    throw new Error("Delta targets an unavailable Markdown node");
  }
  switch (operation.op) {
    case "markdownReplaceRange": {
      const start = utf16PositionOffset(root.text, operation.edit.range.start);
      const end = utf16PositionOffset(root.text, operation.edit.range.end);
      if (start > end) throw new Error("Markdown text edit range is reversed");
      return {
        ...root,
        text: root.text.slice(0, start) + operation.edit.text + root.text.slice(end),
      };
    }
    case "markdownSetSelection":
      return { ...root, selection: operation.selection };
    case "markdownSetPresentation":
      return { ...root, presentation: operation.presentation };
    case "markdownSetDirty":
      return { ...root, dirty: operation.dirty };
    case "markdownSetReadOnly":
      return { ...root, readOnly: operation.readOnly };
    case "markdownSetTitle": {
      const { title: _oldTitle, ...withoutTitle } = root;
      return operation.title === null
        ? withoutTitle
        : { ...withoutTitle, title: operation.title };
    }
    case "markdownSetPlaceholder":
      return { ...root, placeholder: operation.placeholder };
    case "markdownSetActions":
      return { ...root, actions: operation.actions };
    default: {
      const unreachable: never = operation;
      throw new Error(`Unsupported delta operation ${String(unreachable)}`);
    }
  }
}

function utf16PositionOffset(text: string, position: TextPosition): number {
  const lines = text.split("\n");
  const line = lines[position.line];
  if (line === undefined || position.utf16Column < 0 || position.utf16Column > line.length) {
    throw new Error("Markdown text position is outside the document");
  }
  validatePositionInText(text, position, "delta.textPosition");
  let offset = position.utf16Column;
  for (let index = 0; index < position.line; index += 1) {
    offset += lines[index]!.length + 1;
  }
  return offset;
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
  if (participant.kind !== undefined
    && !["human", "agent", "service"].includes(String(participant.kind))) {
    throw new Error(`${path}.kind is unsupported`);
  }
  if (participant.sourceSessionId !== undefined) {
    requireIdentifier(participant.sourceSessionId, `${path}.sourceSessionId`);
  }
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

function validateNode(value: unknown, path: string): void {
  const root = record(value, path);
  requireIdentifier(root.id, `${path}.id`);
  requireIdentifier(root.type, `${path}.type`);
  if (root.type === "media") {
    validateMediaNode(root, path);
    return;
  }
  if (root.type !== "markdownEditor") {
    return;
  }
  requireString(root.text, `${path}.text`, true);
  validateSelection(root.selection, `${path}.selection`);
  validateSelectionInText(root.text, root.selection, `${path}.selection`);
  if (root.presentation !== undefined
    && !["source", "preview", "split"].includes(String(root.presentation))) {
    throw new Error(`Unsupported Markdown presentation ${String(root.presentation)}`);
  }
  for (const field of ["readOnly", "dirty"] as const) {
    if (root[field] !== undefined && typeof root[field] !== "boolean") {
      throw new Error(`${path}.${field} must be a boolean`);
    }
  }
  for (const field of ["placeholder", "title"] as const) {
    if (root[field] !== undefined) requireString(root[field], `${path}.${field}`, true);
  }
  if (root.actions !== undefined) validateMarkdownActions(root.actions, `${path}.actions`);
}

function validateMediaNode(root: Record<string, unknown>, path: string): void {
  validateMediaSource(root.source, `${path}.source`);
  validateMediaPixelSize(root.intrinsic, `${path}.intrinsic`);
  if (root.cells !== undefined) {
    validateMediaOptionalSize(root.cells, `${path}.cells`, 65_535);
  }
  if (root.points !== undefined) {
    validateMediaOptionalSize(root.points, `${path}.points`, 4_294_967_295);
  }
  if (root.fit !== undefined && !["contain", "cover", "fill"].includes(String(root.fit))) {
    throw new Error(`${path}.fit is unsupported`);
  }
  requireString(root.alt, `${path}.alt`, true);
  if (new TextEncoder().encode(root.alt as string).length > 16_384) {
    throw new Error(`${path}.alt must contain at most 16384 bytes`);
  }
  if (root.activate !== undefined) requireIdentifier(root.activate, `${path}.activate`);
}

function validateMediaSource(value: unknown, path: string): void {
  const source = record(value, path);
  switch (source.kind) {
    case "path":
      requireString(source.path, `${path}.path`);
      if (new TextEncoder().encode(source.path as string).length > 4_096
        || (source.path as string).includes("\0")) {
        throw new Error(`${path}.path must contain 1..=4096 non-NUL bytes`);
      }
      return;
    case "inline":
      validateImageMediaType(source.mediaType, `${path}.mediaType`);
      requireString(source.base64, `${path}.base64`);
      if (decodedBase64Length(source.base64 as string) > MAX_INLINE_MEDIA_BYTES) {
        throw new Error(`${path}.base64 exceeds the 256 KiB decoded limit`);
      }
      return;
    case "blob":
      requireString(source.sha256, `${path}.sha256`);
      if (!/^[0-9a-f]{64}$/.test(source.sha256 as string)) {
        throw new Error(`${path}.sha256 must be lowercase SHA-256 hex`);
      }
      validateImageMediaType(source.mediaType, `${path}.mediaType`);
      requireSafeInteger(source.byteLength, `${path}.byteLength`);
      if (source.byteLength === 0) throw new Error(`${path}.byteLength must be positive`);
      return;
    default:
      throw new Error(`Unsupported Media source ${String(source.kind)}`);
  }
}

function validateMediaPixelSize(value: unknown, path: string): void {
  const size = record(value, path);
  for (const axis of ["w", "h"] as const) {
    requireSafeInteger(size[axis], `${path}.${axis}`);
    if (size[axis] === 0 || (size[axis] as number) > 4_294_967_295) {
      throw new Error(`${path}.${axis} must be a positive UInt32`);
    }
  }
}

function validateMediaOptionalSize(value: unknown, path: string, maximum: number): void {
  const size = record(value, path);
  if (size.w === undefined && size.h === undefined) {
    throw new Error(`${path} must contain at least one axis`);
  }
  for (const axis of ["w", "h"] as const) {
    if (size[axis] === undefined) continue;
    requireSafeInteger(size[axis], `${path}.${axis}`);
    if (size[axis] === 0 || (size[axis] as number) > maximum) {
      throw new Error(`${path}.${axis} is outside the supported range`);
    }
  }
}

function validateImageMediaType(value: unknown, path: string): void {
  requireString(value, path);
  if ((value as string).length > 127
    || !/^image\/[A-Za-z0-9!#$&^_.+/-]+$/.test(value as string)) {
    throw new Error(`${path} must be a portable image MIME type`);
  }
}

function decodedBase64Length(value: string): number {
  if (value.length === 0 || value.length > 349_528 || value.length % 4 !== 0
    || !/^[A-Za-z0-9+/]*={0,2}$/.test(value)) {
    throw new Error("Media inline base64 is invalid");
  }
  const decoded = atob(value);
  if (btoa(decoded) !== value) throw new Error("Media inline base64 is non-canonical");
  const length = decoded.length;
  if (length <= 0) throw new Error("Media inline base64 is empty");
  return length;
}

function validateMarkdownActions(value: unknown, path: string): void {
  const actions = record(value, path);
  for (const field of [
    "replaceRange",
    "setSelection",
    "save",
    "undo",
    "redo",
    "setPresentation",
  ]) {
    if (actions[field] !== undefined) requireIdentifier(actions[field], `${path}.${field}`);
  }
}

function validateDeltaOperation(value: unknown, path: string): void {
  const operation = record(value, path);
  switch (operation.op) {
    case "replaceRoot":
      validateNode(operation.root, `${path}.root`);
      return;
    case "markdownReplaceRange": {
      requireIdentifier(operation.nodeId, `${path}.nodeId`);
      const edit = record(operation.edit, `${path}.edit`);
      const range = record(edit.range, `${path}.edit.range`);
      validatePosition(range.start, `${path}.edit.range.start`);
      validatePosition(range.end, `${path}.edit.range.end`);
      requireString(edit.text, `${path}.edit.text`, true);
      const start = record(range.start, `${path}.edit.range.start`);
      const end = record(range.end, `${path}.edit.range.end`);
      if ((start.line as number) > (end.line as number)
        || (start.line === end.line
          && (start.utf16Column as number) > (end.utf16Column as number))) {
        throw new Error(`${path}.edit.range must not be reversed`);
      }
      return;
    }
    case "markdownSetSelection":
      requireIdentifier(operation.nodeId, `${path}.nodeId`);
      validateSelection(operation.selection, `${path}.selection`);
      return;
    case "markdownSetPresentation":
      requireIdentifier(operation.nodeId, `${path}.nodeId`);
      if (!["source", "preview", "split"].includes(String(operation.presentation))) {
        throw new Error(`${path}.presentation is unsupported`);
      }
      return;
    case "markdownSetDirty":
      requireIdentifier(operation.nodeId, `${path}.nodeId`);
      if (typeof operation.dirty !== "boolean") throw new Error(`${path}.dirty must be boolean`);
      return;
    case "markdownSetReadOnly":
      requireIdentifier(operation.nodeId, `${path}.nodeId`);
      if (typeof operation.readOnly !== "boolean") {
        throw new Error(`${path}.readOnly must be boolean`);
      }
      return;
    case "markdownSetTitle":
      requireIdentifier(operation.nodeId, `${path}.nodeId`);
      if (operation.title !== null) requireString(operation.title, `${path}.title`, true);
      return;
    case "markdownSetPlaceholder":
      requireIdentifier(operation.nodeId, `${path}.nodeId`);
      requireString(operation.placeholder, `${path}.placeholder`, true);
      return;
    case "markdownSetActions":
      requireIdentifier(operation.nodeId, `${path}.nodeId`);
      validateMarkdownActions(operation.actions, `${path}.actions`);
      return;
    case "mediaSetSource":
      requireIdentifier(operation.nodeId, `${path}.nodeId`);
      validateMediaSource(operation.source, `${path}.source`);
      validateMediaPixelSize(operation.intrinsic, `${path}.intrinsic`);
      return;
    default:
      throw new Error(`Unsupported delta operation ${String(operation.op)}`);
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
