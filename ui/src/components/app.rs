use crate::components::routing::Route;
use dioxus::prelude::*;

pub fn App() -> Element {
    rsx! {
        Router::<Route> { }
    }
}
