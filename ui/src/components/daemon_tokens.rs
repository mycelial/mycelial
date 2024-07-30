use std::collections::BTreeMap;

use crate::components::app::ControlPlaneClient;
use dioxus::prelude::*;
use uuid::Uuid;

use super::app::Token;

#[derive(Debug)]
struct TokensState {
    tokens: BTreeMap<Uuid, Token>,
}

impl TokensState {
    fn new() -> Self {
        Self { tokens: BTreeMap::new() }
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
            },
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
                    token.issued_at.to_string(),
                    token
                        .used_at
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                )
            });
            rsx! {
                div {
                    id: "table-container",
                    class: "col-span-2 pt-4 w-full",
                    table {
                        class: "table-fix border border-solid text-left w-full mx-auto",
                        thead {
                            tr {
                                class: "border-b border-solid p-4 font-bold bg-night-1/25",
                                th { class: "pl-3", "Id" },
                                th { class: "text-right pl-3", "Issued At" },
                                th { class: "text-right pl-3", "Used At" },
                                th {},
                            }
                        }
                        for (id, issued_at, used_at) in tokens_iter {
                            tr {
                                class: "border-b border-gray-100",
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
