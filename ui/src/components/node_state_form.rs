use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

pub use config::prelude::*;
pub use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::app::{WorkspaceOperation, WorkspaceUpdate};

use super::app::ControlPlaneClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeState {
    pub id: Uuid,
    pub x: f64,
    pub y: f64,
    #[serde(skip)]
    pub w: f64,
    #[serde(skip)]
    pub h: f64,
    #[serde(skip)]
    pub port_diameter: f64,
    pub config: Box<dyn config_registry::Config>,
    pub daemon_id: Option<Uuid>,
}

impl Clone for NodeState {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            port_diameter: self.port_diameter,
            config: self.config.clone(),
            daemon_id: self.daemon_id,
        }
    }
}

impl NodeState {
    pub fn new(
        id: Uuid,
        x: f64,
        y: f64,
        config: Box<dyn config_registry::Config>,
        daemon_id: Option<Uuid>,
    ) -> Self {
        Self {
            id,
            x,
            y,
            w: 0.0,
            h: 0.0,
            port_diameter: 12.0,
            config,
            daemon_id,
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

#[derive(Debug, Clone)]
struct FormFieldValue {
    value: String,
    modified: bool,
}

impl FormFieldValue {
    fn new(value: String) -> Self {
        Self {
            value,
            modified: false,
        }
    }

    fn update(&mut self, value: String) {
        self.value = value;
        self.modified = true;
    }
}

impl From<String> for FormFieldValue {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

// Internal form state to keep track of values and validation
#[derive(Debug, Clone)]
struct FormState {
    config: Signal<NodeState>,
    fields: HashMap<String, FormFieldValue>,
}

impl FormState {
    fn new(config: Signal<NodeState>) -> Self {
        let fields = config
            .read()
            .config
            .fields()
            .into_iter()
            .map(|field| (field.name.to_string(), field.value.to_string().into()))
            .collect();
        Self { config, fields }
    }

    // check if field is valid
    fn is_field_valid(&self, field_name: &str) -> bool {
        let value = match self.fields.get(field_name) {
            Some(value) => value,
            None => {
                tracing::error!("no field with name {field_name} found");
                return false;
            }
        };
        self.config
            .read()
            .config
            .validate_field(field_name, value.value.as_str().into())
            .is_ok()
    }

    // check if form is valid
    fn is_valid(&self) -> bool {
        let config = &self.config.read().config;
        self.fields.iter().all(|(key, value)| {
            config
                .validate_field(key, value.value.as_str().into())
                .is_ok()
        })
    }

    fn update_value(&mut self, field_name: &str, value: String) {
        if let Some(entry) = self.fields.get_mut(field_name) {
            entry.update(value)
        }
    }

    fn get_value(&self, field_name: &str) -> &str {
        self.fields.get(field_name).unwrap().value.as_str()
    }
}

impl IntoIterator for FormState {
    type IntoIter = <HashMap<String, FormFieldValue> as IntoIterator>::IntoIter;
    type Item = (String, FormFieldValue);

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

#[component]
pub fn NodeStateForm(
    workspace: Rc<str>,
    control_plane_client: ControlPlaneClient,
    selected_node: Signal<Option<Signal<NodeState>>>,
) -> Element {
    let node_state = match *selected_node.read() {
        None => return None,
        Some(signal) => signal,
    };
    let NodeState { id, ref config, .. } = *node_state.read();
    let node_type = config.name();

    // Create hashmap with field names => field values from node config.
    // This hashmap will be used as a temporary state holder, which will allow
    // to not update values of the config until 'save' button is pressed
    let mut form_state: Signal<Option<FormState>> = use_signal(|| None);
    // Peeking to avoid re-render on write
    if form_state.peek().is_none() {
        *form_state.write() = Some(FormState::new(node_state));
    }
    let fs_state = &*form_state.read();
    let fs = fs_state.as_ref().unwrap();

    // since field.name is not &'static str, we need to clone it so all event handlers can capture field name by value
    let config_fields = config.fields().into_iter().map(|field| {
        // use value from form_state
        let value = fs.get_value(field.name);
        (field.name.to_string(), field, value)
    });
    return rsx! {
        div {
            class: "border border-solid rounded-md drop-shadow px-5 py-4 mt-4 mx-4",
            div {
                form {
                    // config save
                    onsubmit: {
                        let workspace = Rc::clone(&workspace);
                        move |_event| {
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
                            let mut config_updated_fields: HashSet<String> = HashSet::new();
                            for (field_name, field_value) in form_state.into_iter() {
                                if !field_value.modified {
                                    continue
                                }
                                match config.set_field_value(field_name.as_str(), field_value.value.as_str().into()) {
                                    Ok(_) => {
                                        config_updated_fields.insert(field_name);
                                    }
                                    Err(e) => {
                                        tracing::error!("failed to set field value for {field_name} with value {}: {e}", field_value.value.as_str());
                                    }
                                };
                            }
                            let config_update = RawConfig::new(config.name()).with_fields(
                                config.fields().into_iter().filter(|field| config_updated_fields.contains(field.name))
                            );
                            // do not store secrets in runtime
                            config.strip_secrets();
                            control_plane_client.update_workspace(WorkspaceUpdate::new(
                                &workspace,
                                vec![WorkspaceOperation::UpdateNodeConfig{ id, config: config_update }]
                            ));

                        // FIXME: update daemon field once daemon field is added
                        }
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
                    for (field_name, field, field_value) in config_fields {
                        div {
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
                                        readonly: field.metadata.is_read_only,
                                        onchange: move |event| {
                                            if let Some(form_state) = &mut *form_state.write() {
                                                form_state.update_value(field_name.as_str(), event.value());
                                            }
                                        },
                                        checked: "{field_value}",
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
                                        readonly: field.metadata.is_read_only,
                                        oninput: move |event| {
                                            if let Some(form_state) = &mut *form_state.write() {
                                                form_state.update_value(field_name.as_str(), event.value())
                                            }
                                        },
                                        value: "{field_value}",
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
                                        readonly: field.metadata.is_read_only,
                                        oninput: move |event| {
                                            if let Some(form_state) = &mut *form_state.write() {
                                                form_state.update_value(field_name.as_str(), event.value())
                                            }
                                        },
                                        value: "{ field_value }",
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
