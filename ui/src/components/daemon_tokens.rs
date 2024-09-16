use std::collections::BTreeMap;

use crate::components::{app::ControlPlaneClient, icons::Copy};
use chrono::Timelike as _;
use dioxus::prelude::*;
use uuid::Uuid;

use super::app::Token;

#[derive(Debug)]
struct TokensState {
    tokens: BTreeMap<Uuid, Token>,
}

impl TokensState {
    fn new() -> Self {
        Self {
            tokens: BTreeMap::new(),
        }
    }

    fn add_token(&mut self, token: Token) {
        self.tokens.insert(token.id, token);
    }

    fn reset(&mut self) {
        self.tokens.clear()
    }

    fn remove_token(&mut self, id: Uuid) {
        self.tokens.remove(&id);
    }
}

// FIXME:
// 1. instructions are outdated
// 2. links to external docs
#[component]
fn Doc() -> Element {
    rsx! {
        div {
            class: "my-2 p-4 w-9/12 bg-moss-3 text-black drop-shadow-md rounded-sm mx-auto",
            id: "daemon-join-instructions-container",

            div {
                // TODO: Confirm that the join command is correct.
                id: "daemon-join-instructions",
                class: "py-2 my-4 bg-white text-night-2 shadow-none",
                h3 {
                    class: "text-lg ml-2 py-2 uppercase",
                    "Adding a New Mycelial Daemon to Your Mycelial Network"
                }
                p {
                    class: "mb-2 ml-2",
                    "To add a Mycelial Daemon to this Mycelial network, click the 'Add New Token' button below to generate a new one-time use join token."
                }
                p {
                    class: "mb-2 ml-2",
                    "Copy the token and run the following command in your terminal to join the Mycelial Daemon to this Mycelial network:"
                }
                div {
                    class: "bg-grey-bright mx-2 px-2 py-2 rounded",
                    p {
                        class: "mb-2 ml-2",
                        "./myceliald -- join --control-plane-url=http://localhost:8000 --control-plane-tls-url=https://localhost:8010 --join-token  <YOUR NEW JOIN TOKEN GOES HERE>"
                    }
                }
                p {
                    class: "mb-2 ml-2 mt-2",
                    "Once joined, your Daemon will provide a confirmatory message in the terminal."
                }
                div {
                    class: "bg-grey-bright mx-2 px-2 py-2 rounded",
                    p {
                        class: "mb-2 ml-2",
                        "$ 2024-09-15T15:52:19.351601Z  INFO myceliald: join successful"
                    }
                }
                p {
                    class: "mb-2 ml-2 mt-2",
                    "All of your Mycelial Daemons, along with their most recent status metrics, are displayed in the table below."
                }
                p {
                    class: "mb-2 ml-2 mt-2",
                    "To start your Mycelial Daemon, run the following command in your terminal:"
                }
                div {
                    class: "bg-grey-bright mx-2 px-2 py-2 rounded",
                    p {
                        class: "mb-2 ml-2",
                        "./myceliald"
                    }
                }

            }
                div {
                    // TODO: Confirm that the join command is correct.
                    id: "daemon-join-additional-info",
                    class: "py-2 my-4 bg-white text-night-2 shadow-none",
                h3 {
                    class: "text-lg ml-2 py-2 uppercase",
                    "Additional Information"
                }
                p {
                    class: "mb-2 ml-2 mt-2",
                    "As the name implies, join tokens are one-time use only."
                }
                p {
                    class: "mb-2 ml-2 mt-2",
                    "When your Mycelial Daemon attempts to joins the network, it presents the join token for validation. If the token is valid, the Daemon is added to the network and the token is marked as used. If the token is invalid, the Daemon is not added to the network."
                }
                p {
                    class: "mb-2 ml-2 mt-2",
                    "Subsequent peer-to-peer communications between Mycelial Daemons, as well as communications between Mycelial Daemons and the Mycelial Control Plane, use mutual TLS authentication. This ensures that all communications are secure and authenticated."
                }
            }
        }
    }
}

#[component]
pub fn DaemonTokens() -> Element {
    let control_plane_client = use_context::<ControlPlaneClient>();
    let mut tokens_state = use_signal(TokensState::new);
    let _state_fetcher = use_resource(move || async move {
        match control_plane_client.get_tokens().await {
            Ok(tokens) => {
                let tokens_state = &mut *tokens_state.write();
                tokens_state.reset();
                tokens
                    .iter()
                    .cloned()
                    .for_each(|token| tokens_state.add_token(token));
            }
            Err(e) => {
                tracing::error!("failed to fetch tokens state: {e}");
            }
        }
    });
    let state_ref = &*tokens_state.read();

    let tokens_table = match state_ref.tokens.is_empty() {
        true => None,
        false => {
            let tokens_iter = state_ref.tokens.values().map(|token| {
                (
                    token.id,
                    token.secret.clone(),
                    token.issued_at.with_nanosecond(0).unwrap().to_string(),
                    token
                        .used_at
                        .map(|time| time.with_nanosecond(0).unwrap())
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                )
            });
            rsx! {
                div {
                    id: "table-container",
                    class: "col-span-10 pt-4 w-full",
                    table {
                        class: "table-fix border border-solid text-left w-full",
                        thead {
                            tr {
                                class: "border-b border-solid p-4 font-bold bg-night-1/25",
                                th { },
                                th { class: "pl-3 w-1/4", "Id" },
                                th { class: "text-right pl-3 w-1/4", "Issued At" },
                                th { class: "text-right pl-3 w-1/4", "Used At" },
                                th { class: "w-1/4"},
                            }
                        }
                        for (id, secret, issued_at, used_at) in tokens_iter {
                            tr {
                                class: "border-b border-gray-100",
                                td {
                                    class: "px-3 cursor-pointer hover:bg-stem-1 content-center",
                                    onclick: move |_event| {
                                        let navigator = match web_sys::window() {
                                            Some(window) => window.navigator(),
                                            None => {
                                                tracing::error!("window object is not accessible");
                                                return;
                                            }
                                        };
                                        let _ = navigator.clipboard().write_text(&format!("{id}:{secret}"));
                                    },
                                    Copy{}
                                },
                                td { class: "pl-3", "{id}" }
                                td { class: "text-right pl-3", "{issued_at}" }
                                td { class: "text-right pl-3", "{used_at}" }
                                td {
                                    class: "text-right px-1",
                                    button {
                                        onclick: move |_| async move {
                                            match control_plane_client.delete_token(id).await {
                                                Ok(_) => tokens_state.write().remove_token(id),
                                                Err(e) => {
                                                    tracing::error!("failed to delete token: {e}");
                                                }
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
    };
    rsx! {
        Doc {}
        div {
            class: "container mx-auto grid grid-cols-2",
            div {
                class: "pt-5 pl-3 font-bold",
                h2 {
                    font_size: "1.5em",
                    "Tokens"
                }
            }
            div {
                class: "pt-5 justify-self-end pr-3 select-none",
                div {
                    class: "text-stem-1 px-4 py-2 rounded bg-forest-2 border border-forest-2 hover:bg-forest-3 hover:text-stem-1",
                    onclick: move |_| async move {
                        match control_plane_client.create_token().await {
                            Ok(token) => tokens_state.write().add_token(token),
                            Err(e) => tracing::error!("failed to create token: {e}"),
                        }
                    },
                    "ADD NEW TOKEN"
                }
            }
            { tokens_table }
        }
    }
}
