use crate::components::graph::NodeState;
pub use dioxus::prelude::*;

#[component]
pub fn NodeConfig(selected_node: Signal<Option<Signal<NodeState>>>) -> Element {
    if let Some(inner_signal) = *selected_node.read() {
        let NodeState { id, .. } = *inner_signal.read();
        return rsx! {
            div {
                prevent_default: "onclick",
                    onclick: move |_| {
                        selected_node.set(None);
                    },
                class: "grid grid-flow-rows gap-2",
                "{id}"
        }
        };
    }
    None
}
