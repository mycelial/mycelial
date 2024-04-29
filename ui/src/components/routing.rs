use dioxus::prelude::*;

use crate::components::{
    daemons::Daemons, index::Index, login::Login, workspace::Workspace, workspaces::Workspaces,
};

#[derive(Clone, Debug, PartialEq, Routable)]
pub enum Route {
    #[route("/")]
    Index {},
    #[route("/workspaces")]
    Workspaces {},
    #[route("/workspace/:workspace")]
    Workspace { workspace: String },
    #[route("/login")]
    Login {},
    #[route("/daemons")]
    Daemons {},
}