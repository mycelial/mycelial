// FIXME: merge node_state and node_config?
use crate::components::node_state::NodeState;
pub use dioxus::prelude::*;

// example of simple config
// goal of our configuration system is to be:
// 1. type erased - we don't want to write out and maintain every section configuration more than once
// 2. section should own it's configuration
// 3. need registry for node configurations to match node_type to config

// lets start with some very specific configuration for some db-like section source
// that's how specific section will define it's specific config
//
// in order to be able to work in UI with such configuration - we need a way to access
// field/types definitions and some additional metadata, e.g. input can be password or text area, etc.
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
}

// FIXME: what else can be there?
// FIXME: optional?
#[derive(Debug, Clone, Copy, Default)]
struct MetaData {
    is_password: bool,
    is_text_area: bool,
    is_required: bool,
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
        // State load / intermiddiate save to indexed db
        let NodeState { ref config, .. } = *inner_signal.read();
        let config_fields = config.fields();
        return rsx! {
            div {
                class: "absolute grid grid-flow-rows gap-2 border border-solid min-w-[70%] bg-blue-200 top-[200px] left-[200px] z-[100]",
                div {
                    form {
                        for field in config_fields {
                            div {
                                label {
                                    r#for: "{field.name}",
                                    class: "block text-sm font-medium leading-6 text-gray-900",
                                    "{field.name}"
                                }
                                if field.meta_data.is_text_area {
                                    div {
                                        textarea {
                                            name: "{field.name}",
                                            required: field.meta_data.is_required,
                                            class: "block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1",
                                        }
                                    }
                                } else {
                                    div {
                                        input {
                                            name: "{field.name}",
                                            required: field.meta_data.is_required,
                                            r#type: if field.meta_data.is_password { "password" } else { "text" },
                                            class: "block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1",
                                        }
                                    }
                                }
                            }
                        }
                        div {
                            button {
                                prevent_default: "onclick",
                                onclick: move |_| {
                                    selected_node.set(None);
                                },
                                r#type:"submit",
                                class:"flex w-full justify-center rounded-md bg-indigo-600 px-3 py-1.5 text-sm font-semibold leading-6 text-white shadow-sm",
                                "Update Configuration"
                            }
                        }
                    }
                }
            }
        };
    }
    None
}
