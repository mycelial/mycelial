use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub use config::prelude::*;
pub use dioxus::prelude::*;

pub type NodeType = std::borrow::Cow<'static, str>;
pub type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> =
    std::result::Result<T, E>;

#[derive(Debug, Default, Config)]
#[section(input=dataframe, output=dataframe)]
pub struct ConfigExample {
    host: String,
    #[validate(min = 1)]
    port: u16,
    user: String,
    #[field_type(password)]
    password: String,
    database: String,
    truncate: bool,
    #[field_type(text_area)]
    query: String,
}

pub struct ConfigRegistry {}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NodeState {
    pub id: u64,
    pub x: f64,
    pub y: f64,
    #[serde(skip)]
    pub w: f64,
    #[serde(skip)]
    pub h: f64,
    #[serde(skip)]
    pub port_diameter: f64,
    pub node_type: NodeType,
    pub config: Box<dyn config::Config>,
}

impl NodeState {
    pub fn new(id: u64, node_type: NodeType, x: f64, y: f64) -> Self {
        Self {
            id,
            node_type,
            x,
            y,
            w: 0.0,
            h: 0.0,
            port_diameter: 12.0,
            config: Box::<ConfigExample>::default(),
        }
    }

    pub fn input_pos(&self) -> (f64, f64) {
        let offset = self.port_diameter / 2.0;
        (self.x - offset, self.y + self.h / 2.0 - offset)
    }

    pub fn output_pos(&self) -> (f64, f64) {
        let offset = self.port_diameter / 2.0;
        (self.x - offset + self.w, self.y + self.h / 2.0 - offset)
    }
}

// Internal form state to keep track of values and validation
#[derive(Clone)]
struct FormState {
    config: Signal<NodeState>,
    fields: Rc<RefCell<HashMap<String, String>>>,
}

impl FormState {
    fn new(config: Signal<NodeState>) -> Self {
        let fields = config
            .read()
            .config
            .fields()
            .into_iter()
            .map(|field| (field.name.to_string(), field.value.to_string()))
            .collect();
        Self {
            config,
            fields: Rc::new(RefCell::new(fields)),
        }
    }

    // check if field is valid
    fn is_field_valid(&self, field_name: &str) -> bool {
        let borrow = self.fields.borrow();
        let value = match borrow.get(field_name) {
            Some(value) => value,
            None => {
                tracing::error!("no field with name {field_name} found");
                return false;
            }
        };
        self.config
            .read()
            .config
            .validate_field(field_name, value)
            .is_ok()
    }

    // check if form is valid
    fn is_valid(&self) -> bool {
        let borrow = self.fields.borrow();
        let config = &self.config.read().config;
        borrow
            .iter()
            .all(|(key, value)| config.validate_field(key, value).is_ok())
    }

    fn update_value(&mut self, field_name: &str, value: String) {
        let mut borrow = self.fields.borrow_mut();
        match borrow.get_mut(field_name) {
            Some(entry) => {
                *entry = value;
            }
            None => (),
        }
    }
}

impl IntoIterator for FormState {
    type IntoIter = <HashMap<String, String> as IntoIterator>::IntoIter;
    type Item = (String, String);

    fn into_iter(self) -> Self::IntoIter {
        let inner = self.fields.take();
        inner.into_iter()
    }
}

// FIXME: config needs to be loaded from backend
// How to deal with configuration drift?
// Technically we can carefully update config sections and add new fields with default values,
// but if rename happens for some unavoidable reason - how to notify that to user?
// State load / intermediate save to indexed db
#[component]
pub fn NodeStateForm(selected_node: Signal<Option<Signal<NodeState>>>) -> Element {
    let node_state = match *selected_node.read() {
        None => return None,
        Some(signal) => signal,
    };
    let NodeState {
        id,
        ref node_type,
        ref config,
        ..
    } = *node_state.read();

    // Create hashmap with field names => field values from node config.
    // This hashmap will be used as a temporary state holder, which will allow
    // to not update values of the config until 'save' button is pressed
    let mut form_state: Signal<Option<FormState>> = use_signal(|| None);
    if form_state.read().is_none() {
        *form_state.write() = Some(FormState::new(node_state));
    }
    let fs = form_state.read().as_ref().map(|form| form.clone()).unwrap();

    // since field.name is not &'static str, we need to clone it so all event handlers can capture field name by value
    let config_fields = config
        .fields()
        .into_iter()
        .map(|field| (field.name.to_string(), field));
    return rsx! {
        div {
            class: "border border-solid rounded-md drop-shadow px-5 py-4 mt-4 mx-4",
            div {
                form {
                    onsubmit: move |_event| {
                        // if form is invalid: do nothing
                        if let Some(false) = form_state.read().as_ref().map(|fs| fs.is_valid()) {
                            return
                        }

                        let mut node_state = match selected_node.write().take() {
                            Some(state) => state,
                            None => return,
                        };
                        let form_state = match form_state.write().take() {
                            Some(state) => state,
                            None => return,
                        };
                        let config = &mut node_state.write().config;
                        for (field_name, field_value) in form_state.into_iter() {
                            match config.set_field_value(field_name.as_str(), field_value.as_str()) {
                                Ok(_) => (),
                                Err(e) =>
                                    tracing::error!("failed to set field value for {field_name} with value {field_value}: {e}"),
                            };
                        }
                        // FIXME: update daemon field once daemon field is added
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
                    // FIXME: add daemon field
                    for (field_name, field) in config_fields {
                        div {
                          //if field.ty.is_vec() {
                          //    div {
                          //        class: "flex items-center justify-start",
                          //        label {
                          //            r#for: "{field_name}",
                          //            class: "text-sm font-medium leading-6 text-night-1 uppercase min-w-24",
                          //            "{field_name}"
                          //        }
                          //        select {
                          //            oninput: move |event| {
                          //                tracing::info!("input event: {:?}", event);
                          //            },
                          //            id: "{field_name}",
                          //            name: "{field_name}",
                          //            class: "p-2 ml-3 min-w-32 r unded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2 focus:outline-none",
                          //            class: if fs.is_field_valid(&field_name) { "" } else { "outline outline-red-500" },
                          //            //multiple: "true",
                          //            option {
                          //                value: "*",
                          //                "*",
                          //            }
                          //        }
                          //    }
                          //} else
                            if field.ty.is_bool() {
                                div { class: "flex items-center justify-start",
                                    label {
                                        r#for: "{field_name}",
                                        class: "min-w-24 text-sm font-medium leading-6 text-night-1 uppercase",
                                        "{field.name}"
                                    }
                                    input {
                                        id: "{field.name}",
                                        name: "{field.name}",
                                        r#type: "checkbox",
                                        class: "ml-3 rounded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2 focus:outline-none",
                                        value: "{field.value}",
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
                                        class: if fs.is_field_valid(&field_name) { "" } else { "outline outline-red-500" },
                                        value: "{field.value}",
                                    }
                                }
                            } else {
                                // returns basic text input
                                div {
                                    label {
                                        r#for: "{field_name}",
                                        class: "text-sm font-medium leading-6 text-night-1 uppercase",
                                        "{field.name}"
                                    }
                                    input {
                                        id: "{field.name}",
                                        name: "{field.name}",
                                        r#type: if field.metadata.is_password { "password" } else { "text" },
                                        autocomplete: "off",
                                        class: "w-full rounded-md py-1.5 text-gray-900 drop-shadow-sm ring-1 ring-night-1 focus:ring-2 focus:ring-night-2",
                                        class: if fs.is_field_valid(&field_name) { "" } else { "outline outline-red-500" },
                                        oninput: move |event| {
                                            if let Some(form_state) = &mut *form_state.write() {
                                                form_state.update_value(field_name.as_str(), event.value())
                                            }
                                        },
                                        value: "{field.value}",
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
                            class: if !fs.is_valid() { "opacity-50 cursor-not-allowed "} else { "" },
                            "Save"
                        }
                        button {
                            prevent_default: "onclick",
                            onclick: move |_| {
                                selected_node.set(None);
                                form_state.set(None);
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
