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
            id: "daemon-install-instructions",
            p {
                    "To add your local daemon to this Mycelial network, simply install the
                    daemon using the instructions found "
                    a {
                    class: "underline",
                    href: "https://docs.mycelial.com/getting-started/CLI/",
                    target: "_blank",
                    "here"
                }
                " and copied below:"
            }
            div {
                //TODO: make endpoint and token dynamic
                id: "daemon-install-code-mac",
                class: "py-2 my-4 bg-white text-night-2 shadow-none",
                h3 {
                    class: "text-lg ml-2 py-2 uppercase",
                    "Mac Installation Instructions"
                }
                div {
                    class: "bg-grey-bright mx-2 py-2 rounded",
                    p {
                        class: "ml-2 font-mono",
                        "$ brew install mycelial/tap/mycelial"
                    }
                    p {
                        class: "ml-2 font-mono",
                        "$ mycelial init --daemon --endpoint \"http://localhost:7777\" --token \"d135801c-fd73-477c-b0a8-055d0d117485\""
                    }
                    p {
                        class: "ml-2 font-mono",
                        "$ mycelial start --daemon"
                    }
                }

            }
            div {
                //TODO: make endpoint and token dynamic
                id: "daemon-install-code-linux",
                class: "py-2 my-2 bg-white text-night-2 shadow-none",
                h3 {
                    class: "text-lg ml-2 py-2 uppercase",
                    "Linux Installation Instructions"
                }
                div {
                    class: "bg-grey-bright mx-2 py-2 rounded",
                    p {
                        class: "mb-2 ml-2",
                        "Mycelial maintains CLI builds for Debian and Fedora across several x86 and ARM chip architectures."
                    }
                    p {
                        class: "ml-2",
                        "Please visit the Mycelial CLI documentation page to find the installation instructions for your system "
                        a {
                            href: "https://docs.mycelial.com/getting-started/CLI",
                            target: "_blank",
                            class: "underline",
                            "here"
                        }
                        "."
                    }
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
                                        match navigator.clipboard() {
                                            Some(clipboard) => {
                                                let _ = clipboard.write_text(&format!("{id}:{secret}"));
                                            },
                                            None => {
                                                tracing::error!("clipboard is not available");
                                            }
                                        }
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
