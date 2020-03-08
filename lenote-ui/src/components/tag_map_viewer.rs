use super::NoteViewer;
use crate::comm::{TagEvent, TagEventBus};
use crate::js_util::get_js_date_string;
use lenote_common::models::*;
use std::rc::Rc;
use yew::agent::{Dispatched, Dispatcher};
use yew::format::{Json, Nothing};
use yew::services::fetch::{FetchService, FetchTask};
use yew::services::fetch::{Request as FetchRequest, Response as FetchResponse};
use yew::services::ConsoleService;
use yew::services::DialogService;
use yew::Callback;
use yew::ShouldRender;
use yew::{html, Component, ComponentLink, Html, Properties};

#[derive(Properties, Clone)]
pub struct Props {
    pub tag_map: TagMap,
    pub naked_tag: String,
    pub index: usize,
    #[prop_or_default]
    pub onupdate: Option<Callback<(usize, TagMap)>>,
}

pub enum Msg {
    NotesLoaded(Vec<Rc<Note>>),
    Error(String),
    UpdateTagStatus(TagMapStatus),
    MorePrev,
    MoreNext,
    Updated(TagMap),
}

struct State {
    min_note_id: i64,
    max_note_id: i64,
    notes: Vec<Rc<Note>>,
    error: Option<String>,
    status_updating_to: Option<TagMapStatus>,
}

pub struct TagMapViewer {
    state: State,
    props: Props,
    link: ComponentLink<Self>,
    tag_events: Dispatcher<TagEventBus>,
    console: ConsoleService,
    dialog: DialogService,
    fetch: FetchService,
    fetch_task: Option<anyhow::Result<FetchTask>>,
}

impl Component for TagMapViewer {
    type Message = Msg;
    type Properties = Props;
    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            state: State {
                min_note_id: props.tag_map.note_id - 5,
                max_note_id: props.tag_map.note_id + 5,
                notes: vec![],
                error: None,
                status_updating_to: None,
            },
            props,
            link,
            tag_events: TagEventBus::dispatcher(),
            console: ConsoleService::new(),
            dialog: DialogService::new(),
            fetch: FetchService::new(),
            fetch_task: None,
        }
    }

    fn mounted(&mut self) -> bool {
        self.fetch_notes();
        false
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        match msg {
            Msg::NotesLoaded(notes) => {
                self.console.log("Notes map loaded");
                self.state.notes = notes;
                true
            }
            Msg::UpdateTagStatus(status) => self.update_status(status),
            Msg::Updated(tag_map) => {
                if let Some(cb) = &self.props.onupdate {
                    cb.emit((self.props.index, tag_map.clone()));
                }

                self.props.tag_map = tag_map;
                self.state.status_updating_to = None;
                self.tag_events.send(TagEvent::TagsChanged);
                true
            }
            Msg::MorePrev => {
                self.state.min_note_id -= 10;
                self.fetch_notes();
                false
            }
            Msg::MoreNext => {
                self.state.max_note_id += 10;
                self.fetch_notes();
                false
            }
            Msg::Error(e) => {
                self.console.error(&e);
                self.state.error = Some(e);
                true
            }
        }
    }

    fn view(&self) -> Html {
        if let Some(e) = &self.state.error {
            return html! {
                <div class="error">{ e }</div>
            };
        }

        let status_str = if let Some(status) = &self.state.status_updating_to {
            format!("{:?} (updating to {:?})", self.props.tag_map.status, status)
        } else {
            format!("{:?}", self.props.tag_map.status)
        };

        let status_class = format!("tag-map-status {}", status_str.to_ascii_lowercase());
        let time_str = get_js_date_string(self.props.tag_map.timestamp);

        html! {
            <>
                <div class={ status_class }>{ status_str }</div>
                <div>
                    <span style="margin-left: 5px;">
                        { self.update_status_button(TagMapStatus::Active, "Activate")}
                    </span>
                    <span style="margin-left: 5px;">
                        { self.update_status_button(TagMapStatus::Archived, "Archive")}
                    </span>
                </div>
                <div class="tag-map-time">{ time_str }</div>
                <div class="tag-notes-more">
                    <a class="link-button" onclick=self.link.callback(|_| Msg::MorePrev)>
                        { "More..." }
                    </a>
                </div>
                <div>
                    { for self.state.notes.iter().enumerate().map(|(idx, note)| html! {
                        <NoteViewer
                            note={ note.clone() }
                            highlight={ note.id == self.props.tag_map.note_id }
                        />
                    }) }
                </div>
                <div class="tag-notes-more">
                    <a class="link-button" onclick=self.link.callback(|_| Msg::MoreNext)>
                        { "More..." }
                    </a>
                </div>
                <div class="spacer-50"></div>
            </>
        }
    }
}

impl TagMapViewer {
    fn update_status_button(&self, target_status: TagMapStatus, caption: &str) -> Html {
        html! {
            <button
                disabled={ self.props.tag_map.status == target_status }
                onclick=self.link.callback(move |_| Msg::UpdateTagStatus(target_status))
            >
                { caption }
            </button>
        }
    }

    fn fetch_notes(&mut self) {
        self.console.log("Fetching notes");
        let callback = self.link.callback(
            move |response: FetchResponse<Json<anyhow::Result<Vec<Note>>>>| {
                let (meta, Json(notes)) = response.into_parts();

                if meta.status.is_success() {
                    match notes {
                        Ok(mut notes) => {
                            let mut note_ptrs = Vec::with_capacity(notes.len());
                            while let Some(note) = notes.pop() {
                                note_ptrs.push(Rc::new(note));
                            }

                            note_ptrs.reverse();
                            Msg::NotesLoaded(note_ptrs)
                        }
                        Err(e) => Msg::Error(e.to_string()),
                    }
                } else {
                    Msg::Error(format!("META: {:?}, {:?}", meta, notes))
                }
            },
        );
        let request = FetchRequest::get(format!(
            "/api/notes?min_id={}&max_id={}",
            self.state.min_note_id, self.state.max_note_id,
        ))
        .body(Nothing)
        .unwrap();

        self.fetch_task = Some(self.fetch.fetch(request, callback));
    }

    fn update_status(&mut self, status: TagMapStatus) -> ShouldRender {
        if self.state.status_updating_to.is_some() {
            self.dialog.alert("Status update pending...");
            return false;
        }

        self.state.status_updating_to = Some(status);

        let callback = self.link.callback(
            move |response: FetchResponse<Json<anyhow::Result<TagMap>>>| {
                let (meta, Json(t)) = response.into_parts();

                if meta.status.is_success() {
                    match t {
                        Ok(t) => Msg::Updated(t),
                        Err(e) => Msg::Error(e.to_string()),
                    }
                } else {
                    Msg::Error(format!("META: {:?}, {:?}", meta, t))
                }
            },
        );

        let payload = TagMap {
            note_id: self.props.tag_map.note_id,
            status: status,
            timestamp: self.props.tag_map.timestamp,
        };

        let request = FetchRequest::post(format!("/api/tags/{}", self.props.naked_tag))
            .header("Content-Type", "application/json")
            .body(Json(&payload))
            .unwrap();

        self.fetch_task = Some(self.fetch.fetch(request, callback));
        true
    }
}
