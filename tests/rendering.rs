#[path = "common/mod.rs"]
mod common;

mod rendering {
    use super::common::*;
    use ecss::*;
    use eframe::egui;
    use std::sync::Mutex;
    mod support;
    use support::*;

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    mod backgrounds;
    mod box_model;
    mod columns;
    mod layout;
    mod media;
    mod outline;
    mod selectors;
    mod tables;
    mod text;
    mod text_decoration;
    mod transitions;
}
