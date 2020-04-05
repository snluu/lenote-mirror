use crate::comm::{NoteEvent, NoteEventBus, TagEvent, TagEventBus};
use crate::js_util;
use js_sys::Math::random;
use lenote_common::models::*;
use std::collections::{HashMap, HashSet};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use yew::agent::{Dispatched, Dispatcher};
use yew::events::{InputData, KeyboardEvent};
use yew::format::Json;
use yew::html::NodeRef;
use yew::services::fetch::{FetchService, FetchTask};
use yew::services::fetch::{Request as FetchRequest, Response as FetchResponse};
use yew::services::ConsoleService;
use yew::{html, Component, ComponentLink, Html, ShouldRender};

const CHARS: &'static [char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9',
];

struct State {
    note: String,
    last_client_note_id: i64,
    error: Option<String>,
}

#[derive(Debug)]
pub enum Msg {
    None,
    NoteChanged(String),
    Submit,
    NoteSaved(Note),
    NoteSaveFailed(String),
    ImagePasted(String),
}

pub struct NoteInput {
    id: String,
    state: State,
    link: ComponentLink<Self>,
    note_events: Dispatcher<NoteEventBus>,
    tag_events: Dispatcher<TagEventBus>,
    console: ConsoleService,
    fetch: FetchService,
    fetch_tasks: HashMap<String, anyhow::Result<FetchTask>>,
    input_node: NodeRef,
    paste_callback_interop: Closure<dyn FnMut(String)>,
}

impl Component for NoteInput {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let paste_callback = link.callback(|s: String| Msg::ImagePasted(s));
        let mut id = String::with_capacity(16);
        for _ in 0..id.capacity() {
            id.push(CHARS[(random() * CHARS.len() as f64) as usize]);
        }

        Self {
            id,
            state: State {
                note: String::from(""),
                last_client_note_id: 0,
                error: None,
            },
            link,
            note_events: NoteEventBus::dispatcher(),
            tag_events: TagEventBus::dispatcher(),
            console: ConsoleService::new(),
            fetch: FetchService::new(),
            fetch_tasks: HashMap::new(),
            input_node: NodeRef::default(),
            paste_callback_interop: Closure::wrap(Box::new(move |s: String| {
                paste_callback.emit(s);
            }) as Box<dyn FnMut(String)>),
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        if let Some(input) = self.input_node.cast::<web_sys::HtmlTextAreaElement>() {
            let _ = input.focus().unwrap_or_default();
        }

        set_img_paste_callback(&self.id, &self.paste_callback_interop);

        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::NoteChanged(note) => {
                self.state.note = note.into();
                true
            }
            Msg::Submit => {
                self.submit_note();
                true
            }
            Msg::NoteSaved(note) => {
                self.console.log(&format!("Note saved: {:?}", note));
                self.fetch_tasks.remove(&note.client_id);

                if !note.tags.is_empty() {
                    self.tag_events.send(TagEvent::TagsChanged);
                }
                self.note_events.send(NoteEvent::NoteSaved(note));
                true
            }
            Msg::NoteSaveFailed(e) => {
                self.console.error(&e);
                self.state.error = Some(e);
                true
            }
            Msg::ImagePasted(img_data) => {
                self.submit_image(img_data);
                true
            }
            Msg::None => false,
        }
    }

    fn view(&self) -> Html {
        html! {
            <div class="note-input">
                { self.view_error() }
                <textarea
                    ref=self.input_node.clone()
                    class="note-input"
                    rows="5"
                    placeholder="Enter note here"
                    value=&self.state.note
                    oninput=self.link.callback(|e: InputData| Msg::NoteChanged(e.value))
                    onkeypress=self.link.callback(|e: KeyboardEvent| {
                        if !(e.meta_key() || e.alt_key() || e.shift_key() || e.ctrl_key()) && e.key() == "Enter" {
                            e.prevent_default();
                            Msg::Submit
                        } else { Msg::None }
                    })
                    onpaste="handleInputPaste('main-input', event);"
                ></textarea>
            </div>
        }
    }
}

impl NoteInput {
    fn submit_note(&mut self) {
        if !self.state.note.trim().is_empty() {
            self.console
                .log(&format!("Submitting note {}", self.state.note));

            self.state.last_client_note_id += 1;
            let note = Note {
                id: 0,
                client_id: format!("nn-{}", self.state.last_client_note_id),
                text: self.state.note.clone(),
                timestamp: js_util::now(),
                note_type: NoteType::Text,
                tags: HashSet::new(),
            };

            let fetch_task = self.fetch_submit_note(&note);
            // Need to keep this task alive, otherwise it will go out of scope
            self.fetch_tasks.insert(note.client_id.clone(), fetch_task);
            self.note_events.send(NoteEvent::NoteSubmitted(note));

            self.state.note.clear();
        }
    }

    fn submit_image(&mut self, img_data: String) {
        self.console
            .log(&format!("Submitting image, len: {}", img_data.len()));
        self.state.last_client_note_id += 1;
        let note = Note {
            id: 0,
            client_id: format!("nn-{}", self.state.last_client_note_id),
            text: img_data,
            timestamp: js_util::now(),
            note_type: NoteType::Image,
            tags: HashSet::new(),
        };

        let fetch_task = self.fetch_submit_note(&note);
        // Need to keep this task alive, otherwise it will go out of scope
        self.fetch_tasks.insert(note.client_id.clone(), fetch_task);
        self.note_events.send(NoteEvent::NoteSubmitted(note));
    }

    fn fetch_submit_note(&mut self, note: &Note) -> anyhow::Result<FetchTask> {
        let callback = self.link.callback(
            move |response: FetchResponse<Json<Result<Note, anyhow::Error>>>| {
                let (meta, Json(n)) = response.into_parts();

                if meta.status.is_success() {
                    match n {
                        Ok(note) => Msg::NoteSaved(note),
                        Err(e) => Msg::NoteSaveFailed(e.to_string()),
                    }
                } else {
                    Msg::NoteSaveFailed(format!("META: {:?}, {:?}", meta, n))
                }
            },
        );
        let request = FetchRequest::post("/api/notes")
            .header("Content-Type", "application/json")
            .body(Json(note))
            .unwrap();

        self.fetch.fetch(request, callback)
    }

    fn view_error(&self) -> Html {
        if let Some(error) = &self.state.error {
            html! {
                <div class="error">{ format!("Error: {}", error) }</div>
            }
        } else {
            html! {
                <div class="error"></div>
            }
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "setImagePasteCallback")]
    fn set_img_paste_callback(id: &str, cb: &Closure<dyn FnMut(String)>);
}
