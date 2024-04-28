pub use dioxus::prelude::*;

pub fn Index() -> Element {
    tracing::info!("hello from index");
    rsx! {
        "index"
    }
}