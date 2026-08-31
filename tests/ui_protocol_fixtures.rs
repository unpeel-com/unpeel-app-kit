#![cfg(feature = "ui-bridge")]

use std::io::{BufReader, Cursor};

use unpeel_app_kit::{
    ButtonRole, CanvasControl, ListItemSlot, MediaSource, SurfaceInputPolicy, UiComponent,
    UiEventKind, UiEventValue, UiMessage, read_ui_message,
};

const STREAM: &str = include_str!("../protocol/unpeel-ui-v1.ndjson");

#[test]
fn shared_v1_stream_decodes_and_validates_every_frame() {
    let mut reader = BufReader::new(Cursor::new(STREAM.as_bytes()));
    let mut messages = Vec::new();
    while let Some(message) = read_ui_message(&mut reader).unwrap() {
        messages.push(message);
    }

    assert_eq!(messages.len(), 21);
    let UiMessage::Attach(attach) = &messages[0] else {
        panic!("first fixture must attach an authenticated participant");
    };
    assert!(attach.participant_token.starts_with("upui1."));
    let UiMessage::Attached(attached) = &messages[1] else {
        panic!("second fixture must acknowledge attachment");
    };
    assert!(attached.resumed);
    let UiMessage::Snapshot(snapshot) = &messages[2] else {
        panic!("third fixture must be a snapshot");
    };
    let UiComponent::MarkdownEditor(editor) = &snapshot.root.element else {
        panic!("third fixture must contain a Markdown editor");
    };
    assert_eq!(editor.text, "# Hello\n🙂 world");
    assert_eq!(editor.selection.head.utf16_column, 2);

    let UiMessage::Presence(presence) = &messages[3] else {
        panic!("fourth fixture must contain collaborative presence");
    };
    assert_eq!(presence.members.len(), 2);

    let UiMessage::Event(edit) = &messages[4] else {
        panic!("fifth fixture must be an event");
    };
    assert_eq!(edit.event_id.as_str(), "event-edit-1");
    assert!(matches!(edit.action.value, UiEventValue::TextEdit(_)));
    assert!(matches!(messages[5], UiMessage::Ack(_)));
    assert!(matches!(messages[6], UiMessage::Lifecycle(_)));
    assert!(matches!(messages[7], UiMessage::RequestSnapshot(_)));

    let UiMessage::Event(selection) = &messages[8] else {
        panic!("ninth fixture must be an event");
    };
    assert!(matches!(
        selection.action.value,
        UiEventValue::TextSelection(_)
    ));
    let UiMessage::Event(save) = &messages[9] else {
        panic!("tenth fixture must be an event");
    };
    assert_eq!(save.action.action.as_str(), "save");
    assert_eq!(save.action.value, UiEventValue::None);
    let UiMessage::Delta(delta) = &messages[10] else {
        panic!("eleventh fixture must be a server delta");
    };
    assert_eq!(delta.base_revision, 7);
    assert_eq!(delta.revision, 8);
    assert_eq!(delta.operations.len(), 3);

    let UiMessage::Snapshot(media_snapshot) = &messages[11] else {
        panic!("twelfth fixture must be a Media snapshot");
    };
    let UiComponent::Media(media) = &media_snapshot.root.element else {
        panic!("twelfth fixture must contain Media");
    };
    assert_eq!(media.alt, "Tiny fixture pixel");
    assert_eq!(media.resolved_points(), (40, 40));

    let UiMessage::Delta(media_delta) = &messages[12] else {
        panic!("thirteenth fixture must be a Media reference delta");
    };
    let updated = media_snapshot.applying(media_delta).unwrap();
    let UiComponent::Media(media) = updated.root.element else {
        panic!("Media delta must preserve the component kind");
    };
    assert!(matches!(
        media.source,
        MediaSource::Blob {
            byte_length: 68,
            ..
        }
    ));

    let UiMessage::Snapshot(todo_snapshot) = &messages[13] else {
        panic!("fourteenth fixture must be the canonical Todo Page");
    };
    let UiComponent::Page(page) = &todo_snapshot.root.element else {
        panic!("Todo fixture must contain Page");
    };
    assert_eq!(page.title, "Todos");
    assert_eq!(page.list().items.len(), 3);
    assert_eq!(page.list().items[0].label, "Run the standalone TUI");
    let Some(ListItemSlot::Toggle(toggle)) = &page.list().items[1].trailing else {
        panic!("Todo rows must expose their completion Toggle in a named slot");
    };
    assert!(!toggle.value);
    assert_eq!(
        page.input_spec().unwrap().submit.as_deref(),
        Some("add-todo")
    );

    let UiMessage::Delta(todo_delta) = &messages[14] else {
        panic!("fifteenth fixture must update Todo through a compact delta");
    };
    let updated = todo_snapshot.applying(todo_delta).unwrap();
    let UiComponent::Page(page) = updated.root.element else {
        panic!("Todo delta must preserve Page");
    };
    assert!(page.list().items[1].done);

    let UiMessage::Snapshot(usage_snapshot) = &messages[15] else {
        panic!("sixteenth fixture must contain the Usage master/detail Page");
    };
    let UiComponent::Page(page) = &usage_snapshot.root.element else {
        panic!("Usage fixture must contain Page");
    };
    assert_eq!(page.back.as_deref(), Some("close-provider"));
    assert_eq!(
        page.list().items[0].detail.as_deref(),
        Some("Resets in 6d 18h")
    );
    assert_eq!(page.list().items[0].value.as_deref(), Some("3% used"));
    assert_eq!(
        page.list().items[1].activate.as_deref(),
        Some("refresh-usage")
    );

    let UiMessage::Snapshot(surface_snapshot) = &messages[16] else {
        panic!("seventeenth fixture must contain the planet Surface");
    };
    let UiComponent::Surface(surface) = &surface_snapshot.root.element else {
        panic!("planet fixture must contain Surface");
    };
    assert_eq!(surface.reference.session_id, "terminal-9");
    assert_eq!(surface.reference.stream_id, "planets");
    assert_eq!(surface.input_policy, SurfaceInputPolicy::PointerAndKeyboard);

    let UiMessage::Delta(surface_delta) = &messages[17] else {
        panic!("eighteenth fixture must switch the Surface reference");
    };
    let updated = surface_snapshot.applying(surface_delta).unwrap();
    let UiComponent::Surface(surface) = updated.root.element else {
        panic!("Surface reference delta must preserve the component kind");
    };
    assert_eq!(surface.reference.stream_id, "planets-detail");

    let UiMessage::Snapshot(canvas_snapshot) = &messages[18] else {
        panic!("nineteenth fixture must contain CanvasPage");
    };
    let UiComponent::CanvasPage(canvas) = &canvas_snapshot.root.element else {
        panic!("canvas fixture must contain CanvasPage");
    };
    assert_eq!(canvas.title, "Planet Canvas");
    assert_eq!(canvas.surface.id, "planet-canvas");
    assert_eq!(canvas.surface.surface.reference.stream_id, "canvas-planets");
    assert_eq!(
        canvas.required_capabilities(),
        vec!["canvasPage", "surface", "button"]
    );
    let CanvasControl::Button(select) = &canvas.controls[2];
    assert_eq!(select.role, ButtonRole::Primary);

    let UiMessage::Event(canvas_event) = &messages[19] else {
        panic!("twentieth fixture must activate a Canvas Button");
    };
    assert_eq!(canvas_event.action.node_id.as_str(), "canvas-next");
    assert_eq!(canvas_event.action.kind, UiEventKind::Activate);
    assert_eq!(canvas_event.action.value, UiEventValue::None);

    let UiMessage::Delta(canvas_delta) = &messages[20] else {
        panic!("twenty-first fixture must switch the nested Surface reference");
    };
    let updated = canvas_snapshot.applying(canvas_delta).unwrap();
    let UiComponent::CanvasPage(canvas) = updated.root.element else {
        panic!("nested Surface delta must preserve CanvasPage");
    };
    assert_eq!(
        canvas.surface.surface.reference.stream_id,
        "canvas-planets-detail"
    );
}
