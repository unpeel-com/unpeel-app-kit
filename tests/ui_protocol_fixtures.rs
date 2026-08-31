use std::io::{BufReader, Cursor};

use unpeel_app_kit::{UiComponent, UiEventValue, UiMessage, read_ui_message};

const STREAM: &str = include_str!("../protocol/unpeel-ui-v1.ndjson");

#[test]
fn shared_v1_stream_decodes_and_validates_every_frame() {
    let mut reader = BufReader::new(Cursor::new(STREAM.as_bytes()));
    let mut messages = Vec::new();
    while let Some(message) = read_ui_message(&mut reader).unwrap() {
        messages.push(message);
    }

    assert_eq!(messages.len(), 11);
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
    let UiComponent::MarkdownEditor(editor) = &snapshot.root.element;
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
}
