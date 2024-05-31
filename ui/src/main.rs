fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::DEBUG).expect("logger failed to init");
    dioxus::launch(ui::components::app::App)
}
