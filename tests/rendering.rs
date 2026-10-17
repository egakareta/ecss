mod rendering {
    use ecss::*;
    use eframe::egui;
    use std::sync::Mutex;
    use support::*;

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    automod::dir!("tests/rendering");
}
