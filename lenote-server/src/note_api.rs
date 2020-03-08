use crate::database;
use crate::tag_api;
use crate::AppState;
use actix_web::{web, HttpResponse, Result as WebResult};
use lenote_common::models::*;
use regex::Regex;
use serde::Deserialize;
use std::time::SystemTime;
use tokio::fs::File;
use tokio::prelude::*;

const FILE_NAME_LENGTH: usize = 32;
const FILE_NAME_CHARS: &'static [char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

#[derive(Deserialize)]
pub struct GetNotesFilter {
    pub min_id: Option<i64>,
    pub max_id: Option<i64>,
}

fn gen_file_name() -> String {
    let mut ret = String::with_capacity(FILE_NAME_LENGTH);
    for _ in 0..ret.capacity() {
        ret.push(FILE_NAME_CHARS[rand::random::<usize>() % FILE_NAME_CHARS.len()]);
    }

    ret
}

async fn save_img_file(ctx: &AppState, note: &mut Note) -> Result<(), failure::Error> {
    lazy_static! {
        static ref DATA_URL_PREFIX: Regex = Regex::new(r"data:image/(?P<ext>.+);base64").unwrap();
    }

    if let Some(pos) = note.text.find(",") {
        if let Some(caps) = DATA_URL_PREFIX.captures(&note.text[..pos]) {
            let ext = caps.name("ext").unwrap().as_str();
            let file_name = gen_file_name();
            let file_path = ctx
                .config
                .data
                .join("res")
                .join("images")
                .join(&file_name)
                .with_extension(ext);

            let bin = base64::decode(&note.text[pos + 1..])?;
            info!(
                "Saving image ({} bytes) to path: {}",
                bin.len(),
                file_path.display()
            );
            let mut file = File::create(&file_path).await?;
            file.write_all(&bin).await?;

            note.text = format!("/res/images/{}.{}", file_name, ext);
        }

        Ok(())
    } else {
        Ok(())
    }
}

pub async fn http_save_note(
    ctx: web::Data<AppState>,
    mut req: web::Json<Note>,
) -> WebResult<HttpResponse> {
    req.timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut note = req.into_inner();
    let mut conn = ctx.db.lock().unwrap();
    let mut tx = database::tx(&mut conn)?;

    if note.note_type == NoteType::Image {
        save_img_file(&ctx, &mut note).await?;
    }

    note.id = database::save_note(&note, &mut tx)?;
    note = tag_api::save_tags_for_note(note, &mut tx)?;

    database::commit(tx)?;

    return Ok(HttpResponse::Ok().json(note));
}

pub async fn http_get_notes(
    ctx: web::Data<AppState>,
    filter: web::Query<GetNotesFilter>,
) -> WebResult<HttpResponse> {
    let mut conn = ctx.db.lock().unwrap();
    let notes = database::get_notes(
        &mut conn,
        filter.min_id.unwrap_or(1),
        filter.max_id.unwrap_or(std::i64::MAX - 1),
    )?;

    return Ok(HttpResponse::Ok().json(notes));
}
