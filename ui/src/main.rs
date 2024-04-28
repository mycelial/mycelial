fn main() {
    tracing_wasm::set_as_global_default();
    dioxus::launch(ui::components::app::App)
}