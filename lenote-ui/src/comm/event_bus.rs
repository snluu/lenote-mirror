use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use yew::agent::{Agent, AgentLink, Context, HandlerId};

pub struct EventBus<EventType: Clone + Serialize + for<'de> Deserialize<'de> + 'static> {
    link: AgentLink<Self>,
    subscribers: HashSet<HandlerId>,
}

impl<EventType: Clone + Serialize + for<'de> Deserialize<'de> + 'static> Agent
    for EventBus<EventType>
{
    type Reach = Context;
    type Message = ();
    type Input = EventType;
    type Output = EventType;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, _: <Self as yew::agent::Agent>::Message) {
        unimplemented!()
    }

    fn handle_input(&mut self, msg: Self::Input, _: HandlerId) {
        for sub in self.subscribers.iter() {
            self.link.respond(*sub, msg.clone());
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.subscribers.insert(id);
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }
}
