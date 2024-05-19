pub use dioxus::prelude::*;

#[component]
pub fn Card(title: String, subtitle: String, content: String) -> Element {
    rsx! {
        div {
            class: "min-h-24 rounded border border-night-2 border-2 mt-4 grid grid-cols-1 gap gap-2 divide-y drop-shadow-none divide-moss-1",
            h1 {
                class: "text-lg pt-3 pb-1 pl-3",
                "{title}"
            },
            h2 {
                class: "p-3",
                "{subtitle}"
            },
            h2 {
                class: "p-3",
                "{content}"
                }
        }
    }
}
