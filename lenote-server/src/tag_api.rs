use crate::database;
use crate::AppState;
use actix_web::{web, HttpResponse, Result as WebResult};
use lenote_common::models::*;
use regex::Regex;
use std::collections::HashSet;
use std::ops::Deref;

const TAG_COLORS: [&'static str; 6] = [
    "#34495e", "#8e44ad", "#27ae60", "#3498db", "#c0392b", "#f1c40f",
];

fn parse_tags(text: &str) -> HashSet<String> {
    lazy_static! {
        static ref TAGS_RE: Regex = Regex::new(r"#[a-zA-Z0-9\-_]+").unwrap();
    }

    let mut result = HashSet::new();

    for captures in TAGS_RE.captures_iter(text) {
        for c in captures.iter() {
            if let Some(match_) = c {
                result.insert(match_.as_str().to_ascii_lowercase());
            }
        }
    }

    return result;
}

pub fn save_tags_for_note<Conn: Deref<Target = rusqlite::Connection>>(
    mut note: Note,
    db: &mut Conn,
) -> Result<Note, failure::Error> {
    note.tags = parse_tags(&note.text);

    let tag_objs: Vec<Tag> = note
        .tags
        .iter()
        .map(|t| Tag {
            tag: t.clone(),
            color: TAG_COLORS[rand::random::<usize>() % TAG_COLORS.len()].to_string(),
            maps: vec![TagMap {
                note_id: note.id,
                status: TagMapStatus::Active,
                timestamp: note.timestamp,
            }],
        })
        .collect();

    database::save_tags(&tag_objs, db)?;
    Ok(note)
}

pub async fn http_get_tags(ctx: web::Data<AppState>) -> WebResult<HttpResponse> {
    let mut conn = ctx.db.lock().unwrap();
    let tags = database::get_tags(&mut conn)?;

    return Ok(HttpResponse::Ok().json(tags));
}

pub async fn http_get_tag_map(
    ctx: web::Data<AppState>,
    path: web::Path<(String,)>,
) -> WebResult<HttpResponse> {
    let tag = format!("#{}", path.0);
    let mut conn = ctx.db.lock().unwrap();
    let tag_map = database::get_tag_map(&mut conn, &tag)?;
    Ok(HttpResponse::Ok().json(tag_map))
}

pub async fn http_save_tag_map(
    ctx: web::Data<AppState>,
    ex: (web::Path<(String,)>, web::Json<TagMap>),
) -> WebResult<HttpResponse> {
    let path = ex.0;
    let tag_map = ex.1;
    let tag = format!("#{}", path.0);
    let mut conn = ctx.db.lock().unwrap();

    let mut tx = database::tx(&mut conn)?;
    database::save_tag_map(&mut tx, &tag, &tag_map)?;
    database::commit(tx)?;

    Ok(HttpResponse::Ok().json(tag_map.into_inner()))
}
