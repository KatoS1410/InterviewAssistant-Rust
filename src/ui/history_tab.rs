use crate::app::InterviewApp;
use crate::ui::theme::Theme;
use crate::ui::widgets::{glass_panel, pill_button, section_heading};
use egui::RichText;

pub fn show(ui: &mut egui::Ui, app: &mut InterviewApp) {
    let available = ui.available_size();
    let col_w = available.x / 2.0 - 12.0;
    let col_h = available.y - 8.0;
    ui.columns(2, |cols| {
        // Левая колонка: история вопросов
        cols[0].allocate_ui(egui::vec2(col_w, col_h), |ui| {
            ui.set_clip_rect(ui.max_rect());
            glass_panel(ui, |ui| {
                section_heading(ui, app.t("history.questions"), "");
                egui::ScrollArea::vertical()
                    .id_salt("history_question_scroll")
                    .max_height(ui.available_height() - 44.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        render_history_text(ui, &app.history_questions, false);
                    });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, app.t("history.clear"), false, false).clicked() {
                        app.history_questions.clear();
                        app.history_answers.clear();
                    }
                });
            });
        });

        // Правая колонка: история ответов (ошибки красным)
        cols[1].allocate_ui(egui::vec2(col_w, col_h), |ui| {
            ui.set_clip_rect(ui.max_rect());
            glass_panel(ui, |ui| {
                section_heading(ui, app.t("history.answers"), "");
                egui::ScrollArea::vertical()
                    .id_salt("history_answer_scroll")
                    .max_height(ui.available_height() - 44.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        render_history_text(ui, &app.history_answers, true);
                    });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, app.t("history.clear"), false, false).clicked() {
                        app.history_questions.clear();
                        app.history_answers.clear();
                    }
                });
            });
        });
    });
}

/// Рендерит текст истории: разбивает на параграфы по двойному переносу строки.
/// Если `highlight_errors` — строки с префиксом [ОШИБКА] красятся красным.
fn render_history_text(ui: &mut egui::Ui, text: &str, highlight_errors: bool) {
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    for para in paragraphs {
        if para.is_empty() {
            continue;
        }
        if highlight_errors && para.starts_with("[ОШИБКА]") {
            ui.label(
                RichText::new(para)
                    .monospace()
                    .color(Theme::DANGER),
            );
        } else {
            ui.label(
                RichText::new(para)
                    .monospace()
                    .color(Theme::TEXT),
            );
        }
        ui.add_space(4.0);
    }
}
