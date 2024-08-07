use dioxus::prelude::*;

use crate::components::{
    daemon::Daemon, daemon_tokens::DaemonTokens, daemons::Daemons, index::Index, login::Login,
    navbar::NavBar, workspace::Workspace, workspaces::Workspaces,
};

#[rustfmt::skip]
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
        #[route("/tokens")]
        DaemonTokens {},
    #[end_layout]
    #[route("/login")]
    Login {},
}
