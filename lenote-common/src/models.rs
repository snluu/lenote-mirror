use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum NoteType {
    Text = 0,
    Image = 1,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Note {
    pub id: i64,
    pub client_id: String,
    pub text: String,
    pub timestamp: i64,
    pub note_type: NoteType,
    pub tags: HashSet<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum TagMapStatus {
    Active = 0,
    Archived = 1,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagMap {
    pub note_id: i64,
    pub status: TagMapStatus,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
    pub tag: String,
    pub color: String,
    pub maps: Vec<TagMap>,
}

impl TagMapStatus {
    pub fn from(x: i32) -> anyhow::Result<Self> {
        match x {
            0 => Ok(Self::Active),
            1 => Ok(Self::Archived),
            _ => Err(anyhow!("Cannot convert value {} to TagMapStatus", x)),
        }
    }
}

impl NoteType {
    pub fn from(x: i32) -> anyhow::Result<Self> {
        match x {
            0 => Ok(Self::Text),
            1 => Ok(Self::Image),
            _ => Err(anyhow!("Cannot convert value {} to NoteType", x)),
        }
    }
}
