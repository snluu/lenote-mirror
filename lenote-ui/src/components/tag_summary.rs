use crate::comm::{TagEvent, TagEventBus};
use lenote_common::models::*;
use yew::agent::{Bridge, Bridged};
use yew::format::{Json, Nothing};
use yew::services::fetch::{FetchService, FetchTask};
use yew::services::fetch::{Request as FetchRequest, Response as FetchResponse};
use yew::services::ConsoleService;
use yew::{html, Component, ComponentLink, Html, ShouldRender};

struct State {
    tags: Vec<Tag>,
    error: Option<String>,
}

#[derive(Debug)]
pub enum Msg {
    TagsLoaded(Vec<Tag>),
    NewTagEvent(TagEvent),
    Error(String),
}

pub struct TagSummary {
    state: State,
    link: ComponentLink<Self>,
    console: ConsoleService,
    fetch: FetchService,
    fetch_task: Option<anyhow::Result<FetchTask>>,
    _tag_event_producer: Box<dyn Bridge<TagEventBus>>,
}

impl Component for TagSummary {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let event_bus_cb = link.callback(|e| Msg::NewTagEvent(e));
        Self {
            state: State {
                tags: vec![],
                error: None,
            },
            link,
            console: ConsoleService::new(),
            fetch: FetchService::new(),
            fetch_task: None,
            _tag_event_producer: TagEventBus::bridge(event_bus_cb),
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.fetch_tags();
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::TagsLoaded(tags) => {
                self.state.tags = tags;
                true
            }
            Msg::NewTagEvent(e) => self.handle_tag_event(e),
            Msg::Error(e) => {
                self.console.error(&e);
                self.state.error = Some(e);
                true
            }
        }
    }

    fn view(&self) -> Html {
        if self.state.tags.is_empty() {
            return html! {
                <div>{ "No Tag" }</div>
            };
        }

        html! {
            <>
                <div class="tag-category">{ "Active Tags" }</div>
                {
                    for self.state.tags.iter().map(|tag| {
                        let actives = tag.maps.iter().filter(|m| m.status == TagMapStatus::Active).count();
                        if actives == 0 {
                            html!{}
                        } else {
                            let tag_text = format!("{} ({})", tag.tag, actives);
                            let url = format!("/app/tag/{}", tag.tag.get(1..).unwrap());
                            html! {
                                <div class="tag"><a href={url}>{ tag_text }</a></div>
                            }
                        }
                    })
                }
                <div class="tag-category">{ "All Tags" }</div>
                {
                    for self.state.tags.iter().map(|tag| {
                        let tag_text = format!("{} ({})", tag.tag, tag.maps.len());
                        let url = format!("/app/tag/{}", tag.tag.get(1..).unwrap());
                        html! {
                            <div class="tag"><a href={url}>{ tag_text }</a></div>
                        }
                    })
                }
            </>
        }
    }
}

impl TagSummary {
    fn fetch_tags(&mut self) {
        self.console.log("Fetching tags");
        let callback = self.link.callback(
            move |response: FetchResponse<Json<Result<Vec<Tag>, anyhow::Error>>>| {
                let (meta, Json(tags)) = response.into_parts();

                if meta.status.is_success() {
                    match tags {
                        Ok(tags) => Msg::TagsLoaded(tags),
                        Err(e) => Msg::Error(e.to_string()),
                    }
                } else {
                    Msg::Error(format!("META: {:?}, {:?}", meta, tags))
                }
            },
        );
        let request = FetchRequest::get("/api/tags").body(Nothing).unwrap();

        self.fetch_task = Some(self.fetch.fetch(request, callback));
    }

    fn handle_tag_event(&mut self, e: TagEvent) -> ShouldRender {
        match e {
            TagEvent::TagsChanged => {
                self.fetch_tags();
                false
            }
        }
    }
}
