use dioxus::prelude::*;

#[component]
pub fn Daemon(daemon: String) -> Element {
    rsx! {
     div {
            h1 {
                "Daemon: {daemon}"
            }
    }
    }
}
