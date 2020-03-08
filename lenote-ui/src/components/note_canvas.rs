use super::NoteViewer;
use crate::comm::{NoteEvent, NoteEventBus};
use lenote_common::models::Note;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;
use web_sys::Element;
use yew::agent::{Bridge, Bridged};
use yew::format::{Json, Nothing};
use yew::html::NodeRef;
use yew::services::fetch::{FetchService, FetchTask};
use yew::services::fetch::{Request as FetchRequest, Response as FetchResponse};
use yew::services::{dialog::DialogService, ConsoleService};
use yew::services::{timeout::TimeoutTask, TimeoutService};
use yew::{html, Component, ComponentLink, Html, ShouldRender};

const SHOW_NOTE_TIME_MESSAGE_GAP: i64 = 1800;

struct State {
    notes: Vec<Rc<Note>>,
    pending_notes: HashMap<String, usize>,
}

pub enum Msg {
    NewNoteEvent(NoteEvent),
    NotesLoaded(Vec<Rc<Note>>),
    ScrollBottom,
    Error(String),
}

pub struct NoteCanvas {
    state: State,
    link: ComponentLink<Self>,
    console: ConsoleService,
    dialog: DialogService,
    fetch: FetchService,
    timeout: TimeoutService,
    timeout_task: Option<TimeoutTask>,
    fetch_task: Option<anyhow::Result<FetchTask>>,
    canvas_div_ref: NodeRef,
    _note_event_producer: Box<dyn Bridge<NoteEventBus>>,
}

impl Component for NoteCanvas {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let event_bus_cb = link.callback(|e| Msg::NewNoteEvent(e));
        Self {
            state: State {
                notes: vec![],
                pending_notes: HashMap::new(),
            },
            link,
            console: ConsoleService::new(),
            dialog: DialogService::new(),
            fetch: FetchService::new(),
            timeout: TimeoutService::new(),
            timeout_task: None,
            fetch_task: None,
            canvas_div_ref: NodeRef::default(),
            // Need to keep a reference of this so that it won't
            // disconnect from the event bus when going out of scope
            _note_event_producer: NoteEventBus::bridge(event_bus_cb),
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.fetch_notes();
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::NewNoteEvent(e) => self.handle_note_event(e),
            Msg::ScrollBottom => self.scroll_to_bottom(),
            Msg::NotesLoaded(notes) => {
                self.console.log(&format!("Found {} notes", notes.len()));
                self.state.notes = notes;

                // We will send a delayed message
                // for all the messages to finish rendering
                self.timeout_task = Some(self.timeout.spawn(
                    Duration::from_millis(150),
                    self.link.callback(|_| Msg::ScrollBottom),
                ));

                true
            }
            Msg::Error(e) => {
                self.dialog.alert(&e);
                false
            }
        }
    }

    fn view(&self) -> Html {
        html! {
            <div
                ref=self.canvas_div_ref.clone()
                class="note-canvas"
            >
                {
                    for self.state.notes.iter().enumerate()
                        .map(|(i, note)| self.view_note(note.clone(), i))
                }
            </div>
        }
    }
}

impl NoteCanvas {
    fn view_note(&self, note: Rc<Note>, index: usize) -> Html {
        let show_time = index == 0
            || (note.timestamp > 0
                && note.timestamp - self.state.notes[index - 1].timestamp
                    > SHOW_NOTE_TIME_MESSAGE_GAP);
        html! {
            <NoteViewer note={ note } note_index={ index } show_time={ show_time } />
        }
    }

    fn scroll_to_bottom(&self) -> ShouldRender {
        self.canvas_div_ref
            .cast::<Element>()
            .unwrap()
            .set_scroll_top(std::i32::MAX - 1);
        false
    }

    fn handle_note_event(&mut self, e: NoteEvent) -> ShouldRender {
        match e {
            NoteEvent::NoteSubmitted(note) => {
                self.console
                    .log(&format!("Received new message {}", note.client_id,));
                self.state.notes.push(Rc::new(note));
                self.state.pending_notes.insert(
                    self.state.notes.last().unwrap().client_id.clone(),
                    self.state.notes.len() - 1,
                );

                // This message will kick in on the next iteration of
                // the event loop, which will scroll to the bottom of the div
                self.link.send_message(Msg::ScrollBottom);
                true
            }
            NoteEvent::NoteSaved(note) => {
                self.console
                    .log(&format!("Marking note {} as saved", note.client_id));
                if let Some(index) = self.state.pending_notes.remove(&note.client_id) {
                    self.state.notes[index] = Rc::new(note);
                }
                true
            }
        }
    }

    fn fetch_notes(&mut self) {
        self.console.log("Fetching notes");
        let callback = self.link.callback(
            move |response: FetchResponse<Json<Result<Vec<Note>, anyhow::Error>>>| {
                let (meta, Json(n)) = response.into_parts();

                if meta.status.is_success() {
                    match n {
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
                    Msg::Error(format!("META: {:?}, {:?}", meta, n))
                }
            },
        );
        let request = FetchRequest::get("/api/notes").body(Nothing).unwrap();

        self.fetch_task = Some(self.fetch.fetch(request, callback));
    }
}
