use std::collections::BTreeMap;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::routing::Route;

#[derive(Debug)]
struct State {
    workspaces: BTreeMap<u64, Workspace>,
    counter: u64,
}

impl State {
    fn new() -> Self {
        Self {
            workspaces: BTreeMap::new(),
            counter: 0,
        }
    }

    fn add_workspace(&mut self, name: &str, _created_at: &str) {
        let id = self.get_id();
        self.workspaces.insert(id, Workspace::new(name, "new"));
    }

    fn remove_workspace(&mut self, id: u64) {
        self.workspaces.remove(&id);
    }

    fn get_id(&mut self) -> u64 {
        let id = self.counter;
        self.counter += 1;
        id
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Workspace {
    name: String,
    // use proper date
    created_at: String,
}

impl Workspace {
    fn new(name: &str, created_at: &str) -> Self {
        Self {
            name: name.into(),
            created_at: created_at.into(),
        }
    }
}

#[component]
fn NewWorkspace() -> Element {
    let mut workspaces_state = use_context::<Signal<State>>();
    let mut render_form_state = use_signal(|| false);
    let render_form = *render_form_state.read();
    rsx! {
        if !render_form {
            div {
                class: "grid",
                button {
                    onclick: move |_| {
                        *render_form_state.write() = true;
                    },
                    class: "text-stem-1 px-4 py-2 rounded bg-forest-2 border border-forest-2",
                    "ADD NEW WORKSPACE"
                }
            }
        } else {
            div {
                class: "relative grid grid-flow-col",
                form {
                    onsubmit: move |event| {
                        tracing::info!("button event: {event:?}");
                        *render_form_state.write() = false;
                        if let Some(name) = event.values().get("workspace_name") {
                            let name = name.as_value();
                            workspaces_state.write().add_workspace(&name, "new");
                        } else {
                            tracing::error!("failed to get value of `workspace_name` from form");
                        }
                    },
                    div {
                        input {
                            class: "border border-night-1 rounded mx-4 py-2 px-2",
                            name: "workspace_name",
                            placeholder: "New Workspace Name",
                        }
                        button {
                            class: "text-stem-1 px-4 py-2 rounded bg-forest-1 border border-forest-1",
                            "CREATE"
                        }
                    }
                }
                button {
                    class: "text-toadstool-1 px-4 py-2 ml-2 rounded border border-toadstool-1",
                    onclick: move |_| { *render_form_state.write() = false; },
                    "CANCEL"
                }
            }
        }
    }
}

#[component]
pub fn Workspaces() -> Element {
    let state = use_context_provider(|| {
        let mut state = State::new();
        state.add_workspace("hello", "2020-01-01");
        state.add_workspace("world", "2020-01-01");
        Signal::new(state)
    });
    rsx! {
    div {
        class: "container mx-auto grid grid-cols-2",
        div {
            class: "pt-5 pl-3 font-bold",
            h2 {
                class: "",
                font_size: "1.5em",
                "Workspaces"
            }
        }
        div {
            class: "pt-5 justify-self-end pr-3",
            NewWorkspace {}
        }
        div {
            id: "table-container",
            class: "col-span-2 pt-4 w-full",
            table {
                class: "table-fix border border-solid text-left w-full mx-auto",
                thead {
                    tr {
                        class: "border-b border-solid p-4 font-bold",
                        th {
                            class: "pl-3",
                            "Name"
                        },
                        th {
                            class: "text-right",
                            "Created At"
                        }
                        th { }
                        }
                        for (_, workspace) in state.read().workspaces.iter() {
                            tr {
                                class: "border-b border-gray-100",
                                td {
                                    Link{
                                        class: "block py-3 pl-3",
                                        to: Route::Workspace { workspace: workspace.name.clone() },
                                        children: rsx! { "{workspace.name}" }
                                    }
                                }
                                td {
                                    class: "text-right",
                                    "{workspace.created_at}"
                                }
                                td {
                                    class: "text-right",
                                    button {
                                        onclick: move |_| {
                                        },
                                        class: "text-toadstool-1 border border-toadstool-1 px-4 py-1 my-1 mx-3 rounded bg-white",
                                        "DELETE"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
