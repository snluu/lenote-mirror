use super::note_canvas::NoteCanvas;
use super::note_input::NoteInput;
use super::tag_summary::TagSummary;
use super::tag_viewer::TagViewer;
use super::AppRoute;

use yew::{html, Component, ComponentLink, Html, ShouldRender};
use yew_router::{service::RouteService, Switch};

pub struct Composer {
    route_service: RouteService<()>,
}

impl Component for Composer {
    type Message = ();
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self {
            route_service: RouteService::new(),
        }
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div class="full-height">
                <div class="page-title"><a href="/">{ "Lenote" }</a></div>
                <div class="composer full-height">
                    <div class="side-pane full-height">
                        <TagSummary />
                    </div>
                    <div class="main-pane full-height">
                        { self.view_main_pane() }
                    </div>
                </div>
            </div>
        }
    }
}

impl Composer {
    fn view_main_pane(&self) -> Html {
        let route = self.route_service.get_route();
        match AppRoute::switch(route) {
            Some(AppRoute::Main) => html! {
                <>
                    <NoteCanvas />
                    <br/>
                    <br/>
                    <NoteInput />
                </>
            },
            Some(AppRoute::Tag(tag)) => html! {
                <>
                    <TagViewer naked_tag={ tag } />
                </>
            },
            _ => html! {
                <div>{ "Route not found" }</div>
            },
        }
    }
}
