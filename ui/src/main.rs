use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/")]
    Index {},
    #[route("/workspaces")]
    Workspaces {},
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
    email: String,
    password: String,
}

fn Login() -> Element {
    tracing::info!("hello from login");
    let location: Rc<String> = Rc::new(web_sys::window().unwrap().location().to_string().into());
    rsx! {
        div {
            class: "flex min-h-full flex-col justify-center px-6 py-12 lg:px-8",
            div {
                class: "sm:mx-auto sm:w-full sm:max-w-sm",
                h2 {
                    class: "mt-10 text-center text-2xl font-bold leading-9 tracking-tight text-gray-900",
                    "Sign in to your account"
                }
            }

            div {
                class: "mt-10 sm:mx-auto sm:w-full sm:max-w-sm",
                form {
                    onsubmit: move |event| {
                        let loc = Rc::clone(&location);
                        spawn(async move {
                            tracing::info!("got submit event: {event:?}");
                            let login = event.values().into_iter().fold(LoginForm::default(), |mut login, (key, value)| {
                                match key.as_str() {
                                    "email" => {
                                        login.email= value.as_slice().first().unwrap().into();
                                        login
                                    },
                                    "password" => {
                                        login.password = value.as_slice().first().unwrap().into();
                                        login
                                    },
                                    bad_key => panic!("bad key: {bad_key}")
                                }
                            });
                            // FIXME: check response success
                            let _res = reqwest::Client::new()
                                .post(&*loc)
                                .json(&login)
                                .send()
                                .await;
                            });
                    },
                    class: "space-y-6",
                    div {
                        label {
                            r#for: "email",
                            class: "block text-sm font-medium leading-6 text-gray-900",
                            "Email address"
                        }
                        div {
                            class:"mt-2",
                            input {
                                id: "email",
                                name: "email",
                                r#type: "email",
                                autocomplete: "email",
                                required: true,
                                class: "block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6",
                            }
                        }
                    }

                    div {
                        div {
                            class:"flex items-center justify-between",
                            label {
                                r#for:"password",
                                class:"block text-sm font-medium leading-6 text-gray-900",
                                "Password"
                            }
                            div {
                                class:"text-sm",
                                a {
                                    href:"#",
                                    class:"font-semibold text-indigo-600 hover:text-indigo-500",
                                    "Forgot password?"
                                }
                            }
                        }
                        div {
                            class:"mt-2",
                            input{
                                id:"password",
                                name:"password",
                                r#type:"password",
                                autocomplete:"current-password",
                                required: true,
                                class:"block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
                            }
                        }
                    }

                    div {
                        button {
                            r#type:"submit",
                            class:"flex w-full justify-center rounded-md bg-indigo-600 px-3 py-1.5 text-sm font-semibold leading-6 text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600",
                            "Sign in"
                        }
                    }
                    p {
                        class:"mt-10 text-center text-sm text-gray-500",
                        "Not a member?",
                        a {
                            href:"#",
                            class:"font-semibold leading-6 text-indigo-600 hover:text-indigo-500",
                            "Start a 14 day free trial"
                        }
                    }
                }
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
