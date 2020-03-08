#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

mod database;
mod note_api;
mod tag_api;

use actix_files as fs;
use actix_service::Service;
use actix_web::{http, web};
use actix_web::{App, HttpResponse, HttpServer, Result as WebResult};
use clap::Arg;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct AppConfig {
    ui: PathBuf,
    pages: PathBuf,
    data: PathBuf,
    slow: bool,
    port: String,
}

#[derive(Clone)]
pub struct AppState {
    config: AppConfig,
    db: Arc<Mutex<rusqlite::Connection>>,
}

fn get_config() -> AppConfig {
    let app = clap::App::new("lenote-server")
        .about("Lenote Server")
        .version("0.1.0")
        .arg(
            Arg::with_name("ui")
                .long("ui")
                .value_name("DIR")
                .required(true)
                .help("Path to the directory containing UI js & wasm script files")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("pages")
                .long("pages")
                .value_name("DIR")
                .required(true)
                .help("Path to the directory containing HTML & static files")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("data")
                .long("data")
                .value_name("DIR")
                .required(true)
                .help("Path to the directory containing the database and resource files")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .value_name("PORT")
                .default_value("8080")
                .help("Port for the HTTP server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("slow")
                .long("slow")
                .help("Slow down each request. Used for development purpose"),
        );

    let matches = app.get_matches();
    return AppConfig {
        ui: PathBuf::from(matches.value_of("ui").expect("Missing UI parameter")),
        pages: PathBuf::from(matches.value_of("pages").expect("Missing pages parameter")),
        data: PathBuf::from(matches.value_of("data").expect("Missing DB parameter")),
        port: matches.value_of("port").unwrap_or("8080").to_string(),
        slow: matches.is_present("slow"),
    };
}

async fn index() -> HttpResponse {
    HttpResponse::Found()
        .header(http::header::LOCATION, "/app/main")
        .finish()
}

async fn app_page(state: web::Data<AppState>) -> WebResult<fs::NamedFile> {
    Ok(fs::NamedFile::open(state.config.pages.join("index.html"))?)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let config = get_config();

    std::fs::create_dir_all(config.data.join("res").join("images"))?;

    let db_path = config.data.join("lenote.db");
    info!("Opening DB connection to {}", db_path.display());
    let mut connection = rusqlite::Connection::open(&db_path).unwrap();
    database::init(&mut connection).unwrap();
    let db = Arc::new(Mutex::new(connection));

    let addr = format!("127.0.0.1:{}", config.port);
    info!("Listening on {}", addr);
    HttpServer::new(move || {
        let app_state = AppState {
            config: config.clone(),
            db: db.clone(),
        };

        let slow = config.slow;
        App::new()
            .app_data(web::PayloadConfig::default().limit(1024 * 1024 * 500))
            .app_data(web::JsonConfig::default().limit(1024 * 1024 * 500))
            .wrap_fn(move |req, srv| {
                if slow {
                    std::thread::sleep(Duration::from_millis(100));
                }
                srv.call(req)
            })
            .data(app_state)
            .service(fs::Files::new("/ui", &config.ui))
            .service(fs::Files::new("/static", &config.pages.join("static")))
            .service(fs::Files::new("/res", &config.data.join("res")))
            .route("/api/notes{_:/?}", web::post().to(note_api::http_save_note))
            .route("/api/notes{_:/?}", web::get().to(note_api::http_get_notes))
            .route("/api/tags{_:/?}", web::get().to(tag_api::http_get_tags))
            .route(
                "/api/tags/{tag}{_:/?}",
                web::get().to(tag_api::http_get_tag_map),
            )
            .route(
                "/api/tags/{tag}{_:/?}",
                web::post().to(tag_api::http_save_tag_map),
            )
            .route("/", web::get().to(index))
            .route("/app{_:/?}", web::get().to(index))
            .route("/app/{app:[a-zA-z0-9_\\-/]+}", web::get().to(app_page))
    })
    .bind(&addr)?
    .run()
    .await?;

    Ok(())
}
