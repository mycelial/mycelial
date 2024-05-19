use crate::components::routing::Route;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct State {
    daemons: Vec<Daemon>,
}

impl State {
    fn new() -> Self {
        Self {
            daemons: Vec::<Daemon>::new(),
        }
    }

    fn add_daemon(
        &mut self,
        name: String,
        id: String,
        address: String,
        active_sections: u64,
        active_pipelines: u64,
        // TODO use proper date for last_seen and created_at
        last_seen: String,
        created_at: String,
        // TODO change to enum?
        status: String,
    ) {
        // let id = self.get_id();
        self.daemons.push(Daemon::new(
            &name,
            &id,
            &address,
            active_sections,
            active_pipelines,
            &last_seen,
            &created_at,
            &status,
        ));
    }

    fn remove_daemon(&mut self, id: &str) {
        // delete the daemon in self.daemons where the id matches the id in params
        self.daemons.retain(|daemon| daemon.id != id);
    }

    // Function to check if there are any daemons
    fn has_daemons(&self) -> bool {
        !self.daemons.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Daemon {
    name: String,
    id: String,
    address: String,
    active_sections: u64,
    active_pipelines: u64,
    // TODO use proper date for last_seen and created_at
    last_seen: String,
    created_at: String,
    status: String,
}

impl Daemon {
    fn new(
        name: &str,
        id: &str,
        address: &str,
        active_sections: u64,
        active_pipelines: u64,
        last_seen: &str,
        created_at: &str,
        status: &str,
    ) -> Self {
        Self {
            name: name.into(),
            id: id.into(),
            address: address.into(),
            active_sections,
            active_pipelines,
            last_seen: last_seen.into(),
            created_at: created_at.into(),
            status: status.into(),
        }
    }
}

#[component]
fn NewDaemon() -> Element {
    let _workspaces_state = use_context::<Signal<State>>();
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
                    onsubmit: move |event| {
                        tracing::info!("button event: {event:?}");
                        *render_form_state.write() = false;
                        if let Some(name) = event.values().get("workspace_name") {
                            let _name = name.as_value();
                            // workspaces_state.write().add_workspace(&name, "new");
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
pub fn Daemons() -> Element {
    let mut daemon_state = use_context_provider(|| {
        let mut daemon_state = State::new();
        daemon_state.add_daemon(
            "GCP Daemon".to_string(),
            "ID-1000001".to_string(),
            "101.74.17.73".to_string(),
            3,
            4,
            "2024-04-12 12:12:12".to_string(),
            "2024-05-16 08:34:44".to_string(),
            "HEALTHY".to_string(),
        );
        daemon_state.add_daemon(
            "Azure Daemon".to_string(),
            "ID-1000002".to_string(),
            "101.74.17.73".to_string(),
            3,
            4,
            "2024-04-12 12:12:12".to_string(),
            "2024-05-16 08:34:44".to_string(),
            "HEALTHY".to_string(),
        );
        daemon_state.add_daemon(
            "Edge Compute Daemon".to_string(),
            "ID-1000003".to_string(),
            "101.74.17.73".to_string(),
            3,
            4,
            "2024-04-12 12:12:12".to_string(),
            "2024-05-16 08:34:44".to_string(),
            "DEGRADED".to_string(),
        );
        Signal::new(daemon_state)
    });

    let state_ref = &*daemon_state.read();

    rsx! {
    div {
        class: "container mx-auto grid grid-cols-2",
        div {
            class: "pt-5 pl-3 font-bold",
            h2 {
                class: "",
                font_size: "1.5em",
                "Daemons"
            }
        }
        div {
            class: "pt-5 justify-self-end pr-3",
            // NewWorkspace {}
        }

        if state_ref.has_daemons() {
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
                    class: "text-right pl-3",
                    "ID"
                    },
                    th {
                    class: "text-right pl-3",
                    "Address"
                    },
                    th {
                    class: "text-right pl-3",
                    "Active Sections"
                    },
                    th {
                    class: "text-right pl-3",
                    "Active Pipelines"
                    },
                    th {
                    class: "text-right pl-3",
                    "Last Seen"
                    },
                    th {
                    class: "text-right px-3",
                    "Created At"
                    },
                    th {
                        class: "text-right px-3",
                        "Status"
                    },
                    th {},
                }
                }
                for daemon in state_ref.daemons.iter().map(|daemon| daemon.clone() ) {
                tr {
                    class: "border-b border-gray-100",
                    td {
                    class: "px-1",
                    Link {
                        class: "block py-3 pl-3 hover:underline",
                        to: Route::Daemon { daemon: daemon.id.clone() },
                        children: rsx! { "{daemon.name}" }
                        }
                    }
                    td {
                    class: "text-right px-1",
                    "{daemon.id}"
                    }
                    td {
                    class: "text-right px-1",
                    "{daemon.address}"
                    }
                    td {
                    class: "text-right px-1",
                    "{daemon.active_sections}"
                    }
                    td {
                    class: "text-right px-1",
                    "{daemon.active_pipelines}"
                    }
                    td {
                    class: "text-right px-1",
                    "{daemon.last_seen}"
                    }
                    td {
                    class: "text-right px-1",
                    "{daemon.created_at}"
                    }
                    if daemon.status == "HEALTHY" {
                        td {
                            class: "text-right px-1 text-forest-2",
                            "{daemon.status}"
                            }
                    } else {
                        td {
                            class: "text-right px-1 text-toadstool-1",
                            "{daemon.status}"
                        }
                    }
                    td {
                        class: "text-right px-1",
                        button {
                            onclick: move |_| {
                                daemon_state.write().remove_daemon(&daemon.id);
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

            }
        }
}
