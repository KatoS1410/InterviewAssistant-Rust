use crate::app::InterviewApp;
use crate::ui::widgets::{glass_panel, pill_button, section_heading};

pub fn show(ui: &mut egui::Ui, app: &mut InterviewApp) {
    glass_panel(ui, |ui| {
        section_heading(ui, "Логи / Диагностика", "");
        ui.add_space(2.0);
        egui::ScrollArea::vertical()
            .id_salt("logs_scroll")
            .max_height(ui.available_height() - 50.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut app.logs)
                        .id_salt("logs")
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .interactive(true),
                );
            });
        ui.horizontal(|ui| {
            if pill_button(ui, "Очистить", false, false).clicked() {
                app.logs.clear();
            }
            if pill_button(ui, "Копировать", false, false).clicked() {
                ui.ctx().copy_text(app.logs.clone());
            }
        });
    });
}