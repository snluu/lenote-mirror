use crate::js_util::get_js_date_string;
use lenote_common::models::*;
use std::rc::Rc;
use yew::virtual_dom::VNode;
use yew::{html, Component, ComponentLink, Html, Properties};

#[derive(Properties, Clone)]
pub struct Props {
    pub note: Rc<Note>,
    #[prop_or_default]
    pub show_time: bool,
    #[prop_or_default]
    pub note_index: usize,
    #[prop_or_default]
    pub highlight: bool,
}

pub struct NoteViewer {
    props: Props,
}

impl Component for NoteViewer {
    type Message = ();
    type Properties = Props;
    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self { props }
    }

    fn update(&mut self, _: Self::Message) -> bool {
        unimplemented!()
    }

    fn change(&mut self, props: Props) -> bool {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        let note = &self.props.note;
        let time_str = get_js_date_string(note.timestamp);
        let title = format!("ID: {}. {}. Tags: {:?}", note.id, time_str, note.tags);
        let note_class = if self.props.highlight {
            "note note-highlight"
        } else {
            "note"
        };

        html! {
            <div class={ note_class } title={ title }>
                {
                    if self.props.show_time {
                        let time_class = if self.props.note_index == 0 {
                            "unselectable note-time-first"
                        } else {
                            "unselectable note-time"
                        };
                        html! {
                            <div class={ time_class }>{ time_str }</div>
                        }
                    } else {
                        html! {}
                    }
                }
                { match note.note_type {
                    NoteType::Text => self.view_text(),
                    NoteType::Image => self.view_image(),
                    _ => html! {}
                }}
            </div>
        }
    }
}

fn raw_node(element: &str, inner_html: &str) -> Html {
    let elem = yew::utils::document().create_element(element).unwrap();
    elem.set_inner_html(inner_html);

    let node: web_sys::Node = elem.into();
    VNode::VRef(node)
}

impl NoteViewer {
    fn view_text(&self) -> Html {
        let note = &self.props.note;
        html! {
            {
                for note.text.lines().map(|l| {
                    if l.trim().is_empty() {
                        raw_node("div", "&nbsp;")
                    } else {
                        html! {
                            <div
                                class={ if note.id > 0 { "note-line" } else { "note-line-pending" }}
                            >
                                { l }
                            </div>
                        }
                    }
                })
            }
        }
    }

    fn view_image(&self) -> Html {
        let note = &self.props.note;
        html! {
            <img class="image-note" src={&note.text} />
        }
    }
}
