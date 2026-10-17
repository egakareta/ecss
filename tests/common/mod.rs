pub(crate) fn empty_response(ui: &mut egui::Ui) -> egui::Response {
    ui.allocate_exact_size(egui::Vec2::ZERO, egui::Sense::hover())
        .1
}
