use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    components::{app::AppState, routing::Route},
    StdError,
};

#[derive(Debug)]
struct WorkspacesState {
    workspaces: BTreeMap<u64, Workspace>,
    counter: u64,
}

impl WorkspacesState {
    fn new() -> Self {
        Self {
            workspaces: BTreeMap::new(),
            counter: 0,
        }
    }

    fn add_workspace(&mut self, name: String, created_at: DateTime<Utc>) {
        let id = self.get_id();
        self.workspaces.insert(id, Workspace::new(name, created_at));
    }
    
    fn get_workspace(&self, id: u64) -> Option<&Workspace> {
        self.workspaces.get(&id)
    }

    fn remove_workspace(&mut self, id: u64) {
        self.workspaces.remove(&id);
    }

    fn get_id(&mut self) -> u64 {
        let id = self.counter;
        self.counter += 1;
        id
    }

    fn reset(&mut self) {
        self.workspaces.clear();
    }

    // Function to check if there are any workspaces
    fn has_workspaces(&self) -> bool {
        !self.workspaces.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl Workspace {
    fn new(name: String, created_at: DateTime<Utc>) -> Self {
        Self { name, created_at }
    }
}

#[component]
fn NewWorkspace(
    mut restart_fetcher: Signal<bool>,
) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut render_form_state = use_signal(|| false);
    rsx! {
        if !*render_form_state.read() {
            div {
                class: "grid",
                button {
                    onclick: move |_| {
                        *render_form_state.write() = true;
                    },
                    class: "text-stem-1 px-4 py-2 rounded bg-forest-2 border border-forest-2 hover:bg-forest-3 hover:text-stem-1",
                    "ADD NEW WORKSPACE"
                }
            }
        } else {
            div {
                class: "relative grid grid-flow-col",
                form {
                    onsubmit: move |event| async move {
                        *render_form_state.write() = false;
                        if let Some(name) = event.values().get("workspace_name") {
                            let name = name.as_value();
                            match app_state.read().create_workspace(&name).await {
                                Ok(_) => {
                                    restart_fetcher.set(true);
                                },
                                Err(e) => tracing::error!("failed to create workspace {name}: {e}"),
                            }
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
                    class: "text-toadstool-1 px-4 py-2 ml-2 rounded border border-toadstool-1 hover:text-white hover:bg-toadstool-2",
                    onclick: move |_| { *render_form_state.write() = false; },
                    "CANCEL"
                }
            }
        }
    }
}

#[component]
pub fn Workspaces() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut workspaces_state = use_signal(|| WorkspacesState::new());
    let mut restart_fetcher = use_signal(|| false);
    let mut fetcher: Resource<Result<usize, StdError>> = use_resource(move || async move {
        let app_state_ref = &*app_state.read();
        let workspaces = app_state_ref.read_workspaces().await?;
        let workspaces_len = workspaces.len();
        {
            let state_ref = &mut *workspaces_state.write();
            state_ref.reset();
            workspaces.into_iter().for_each(|workspace| {
                state_ref.add_workspace(workspace.name, workspace.created_at)
            });
        }
        Ok(workspaces_len)
    });
    if *restart_fetcher.read() {
        fetcher.restart();
        restart_fetcher.set(false);
    }
    let child = match &*fetcher.read_unchecked() {
        Some(Ok(0)) => rsx!{
            div {
                class: "mt-10 p-4 w-9/12 bg-moss-3 text-black drop-shadow-md rounded-sm mx-auto col-span-2",
                div {
                    class: "py-2 my-2 bg-white text-night-2 shadow-none",
                    h3 {
                        class: "text-lg ml-2 py-2",
                        "Create your first workspace to start building pipelines!"
                    }
                    div {
                        class: "bg-grey-bright mx-2 p-2 rounded",
                        "Mycelial pipelines are organized into groups called Workspaces. Click the \"Add New Workspace\" button above to create your first workspace."
                    }
                }
            }
        },
        Some(Ok(_)) => rsx! {
            div {
                id: "table-container",
                class: "col-span-2 pt-4 w-full",
                table {
                    class: "table-fix border border-solid text-left w-full mx-auto",
                    thead {
                        tr {
                            class: "border-b border-solid p-4 font-bold bg-night-1/25",
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
                        for (&id, workspace) in workspaces_state.read().workspaces.iter() {
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
                                        onclick: move |_| async move {
                                            let name = match (&mut *workspaces_state.write()).get_workspace(id) {
                                                Some(Workspace{ ref name, .. }) => name.to_string(),
                                                None => return
                                            };
                                            match app_state.read().remove_workspace(name.as_str()).await {
                                                Ok(_) => restart_fetcher.set(true),
                                                Err(e) => tracing::error!("failed to remove workspace: {e}"),
                                            }
                                        },
                                        class: "text-toadstool-1 border border-toadstool-1 px-4 py-1 my-1 mx-3 rounded bg-white hover:text-white hover:bg-toadstool-2",
                                        "DELETE"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        Some(Err(e)) => {
            tracing::error!("error reading workspaces: {e}");
            rsx! {
                div {
                    "error loading workspaces"
                }
            }
        }
        None => rsx! {
            div {
                "loading"
            }
        },
    };
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
                NewWorkspace { restart_fetcher }
            }
            { child }
        }
    }
}
