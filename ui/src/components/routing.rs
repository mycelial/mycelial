use dioxus::prelude::*;

use crate::components::{
    daemon::Daemon, daemons::Daemons, index::Index, login::Login, navbar::NavBar,
    workspace::Workspace, workspaces::Workspaces,
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
    #[nest("/daemons")]
    #[route("/")]
    Daemons {},
    #[route("/:daemon")]
    Daemon { daemon: String },
    #[end_nest]
    #[route("/login")]
    Login {},
}
