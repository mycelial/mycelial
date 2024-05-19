// FIXME: merge node_state and node_config?
use crate::components::node_state::NodeState;
pub use dioxus::prelude::*;

// Example of a simple config
// The goal of our configuration system is to be:
// 1. type erased - we don't want to write out and maintain every section configuration more than once
// 2. section should own its configuration
// 3. need registry for node configurations to match node_type to config

// Lets start with some very specific configuration for some db-like source section.
// This is how a specific section will define its specific config.
//
// In order to be able to work in UI with this configuration - we need a way to access
// field/type definitions and some additional metadata, e.g. input can be password or text area, etc.
#[derive(Debug, Default)]
pub struct Config {
    host: String,
    port: u16,
    user: String,
    //#[input(type=password)]
    password: String,
    database: String,
    //#[input(type=text_area)]
    query: String,
    truncate: bool,
    daemon: String,
}

// field describes some specific field in config struct
// FIXME: field value where?
// FIXME: input validators? e.g. if type is integer, but original field is defined as u16, it
// doesn't make sense to allow user to submit request
#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub name: &'static str,
    pub ty: FieldType,
    pub meta_data: MetaData,
}

#[derive(Debug, Clone, Copy)]
enum FieldType {
    String,
    Int,
    Bool,
}

// FIXME: what else can be there?
// FIXME: optional?
#[derive(Debug, Clone, Copy, Default)]
struct MetaData {
    is_password: bool,
    is_text_area: bool,
    is_required: bool,
    is_toggle: bool,
    is_drop_down: bool,
}

// FIXME: how to represent enumeration as a config?
pub trait ConfigTrait: std::fmt::Debug + Send + Sync + 'static {
    fn fields(&self) -> Vec<Field>;
}

// Config Trait should be part of section configuration crate (currently doesn't exist)
impl ConfigTrait for Config {
    fn fields(&self) -> Vec<Field> {
        vec![
            Field {
                name: "daemon",
                ty: FieldType::String,
                meta_data: MetaData {
                    is_drop_down: true,
                    ..Default::default()
                },
            },
            Field {
                name: "truncate",
                ty: FieldType::Bool,
                meta_data: MetaData {
                    is_toggle: true,
                    ..Default::default()
                },
            },
            Field {
                name: "host",
                ty: FieldType::String,
                meta_data: MetaData::default(),
            },
            Field {
                name: "port",
                ty: FieldType::Int,
                meta_data: MetaData::default(),
            },
            Field {
                name: "user",
                ty: FieldType::String,
                meta_data: MetaData::default(),
            },
            Field {
                name: "password",
                ty: FieldType::String,
                meta_data: MetaData {
                    is_password: true,
                    ..Default::default()
                },
            },
            Field {
                name: "database",
                ty: FieldType::String,
                meta_data: MetaData::default(),
            },
            Field {
                name: "query",
                ty: FieldType::String,
                meta_data: MetaData {
                    is_text_area: true,
                    ..Default::default()
                },
            },
        ]
    }
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
                                if field.meta_data.is_drop_down {
                                    div {
                                        class: "flex items-center justify-start",
                                        label {
                                            r#for: "{field.name}",
                                            class: "text-sm font-medium leading-6 text-night-1 uppercase min-w-24",
                                            "{field.name}"
                                        }
                                        select {
                                            name: "{field.name}",
                                            required: field.meta_data.is_required,
                                            class: "p-2 ml-3 min-w-32 rounded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2 focus:outline-none",
                                            option {
                                                value: "One",
                                                "One",
                                            }
                                            option {
                                                value: "Two",
                                                "Two",
                                            }
                                        }
                                    }
                                } else if field.meta_data.is_text_area {
                                    label {
                                        r#for: "{field.name}",
                                        class: "text-sm font-medium leading-6 text-night-1 uppercase",
                                        "{field.name}"
                                    }
                                    // div included here so that textarea below appears on new grid row (ie, below the label)
                                    div {
                                        textarea {
                                            name: "{field.name}",
                                            required: field.meta_data.is_required,
                                            class: "w-full rounded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2 focus:outline-none",
                                        }
                                    }
                                }
                                else if field.meta_data.is_toggle {
                                    div {
                                        class: "flex items-center justify-start",
                                        label {
                                            r#for: "{field.name}",
                                            class: "min-w-24 text-sm font-medium leading-6 text-night-1 uppercase",
                                            "{field.name}"
                                        }
                                        input {
                                            name: "{field.name}",
                                            required: field.meta_data.is_required,
                                            r#type: "checkbox",
                                            class: "ml-3 rounded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2 focus:outline-none",
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
                                            name: "{field.name}",
                                            required: field.meta_data.is_required,
                                            r#type: if field.meta_data.is_password { "password" } else { "text" },
                                            class: "w-full rounded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2 focus:outline-none",
                                        }
                                    }
                                }
                            }
                        }
                        div {
                            class: "justify-self-center",
                            button {
                                prevent_default: "onclick",
                                onclick: move |_| {
                                    selected_node.set(None);
                                },
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
