use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct State {
    workspaces: Vec<Workspace> 
}

impl State {
    fn new() -> Self {
        Self {
            workspaces: vec![
                Workspace::new("hello", "2020-01-01"),
                Workspace::new("world", "2020-01-01"),
            ]
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Workspace {
    name: String,
    // use proper date
    created_at: String
}

impl Workspace {
    fn new(name: &str, created_at: &str) -> Self {
        Self { name: name.into(), created_at: created_at.into() }
    }
}

#[component]
fn NewWorkspace() -> Element {
    let mut workspaces_state = use_context::<Signal<State>>();
    let mut render_form_state = use_signal(|| false);
    let render_form = *render_form_state.read();
    rsx! {
        if !render_form {
            button {
                onclick: move |_| {
                    *render_form_state.write() = true;
                },
                class: "flex-initial text-white px-4 py-2 rounded",
                style: "background-color: #1a237e",
                "ADD NEW WORKSPACE"
            }
        } else {
            div {
                class: "relative",
                form {
                    onsubmit: move |event| {
                        tracing::info!("button event: {event:?}");
                        *render_form_state.write() = false;
                        if let Some(name) = event.values().get("workspace_name") {
                            let name = name.as_value();
                            workspaces_state
                                .write()
                                .workspaces
                                .push(Workspace::new(&name, "new"));
                        } else {
                            tracing::error!("failed to get value of `workspace_name` from form");
                        }
                    },
                    h2 {
                        class: "m-w-max text-left",
                        "New Workspace",
                    },
                    input {
                        class: "m-w-max",
                        name: "workspace_name",
                        placeholder: "Name *",
                    }
                    button {
                        class: "text-white px-4 py-2 rounded",
                        style: "background-color: #1a237e",
                        "CREATE NEW WORKSPACE"
                    }
                }
            }
        }
    }
}

#[component]
pub fn Workspaces() -> Element {
    let state = use_context_provider(|| Signal::new(State::new()));
    rsx! {
        div {
            class: "flex pt-4 m-w-max",
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
                NewWorkspace {}
            }
            div {
                class: "w-1/12",
            }
        }
        div {
            class: "flex pt-4",
            div {
                class: "w-1/12",
            }
            table {
                class: "w-10/12 table-fix border border-solid text-left",
                thead {
                    tr {
                        class: "border-b border-solid p-4 font-bold",
                        th { "Name" },
                        th { "Created At"}
                        th { }
                    }
                    for workspace in state.read().workspaces.as_slice() {
                        tr {
                            th {
                                "{workspace.name}"
                            }
                            th {
                                "{workspace.created_at}"
                            }
                            th {}
                        }
                    }
                }
            }
            div {
                class: "w-1/12",
            }
        }
    }
}
