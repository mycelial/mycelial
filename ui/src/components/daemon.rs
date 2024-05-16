use dioxus::prelude::*;

pub fn Daemon(daemon: String) -> Element {
    rsx! {
     div {
            h1 {
                "Daemon: {daemon}"
            }
    }
    }
}
