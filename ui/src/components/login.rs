use crate::components::logo::LogoDark;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, Default, Serialize, Deserialize)]
struct LoginForm {
    email: String,
    password: String,
}

struct LoginGuard(Signal<Option<Task>>);

impl Drop for LoginGuard {
    fn drop(&mut self) {
        if self.0.read().is_some() {
            *self.0.write() = None;
        }
    }
}

pub fn Login() -> Element {
    let location: String = web_sys::window().unwrap().location().to_string().into();
    let location: Rc<str> = Rc::from(location);
    let mut logging_in = use_signal(|| None);

    rsx! {
        div {
            class: "container mx-auto min-h-full lg:px-8 mt-10 ",
            div {
                class: "bg-moss-1 p-6 sm:mx-auto sm:w-full sm:max-w-sm shadow-md rounded-md text-grey-bright",
                div {
                    class: "flex justify-center",
                    LogoDark{},
                }
                div {
                    class: "sm:mx-auto sm:w-full sm:max-w-sm",
                    h2 {
                        class: "mt-10 text-center text-2xl font-bold leading-9 tracking-tight text-night-1",
                        "Sign in to your account"
                    }
                }
            div {
                class: "mt-10 sm:mx-auto sm:w-full sm:max-w-sm",
                form {
                    onsubmit: move |event| {
                        if logging_in.read().is_some() {
                            // already logging in
                            return
                        }
                        let loc = Rc::clone(&location);
                        let task = spawn(async move {
                            LoginGuard(logging_in);
                            let login = event.values().into_iter().fold(LoginForm::default(), |mut login, (key, value)| {
                                match key.as_str() {
                                    "email" => {
                                        login.email= value.as_value();
                                        login
                                    },
                                    "password" => {
                                        login.password = value.as_value();
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
                        *logging_in.write() = Some(task);
                    },
                    class: "space-y-6",
                    div {
                        label {
                            r#for: "email",
                            class: "block text-sm font-medium leading-6 text-night-1",
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
                                class:"block text-sm font-medium leading-6 text-night-1",
                                "Password"
                            }
                            div {
                                class:"text-sm",
                                a {
                                    href:"#",
                                    class:"font-semibold text-grey-bright hover:underline",
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
                }

                    div {
                        button {
                            r#type:"submit",
                            class:"flex w-full justify-center rounded-md bg-forest-2 px-3 py-1.5 text-sm font-semibold leading-6 text-white shadow-sm mt-5",
                            "Sign in"
                        }
                    }
                    p {
                        class:"mt-10 text-center text-sm text-grey-bright",
                        "Need an account? ",
                        a {
                            href:"#",
                            class:"font-semibold leading-6 hover:underline",
                            "Sign up now."
                        }
                    }
                }
            }
        }
    }
}
