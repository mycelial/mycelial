use crate::components::navbar::NavBar;
pub use dioxus::prelude::*;

pub fn Index() -> Element {
    rsx! {
        NavBar{},
        "index"
    }
}
