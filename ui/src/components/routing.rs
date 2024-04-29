use dioxus::prelude::*;

use crate::components::{
    daemons::Daemons, index::Index, login::Login, workspace::Workspace, workspaces::Workspaces,
    navbar::NavBar,
};

#[derive(Routable, Clone, Debug)]
pub enum Route {
    #[layout(NavBar)]
        #[route("/")]
        Index {},
        #[nest("/workspaces")]
            #[route("/")]
            Workspaces {},
            #[route("/:workspace")]
            Workspace { workspace: String },
        #[end_nest]
        #[route("/login")]
        Login {},
        #[route("/daemons")]
        Daemons {}
}