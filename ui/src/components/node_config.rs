// FIXME: merge node_state and node_config?
use crate::components::node_state::NodeState;
pub use config::prelude::*;
pub use dioxus::prelude::*;

#[derive(Debug, Default, Config)]
#[section(input=dataframe, output=dataframe)]
pub struct ConfigExample {
    daemon: Vec<String>,
    truncate: bool,
    host: String,
    port: u16,
    user: String,
    #[field_type(password)]
    password: String,
    database: String,
    #[field_type(text_area)]
    query: String,
}
#[component]
pub fn NodeConfig(selected_node: Signal<Option<Signal<NodeState>>>) -> Element {
    if let Some(inner_signal) = *selected_node.read() {
        // FIXME: config needs to be loaded from backend
        // How to deal with configuration drift?
        // Technically we can carefully update config sections and add new fields with default values,
        // but if rename happens for some unavoidable reason - how to notify that to user?
        // State load / intermediate save to indexed db
        let NodeState {
            ref config,
            id,
            node_type,
            ..
        } = *inner_signal.read();
        let config_fields = config.fields();
        return rsx! {
            div {
                class: "border border-solid rounded-md drop-shadow px-5 py-4 mt-4 mx-4",
                div {
                    form {
                        onsubmit: move |event| {
                            tracing::info!("event: {:?}", event);
                            selected_node.set(None);
                        },
                        class: "grid grid-flow-rows gap-2",
                        div {
                            h2 {
                                class: "text-lg",
                                "Editing Node {id}"
                            }
                            h3 {
                                "Section Type: {node_type}"
                            }
                        }
                        for field in config_fields {
                            div {
                                if field.ty.is_vec() {
                                    div {
                                        class: "flex items-center justify-start",
                                        label {
                                            r#for: "{field.name}",
                                            class: "text-sm font-medium leading-6 text-night-1 uppercase min-w-24",
                                            "{field.name}"
                                        }
                                        select {
                                            id: "{field.name}",
                                            name: "{field.name}",
                                            class: "p-2 ml-3 min-w-32 r unded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2 focus:outline-none",
                                            //multiple: "true",
                                            option {
                                                value: "*",
                                                "*",
                                            }
                                        }
                                    }
                                } else if field.ty.is_bool() {
                                    div { class: "flex items-center justify-start",
                                        label {
                                            r#for: "{field.name}",
                                            class: "min-w-24 text-sm font-medium leading-6 text-night-1 uppercase",
                                            "{field.name}"
                                        }
                                        input {
                                            id: "{field.name}",
                                            name: "{field.name}",
                                            r#type: "checkbox",
                                            class: "ml-3 rounded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2 focus:outline-none",
                                        }
                                    }
                                } else if field.metadata.is_text_area {
                                    label {
                                        r#for: "{field.name}",
                                        class: "text-sm font-medium leading-6 text-night-1 uppercase",
                                        "{field.name}"
                                    }
                                    // div included here so that textarea below appears on new grid row (ie, below the label)
                                    div {
                                        textarea {
                                            id: "{field.name}",
                                            name: "{field.name}",
                                            class: "w-full rounded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2 focus:outline-none",
                                        }
                                    }
                                } else {
                                    // returns basic text input
                                    div {
                                        label {
                                            r#for: "{field.name}",
                                            class: "text-sm font-medium leading-6 text-night-1 uppercase",
                                            "{field.name}"
                                        }
                                        input {
                                            id: "{field.name}",
                                            name: "{field.name}",
                                            r#type: if field.metadata.is_password { "password" } else { "text" },
                                            autocomplete: "off",
                                            class: "w-full rounded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2 focus:outline-none",
                                        }
                                    }
                                }
                            }
                        }
                        div {
                            class: "justify-self-center",
                            button {
                                r#type:"submit",
                                class:"text-stem-1 px-4 py-2 rounded bg-forest-1 border border-forest-2 hover:bg-forest-2 hover:text-white uppercase font-semibold  drop-shadow-sm",
                                "Save"
                            }
                            button {
                                prevent_default: "onclick",
                                onclick: move |_| {
                                    selected_node.set(None);
                                },
                                r#type:"submit",
                                class: "uppercase text-toadstool-1 px-4 py-2 ml-2 rounded border border-toadstool-1 hover:text-white hover:bg-toadstool-2",
                                "Cancel"
                            }
                        }
                    }
                }
            }
        };
    }
    None
}
