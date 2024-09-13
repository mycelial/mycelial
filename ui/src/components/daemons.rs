use std::{collections::BTreeMap, rc::Rc};

use crate::components::{
    app::{AppError, ControlPlaneClient, Daemon},
    icons::Edit,
    routing::Route,
};
use chrono::Timelike;
use dioxus::prelude::*;
use uuid::Uuid;

use super::app::DaemonStatus;

#[derive(Debug)]
struct DaemonsState {
    map: BTreeMap<Uuid, Rc<Daemon>>,
}

impl DaemonsState {
    fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    fn add_daemon(&mut self, mut daemon: Daemon) {
        // strip nanoseconds, unwrap is safe since '0' will not overflow second;
        daemon.last_seen = daemon
            .last_seen
            .map(|time| time.with_nanosecond(0).unwrap());
        daemon.joined_at = daemon.joined_at.with_nanosecond(0).unwrap();
        self.map.insert(daemon.id, Rc::new(daemon));
    }

    fn remove_daemon(&mut self, id: Uuid) {
        self.map.remove(&id);
    }

    // Function to check if there are any daemons
    fn has_daemons(&self) -> bool {
        !self.map.is_empty()
    }
}

pub fn render(
    daemon: &Rc<Daemon>,
) -> (Rc<Daemon>, String, &str, &str, String, String, DaemonStatus) {
    (
        Rc::clone(daemon),
        daemon.id.to_string(),
        daemon.name.as_deref().unwrap_or(""),
        daemon.address.as_deref().unwrap_or(""),
        daemon
            .last_seen
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default(),
        daemon.joined_at.to_string(),
        daemon.status,
    )
}

#[component]
fn EditDaemonName(
    edit_daemon_name: Signal<Option<Uuid>>,
    name: String,
    restart_fetcher: Signal<()>,
) -> Element {
    let control_plane_client = use_context::<ControlPlaneClient>();
    rsx! {
        div {
            class: "w-full",
            form {
                class: "grid gap-1 grid-cols-6 gap-1",
                onsubmit: move |event| async move {
                    let id = match *edit_daemon_name.read() {
                        Some(id) => id,
                        None => return
                    };
                    if let Some(name) = event.values().get("new_daemon_name") {
                        let name = name.as_value();
                        let name = match name.as_str() {
                            "" => None,
                            name => Some(name),
                        };
                        if let Err(e) = control_plane_client.set_daemon_name(id, name).await {
                            tracing::error!("failed to create workspace {name:?}: {e}");
                        }
                    } else {
                        tracing::error!("failed to get value of `new_daemon_name` from form");
                    }
                    edit_daemon_name.set(None);
                    restart_fetcher.set(());
                },
                input {
                    class: "col-span-4 border border-night-1 rounded py-1",
                    name: "new_daemon_name",
                    value: "{name}"
                }
                button {
                    class: "text-stem-1 py-1 rounded bg-forest-2 border border-forest-1",
                    "UPDATE"
                }
                button {
                    class: "text-toadstool-1 py-1 rounded border border-toadstool-1 hover:text-white hover:bg-toadstool-2",
                    onclick: move |_| { edit_daemon_name.set(None); },
                    "CANCEL"
                }
            }
        }
    }
}

#[component]
pub fn Daemons() -> Element {
    let control_plane_client = use_context::<ControlPlaneClient>();
    let mut daemons_state = use_signal(DaemonsState::new);
    let mut edit_daemon_name: Signal<Option<Uuid>> = use_signal(|| None);
    let mut restart_fetcher = use_signal(|| ());
    let _state_fetcher: Resource<Result<(), AppError>> = use_resource(move || async move {
        let daemons = control_plane_client.get_daemons().await.map_err(|e| {
            tracing::error!("failed to fetch daemons: {e}");
            e
        })?;
        let state = &mut *daemons_state.write();
        daemons
            .into_iter()
            .for_each(|daemon| state.add_daemon(daemon));
        restart_fetcher.read();
        Ok(())
    });
    let state_ref = &*daemons_state.read();
    rsx! {
        div {
            class: "container mx-auto grid grid-cols-2",
            div {
                class: "pt-5 pl-3 font-bold",
                h2 {
                    font_size: "1.5em",
                    "Daemons"
                }
            }
            div {
                class: "pt-5 justify-self-end pr-3",
                Link {
                    to: Route::DaemonTokens{},
                    children: rsx!{
                        div {
                            class: "text-stem-1 px-4 py-2 rounded bg-forest-2 border border-forest-2 hover:bg-forest-3 hover:text-stem-1",
                            "ADD NEW DAEMON"
                        }
                    }
                }
            }

            if state_ref.has_daemons() {
                div {
                    id: "table-container",
                    class: "col-span-2 pt-4 w-full",
                    table {
                        class: "table-fix border border-solid text-left w-full mx-auto border-1",
                        thead {
                            tr {
                                class: "border-b border-solid p-4 font-bold bg-night-1/25",
                                th { class: "pl-3 w-[20%]", "Id" },
                                th { class: "pl-3 w-[30%]", "Name" },
                                th { class: "pl-3 w-[15%]", "Address" },
                                th { class: "pl-3 w-[12.5%]", "Last Seen" },
                                th { class: "pl-3 w-[12.5%]", "Created At" },
                                th { class: "pl-3 w-[5%]", "Status" },
                                th { class: "pl-3 w-[5%]" },
                            }
                        }
                        for (daemon, id, name, address, last_seen, joined_at, status) in state_ref.map.values().map(render) {
                            tr {
                                class: "border-b border-gray-100",
                                td {
                                    class: "pl-3",
                                    Link {
                                        class: "block hover:underline",
                                        to: Route::Daemon { daemon: id },
                                        children: rsx! { "{daemon.id}" }
                                    }
                                }
                                td {
                                    class: "pl-3",
                                    match *edit_daemon_name.read() == Some(daemon.id) {
                                        false => rsx!{
                                            div {
                                                class: "flex items-center",
                                                div {
                                                    class: "pr-2",
                                                    onclick: {
                                                        let id = daemon.id;
                                                        move |_| {
                                                            edit_daemon_name.set(Some(id))
                                                        }
                                                    },
                                                    Edit{}
                                                }
                                                "{name}"
                                            },
                                        },
                                        true => rsx!{ EditDaemonName { edit_daemon_name, name, restart_fetcher} }
                                    }
                                }
                                td { class: "pl-3", "{address}" }
                                td { class: "pl-3", "{last_seen}" }
                                td { class: "pl-3", "{joined_at}" }
                                td { class: "pl-3", "{status}" }
                                td {
                                    class: "pl-3 text-right",
                                    button {
                                        onclick: move |_| {
                                            let daemon = Rc::clone(&daemon);
                                            async move {
                                                match control_plane_client.remove_daemon(daemon.id).await  {
                                                    Ok(()) => daemons_state.write().remove_daemon(daemon.id),
                                                    Err(e) => tracing::error!("failed to remove daemon: {e}"),
                                                };
                                                restart_fetcher.set(())
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
        }
    }
}
