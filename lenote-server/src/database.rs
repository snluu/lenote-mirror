use lenote_common::models::*;
use rusqlite::{params, NO_PARAMS};
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::ops::Deref;
use std::rc::Rc;
use std::time::SystemTime;

const EVOLUTIONS: [&'static str; 5] = [
    // Version 1
    "CREATE TABLE notes(
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        text VARCHAR NOT NULL,
        timestamp BIGINT
    )",
    // Version 2
    "CREATE TABLE tags(
        tag VARCHAR NOT NULL PRIMARY KEY,
        color VARCHAR NOT NULL
    )",
    // Version 3
    "CREATE TABLE tag_map(
        tag VARCHAR NOT NULL,
        note_id BIGINT NOT NULL,
        status INT NOT NULL,
        PRIMARY KEY (tag, note_id),
        FOREIGN KEY(note_id) REFERENCES notes(id),
        FOREIGN KEY(tag) REFERENCES tags(tag)
    )",
    // Version 4
    "CREATE TABLE tag_map_history(
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        tag VARCHAR NOT NULL,
        note_id BIGINT NOT NULL,
        status INT NOT NULL,
        timestamp BIGINT,
        FOREIGN KEY(note_id) REFERENCES notes(id),
        FOREIGN KEY(tag) REFERENCES tags(tag)
    )",
    // Version 5
    "ALTER TABLE notes ADD COLUMN note_type INTEGER NOT NULL DEFAULT 0",
];

fn now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[derive(Debug, failure::Fail)]
#[fail(display = "DB Error: {}", message)]
struct DBError {
    message: String,
}

fn init_version_table(conn: &mut rusqlite::Connection) -> Result<(), failure::Error> {
    conn.execute("CREATE TABLE IF NOT EXISTS db_version(v INT)", NO_PARAMS)?;
    conn.execute(
        "INSERT INTO db_version(v) SELECT 0 WHERE NOT EXISTS(SELECT 1 FROM db_version)",
        NO_PARAMS,
    )?;

    return Ok(());
}

fn get_db_version(conn: &mut rusqlite::Connection) -> Result<usize, failure::Error> {
    let version: i32 = conn.query_row("SELECT v FROM db_version", NO_PARAMS, |row| row.get(0))?;
    return Ok(version.try_into().unwrap());
}

fn evolve(
    conn: &mut rusqlite::Connection,
    query: &str,
    version: usize,
) -> Result<(), failure::Error> {
    let tx = conn.transaction()?;
    let version: i32 = version.try_into().unwrap();

    tx.execute(query, NO_PARAMS)?;
    let affected = tx.execute(
        "UPDATE db_version SET v = ?1 WHERE v = ?2",
        params![version, version - 1],
    )?;

    if affected != 1 {
        return Err(DBError {
            message: format!("Failed to evolve database to version {}", version),
        }
        .into());
    }

    tx.commit()?;

    println!("Evolved database to version {}", version);
    return Ok(());
}

fn evolve_all_versions(conn: &mut rusqlite::Connection) -> Result<(), failure::Error> {
    let db_version: usize = get_db_version(conn)?;
    let max_version = EVOLUTIONS.len();

    for v in db_version..max_version {
        evolve(conn, EVOLUTIONS[v], v + 1)?;
    }

    return Ok(());
}

pub fn init(conn: &mut rusqlite::Connection) -> Result<(), failure::Error> {
    init_version_table(conn)?;
    evolve_all_versions(conn)?;
    rusqlite::vtab::array::load_module(conn)?;
    return Ok(());
}

pub fn tx(conn: &mut rusqlite::Connection) -> Result<rusqlite::Transaction, failure::Error> {
    Ok(conn.transaction()?)
}

pub fn commit(tx: rusqlite::Transaction) -> Result<(), failure::Error> {
    Ok(tx.commit()?)
}

pub fn save_note<Conn: Deref<Target = rusqlite::Connection>>(
    note: &Note,
    conn: &mut Conn,
) -> Result<i64, failure::Error> {
    conn.execute(
        "INSERT INTO notes(text, timestamp, note_type) VALUES(?1, ?2, ?3)",
        params![&note.text, &note.timestamp, note.note_type as i32],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn get_tags_for_note<Conn: Deref<Target = rusqlite::Connection>>(
    note_ids: &[i64],
    conn: &mut Conn,
) -> Result<Vec<(i64, String)>, failure::Error> {
    let mut stmt = conn.prepare("SELECT note_id, tag FROM tag_map WHERE note_id IN rarray(?1)")?;

    let note_ids_param = note_ids
        .into_iter()
        .map(|i| rusqlite::types::Value::from(*i))
        .collect();
    let note_ids_ptr = Rc::new(note_ids_param);
    let tags_iter = stmt.query_map(params![&note_ids_ptr], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut result = vec![];
    for tag in tags_iter {
        result.push(tag?);
    }

    Ok(result)
}

pub fn get_notes<Conn: Deref<Target = rusqlite::Connection>>(
    conn: &mut Conn,
    min_id: i64,
    max_id: i64,
) -> Result<Vec<Note>, failure::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, text, timestamp, note_type FROM notes 
        WHERE id BETWEEN ?1 AND ?2
        ORDER BY id DESC LIMIT 500",
    )?;

    let note_iters = stmt.query_map(params![&min_id, &max_id], |row| {
        Ok(Note {
            id: row.get(0)?,
            client_id: String::from(""),
            text: row.get(1)?,
            timestamp: row.get(2)?,
            note_type: NoteType::from(row.get(3)?).unwrap(),
            tags: HashSet::new(),
        })
    })?;

    let mut result = vec![];
    let mut id_to_index = HashMap::new();
    let mut note_ids = vec![];
    for note in note_iters {
        result.push(note?);
        let note_id = result.last().unwrap().id;
        id_to_index.insert(note_id, result.len() - 1);
        note_ids.push(note_id);
    }

    drop(stmt);
    let tag_map = get_tags_for_note(&note_ids, conn)?;

    for tm in tag_map {
        let note_id = tm.0;
        let note = &mut result[id_to_index[&note_id]];
        note.tags.insert(tm.1);
    }

    result.reverse();

    return Ok(result);
}

pub fn save_tags<Conn: Deref<Target = rusqlite::Connection>>(
    tags: &Vec<Tag>,
    conn: &mut Conn,
) -> Result<(), failure::Error> {
    for tag in tags {
        conn.execute(
            "INSERT OR IGNORE INTO tags(tag, color) VALUES(?1, ?2)",
            params![&tag.tag, &tag.color,],
        )?;

        for map in &tag.maps {
            conn.execute(
                "INSERT INTO tag_map(tag, note_id, status) VALUES(?1, ?2, ?3)",
                params![&tag.tag, &map.note_id, map.status as i32],
            )?;
        }
    }

    Ok(())
}

pub fn get_tags<Conn: Deref<Target = rusqlite::Connection>>(
    conn: &mut Conn,
) -> Result<Vec<Tag>, failure::Error> {
    let mut stmt = conn.prepare(
        "SELECT t.tag, t.color, m.note_id, m.status, n.timestamp
        FROM tag_map m
            INNER JOIN tags t ON m.tag = t.tag
            INNER JOIN notes n ON m.note_id = n.id
        ORDER BY m.note_id DESC
        LIMIT 500",
    )?;

    let iter = stmt.query_map(NO_PARAMS, |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            TagMap {
                note_id: row.get(2)?,
                status: TagMapStatus::from(row.get(3)?).unwrap(),
                timestamp: row.get(4)?,
            },
        ))
    })?;

    let mut result = vec![];
    let mut tag_to_index = HashMap::new();
    for pair in iter {
        let pair = pair?;
        let index: usize = if let Some(x) = tag_to_index.get(&pair.0) {
            *x
        } else {
            result.push(Tag {
                tag: pair.0,
                color: pair.1,
                maps: vec![],
            });

            tag_to_index.insert(result.last().unwrap().tag.clone(), result.len() - 1);
            result.len() - 1
        };

        result[index].maps.push(pair.2);
    }

    result.sort_by(|a, b| {
        a.tag
            .partial_cmp(&b.tag)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(result)
}

pub fn get_tag_map<Conn: Deref<Target = rusqlite::Connection>>(
    conn: &mut Conn,
    tag: &str,
) -> Result<Vec<TagMap>, failure::Error> {
    let mut stmt = conn.prepare(
        "SELECT m.note_id, m.status, n.timestamp
        FROM tag_map m
            INNER JOIN tags t ON m.tag = t.tag
            INNER JOIN notes n ON m.note_id = n.id
        WHERE m.tag = ?1
        ORDER BY m.note_id DESC",
    )?;
    let iter = stmt.query_map(params![tag], |row| {
        Ok(TagMap {
            note_id: row.get(0)?,
            status: TagMapStatus::from(row.get::<_, i32>(1)?).unwrap(),
            timestamp: row.get(2)?,
        })
    })?;

    let mut result = vec![];
    for tag_map in iter {
        result.push(tag_map?);
    }

    Ok(result)
}

pub fn save_tag_map<Conn: Deref<Target = rusqlite::Connection>>(
    conn: &mut Conn,
    tag: &str,
    tag_map: &TagMap,
) -> Result<(), failure::Error> {
    info!("Updating tag map status");
    conn.execute(
        "UPDATE tag_map SET status = ?1 WHERE tag = ?2 AND note_id = ?3",
        params![tag_map.status as i32, tag, &tag_map.note_id],
    )?;

    info!("Saving tag map history");
    conn.execute(
        "INSERT INTO tag_map_history(tag, note_id, status, timestamp) VALUES(?1, ?2, ?3, ?4)",
        params![tag, &tag_map.note_id, tag_map.status as i32, &now()],
    )?;

    Ok(())
}
