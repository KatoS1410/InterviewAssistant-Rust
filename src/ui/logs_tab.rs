use crate::app::InterviewApp;
use crate::ui::widgets::{glass_panel, pill_button, section_heading};

pub fn show(ui: &mut egui::Ui, app: &mut InterviewApp) {
    glass_panel(ui, |ui| {
        section_heading(ui, app.t("logs.title"), "");
        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .id_salt("logs_scroll")
            .max_height(500.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut app.logs)
                        .id_salt("logs")
                        .font(egui::TextStyle::Monospace)
                        .interactive(false)
                        .desired_width(f32::INFINITY),
                );
            });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if pill_button(ui, app.t("logs.clear"), false, false).clicked() {
                app.logs.clear();
            }
            if pill_button(ui, app.t("main.copy_answer"), false, false).clicked() {
                ui.ctx().copy_text(app.logs.clone());
            }
        });
    });
}