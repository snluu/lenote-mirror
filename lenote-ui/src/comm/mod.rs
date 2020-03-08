mod event_bus;

use lenote_common::models::Note;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NoteEvent {
    NoteSubmitted(Note),
    NoteSaved(Note),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TagEvent {
    TagsChanged,
}

pub type NoteEventBus = event_bus::EventBus<NoteEvent>;
pub type TagEventBus = event_bus::EventBus<TagEvent>;
