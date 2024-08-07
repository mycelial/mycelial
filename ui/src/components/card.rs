pub use dioxus::prelude::*;

#[component]
pub fn Card(title: String, subtitle: String, content: String) -> Element {
    rsx! {
        div {
            class: "min-h-24 md:w-96 rounded border border-night-2 border-2 mt-4 grid grid-cols-1 gap gap-2 divide-y drop-shadow-none divide-night-2/85 bg-night-2/25",
            h1 {
                class: "text-lg pt-3 pb-1 px-3",
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
