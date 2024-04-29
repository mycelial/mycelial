use dioxus::prelude::*;

#[component]
pub fn Workspaces() -> Element {
    rsx! {
        div {
            class: "flex pt-4",
            div {
                class: "w-1/12",
            }
            div {
                class: "flex w-10/12 justify-between",
                h2 {
                    class: "flex-none content-center",
                    font_size: "1.5em",
                    font_weight: "bold",
                    "Workspaces"
                }
                button {
                    class: "flex-initial text-white px-4 py-2 rounded",
                    style: "background-color: #1a237e",
                    "ADD NEW WORKSPACE"
                }
            }
        }
    }
}
