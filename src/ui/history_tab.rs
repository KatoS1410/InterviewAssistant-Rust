use crate::app::InterviewApp;
use crate::ui::theme::Theme;
use crate::ui::widgets::{glass_panel, pill_button, section_heading};
use egui::RichText;

pub fn show(ui: &mut egui::Ui, app: &mut InterviewApp) {
    ui.columns(2, |cols| {
        // Левая колонка: история вопросов
        cols[0].vertical(|ui| {
            glass_panel(ui, |ui| {
                section_heading(ui, "Вопросы", "");
                ui.add_space(2.0);
                egui::ScrollArea::vertical()
                    .id_salt("history_question_scroll")
                    .max_height(ui.available_height() - 50.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        render_history_text(ui, &app.history_questions, false);
                    });
                ui.horizontal(|ui| {
                    if pill_button(ui, "Очистить", false, false).clicked() {
                        app.history_questions.clear();
                        app.history_answers.clear();
                    }
                });
            });
        });

        // Правая колонка: история ответов (ошибки красным)
        cols[1].vertical(|ui| {
            glass_panel(ui, |ui| {
                section_heading(ui, "Ответы", "");
                ui.add_space(2.0);
                egui::ScrollArea::vertical()
                    .id_salt("history_answer_scroll")
                    .max_height(ui.available_height() - 50.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        render_history_text(ui, &app.history_answers, true);
                    });
                ui.horizontal(|ui| {
                    if pill_button(ui, "Очистить", false, false).clicked() {
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