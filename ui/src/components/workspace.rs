use dioxus::prelude::*;

#[component]
pub fn Workspace(workspace: String) -> Element {
    rsx! {
        "workspace: {workspace}"
    }
}