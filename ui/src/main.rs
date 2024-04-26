use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/")]
    Index {},
    #[route("/workspaces")]
    Workspaces{},
    #[route("/login")]
    Login {},
}


fn Index() -> Element {
    tracing::info!("hello from index");
    rsx! {
        "index"
    }
}


#[derive(Debug, Default, Serialize, Deserialize)]
struct LoginForm {
    login: String,
    password: String, 
}

fn Login() -> Element {
    tracing::info!("hello from login");
    let location: Rc<String> = Rc::new(web_sys::window().unwrap().location().to_string().into());
    rsx!{
        form {
            onsubmit: move |event| {
                let loc = Rc::clone(&location);
                spawn(async move {
                    tracing::info!("got submit event: {event:?}");
                    let login = event.values().into_iter().fold(LoginForm::default(), |mut login, (key, value)| {
                        match key.as_str() {
                            "login" => {
                                login.login = value.as_slice().first().unwrap().into();
                                login
                            },
                            "password" => {
                                login.password = value.as_slice().first().unwrap().into();
                                login
                            },
                            bad_key => panic!("bad key: {bad_key}")
                        }
                    });
                    let res = reqwest::Client::new()
                        .post(&*loc)
                        .json(&login)
                        .send()
                        .await;
                });
            },
            div {
                "login"
                input {
                    name: "login"
                }
            }
            div {
                "password"
                input {
                    name: "password",
                    r#type: "password"
                }
            }
            button {
                "login"
            }
        }
    }
}

fn Workspaces() -> Element {
    tracing::info!("hello from workspace");
    rsx! {
        "workspaces"
    }
}

fn App() -> Element {
    rsx! {
        Router::<Route> { }
    }
}

fn main() {
    tracing_wasm::set_as_global_default();
    dioxus::launch(App)
}