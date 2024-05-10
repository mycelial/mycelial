pub use dioxus::prelude::*;
use crate::components::workspace::NodeState;


#[component]
pub fn NodeConfig(selected_node: Signal<Option<Signal<NodeState>>>,) -> Element {

if let Some(inner_signal) = *selected_node.read() {
    let NodeState { id, node_type, ..} = *inner_signal.read();
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