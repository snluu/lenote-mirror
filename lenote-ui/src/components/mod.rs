mod composer;
mod note_canvas;
mod note_input;
mod note_viewer;
mod tag_map_viewer;
mod tag_summary;
mod tag_viewer;

use yew_router::Switch;

#[derive(Debug, Switch, Clone)]
pub enum AppRoute {
    #[to = "/app/main"]
    Main,
    #[to = "/app/tag/{anything}"]
    Tag(String),
}

pub use composer::Composer;
pub use note_viewer::NoteViewer;
pub use tag_map_viewer::TagMapViewer;
pub use tag_summary::TagSummary;
pub use tag_viewer::TagViewer;
