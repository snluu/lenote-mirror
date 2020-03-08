use super::TagMapViewer;
use lenote_common::models::*;
use yew::format::{Json, Nothing};
use yew::services::fetch::{FetchService, FetchTask};
use yew::services::fetch::{Request as FetchRequest, Response as FetchResponse};
use yew::services::ConsoleService;
use yew::{html, Component, ComponentLink, Html, Properties};

#[derive(Properties, Clone)]
pub struct Props {
    pub naked_tag: String,
}

pub enum Msg {
    TagMapLoaded(Vec<TagMap>),
    ToggleShowActives,
    ToggleShowArchived,
    TagMapUpdated((usize, TagMap)),
    Error(String),
}

struct State {
    tag_map: Vec<TagMap>,
    error: Option<String>,
    show_actives: bool,
    show_archived: bool,
}

pub struct TagViewer {
    state: State,
    props: Props,
    link: ComponentLink<Self>,
    console: ConsoleService,
    fetch: FetchService,
    fetch_task: Option<anyhow::Result<FetchTask>>,
}

impl Component for TagViewer {
    type Message = Msg;
    type Properties = Props;
    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            state: State {
                tag_map: vec![],
                error: None,
                show_actives: true,
                show_archived: false,
            },
            props,
            link,
            console: ConsoleService::new(),
            fetch: FetchService::new(),
            fetch_task: None,
        }
    }

    fn mounted(&mut self) -> bool {
        self.fetch_tag_map();
        false
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        match msg {
            Msg::TagMapLoaded(tag_map) => {
                self.console.log("Tag map loaded");
                self.state.tag_map = tag_map;
                true
            }
            Msg::ToggleShowActives => {
                self.state.show_actives = !self.state.show_actives;
                true
            }
            Msg::ToggleShowArchived => {
                self.state.show_archived = !self.state.show_archived;
                true
            }
            Msg::TagMapUpdated((i, tag_map)) => {
                self.state.tag_map[i] = tag_map;
                true
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

        html! {
            <>
                <div style="margin-bottom: 30px;">
                    <div class="tag-headline">{ format!("#{}", self.props.naked_tag) }</div>
                    <div>
                        <input
                            type="checkbox"
                            name="show_actives"
                            id="show_actives"
                            checked=self.state.show_actives
                            onclick=self.link.callback(|_| Msg::ToggleShowActives)
                        />
                        <label for="show_actives">{ "Show Active Items" }</label>

                        <input
                            type="checkbox"
                            name="show_archive"
                            id="show_archive"
                            style="margin-left: 30px"
                            checked=self.state.show_archived
                            onclick=self.link.callback(|_| Msg::ToggleShowArchived)
                        />
                        <label for="show_archive">{ "Show Archived Items" }</label>
                    </div>
                </div>
                { for self.state.tag_map.iter().enumerate().map(|(i, t)|
                    if self.should_show(t) {
                        html! {
                            <TagMapViewer
                                naked_tag={ self.props.naked_tag.clone() }
                                tag_map={ t.clone() }
                                index=i
                                onupdate=self.link.callback(|e| Msg::TagMapUpdated(e))
                            />
                        }
                    } else {
                        html! {}
                    }
                ) }
            </>
        }
    }
}

impl TagViewer {
    fn fetch_tag_map(&mut self) {
        self.console.log("Fetching tags");
        let callback = self.link.callback(
            move |response: FetchResponse<Json<Result<Vec<TagMap>, anyhow::Error>>>| {
                let (meta, Json(tags)) = response.into_parts();

                if meta.status.is_success() {
                    match tags {
                        Ok(tags) => Msg::TagMapLoaded(tags),
                        Err(e) => Msg::Error(e.to_string()),
                    }
                } else {
                    Msg::Error(format!("META: {:?}, {:?}", meta, tags))
                }
            },
        );
        let request = FetchRequest::get(format!("/api/tags/{}", self.props.naked_tag))
            .body(Nothing)
            .unwrap();

        self.fetch_task = Some(self.fetch.fetch(request, callback));
    }

    fn should_show(&self, tag_map: &TagMap) -> bool {
        match tag_map.status {
            TagMapStatus::Active => self.state.show_actives,
            TagMapStatus::Archived => self.state.show_archived,
        }
    }
}
