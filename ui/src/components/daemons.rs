use std::{collections::BTreeMap, rc::Rc};

use crate::components::{
    app::{AppError, ControlPlaneClient, Daemon},
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
        daemon
            .last_seen
            .as_mut()
            .map(|time| *time = time.with_nanosecond(0).unwrap());
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
        daemon.name.as_str(),
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
pub fn Daemons() -> Element {
    let control_plane_client = use_context::<ControlPlaneClient>();
    let mut daemons_state = use_signal(DaemonsState::new);
    let _state_fetcher: Resource<Result<(), AppError>> = use_resource(move || async move {
        let daemons = control_plane_client.get_daemons().await.map_err(|e| {
            tracing::error!("failed to fetch daemons: {e}");
            e
        })?;
        let state = &mut *daemons_state.write();
        daemons
            .into_iter()
            .for_each(|daemon| state.add_daemon(daemon));
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
                        class: "table-fix border border-solid text-left w-full mx-auto",
                        thead {
                            tr {
                                class: "border-b border-solid p-4 font-bold bg-night-1/25",
                                th { class: "pl-3 w-1/6", "Id" },
                                th { class: "pl-3 w-1/12", "Name" },
                                th { class: "pl-3 w-1/12", "Address" },
                                th { class: "pl-3 w-1/12", "Last Seen" },
                                th { class: "pl-3 w-1/12", "Created At" },
                                th { class: "pl-3 w-1/12", "Status" },
                                th { class: "pl-3 w-1/12" },
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
                                td { class: "pl-3", "{name}" }
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
