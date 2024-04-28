use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::components::{
    login::Login,
    workspaces::Workspaces,
    daemons::Daemons,
    index::Index,
};

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/")]
    Index{},
    #[route("/workspaces")]
    Workspaces{},
    #[route("/login")]
    Login{},
    #[route("/daemons")]
    Daemons{},
}

pub fn App() -> Element {
    rsx! {
        Router::<Route> { }
    }
}