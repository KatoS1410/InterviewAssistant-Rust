use crate::app::InterviewApp;
use crate::ui::widgets::{glass_panel, pill_button, section_heading};

pub fn show(ui: &mut egui::Ui, app: &mut InterviewApp) {
    // Большой статус (распознавание / нет ответа AI).
    if !app.big_status.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(&app.big_status)
                    .size(20.0)
                    .strong()
                    .color(crate::ui::theme::Theme::ACCENT),
            );
        });
        ui.add_space(6.0);
    }

    // Компактная панель управления записью сверху.
    ui.set_max_width(ui.available_width());
    glass_panel(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(app.t("main.recording"))
                    .strong()
                    .color(crate::ui::theme::Theme::TEXT),
            );
            ui.separator();

            let loopback_rec = app.recording && app.record_mode == Some(crate::services::audio::AudioMode::Loopback);
            if pill_button(ui, "Loopback", false, loopback_rec).clicked() {
                app.detect_loopback();
            }

            let mic_rec = app.recording && app.record_mode == Some(crate::services::audio::AudioMode::Mic);
            if pill_button(ui, "Mic", false, mic_rec).clicked() {
                app.detect_mic();
            }

            if pill_button(ui, app.t("settings.save"), false, false).clicked() {
                app.save_config();
            }

            ui.separator();
            ui.label(
                egui::RichText::new(&app.status)
                    .size(11.0)
                    .color(crate::ui::theme::Theme::TEXT_DIM),
            );
        });

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Loopback:")
                    .size(11.0)
                    .color(crate::ui::theme::Theme::TEXT_DIM),
            );
            egui::ComboBox::from_id_salt("loopback_dev_main")
                .selected_text(if app.cfg.loopback_device.is_empty() {
                    "—"
                } else {
                    &app.cfg.loopback_device
                })
                .width(ui.available_width() / 2.0 - 40.0)
                .show_ui(ui, |ui| {
                    for name in &app.device_names {
                        ui.selectable_value(&mut app.cfg.loopback_device, name.clone(), name);
                    }
                });
            ui.label(
                egui::RichText::new("Mic:")
                    .size(11.0)
                    .color(crate::ui::theme::Theme::TEXT_DIM),
            );
            egui::ComboBox::from_id_salt("mic_dev_main")
                .selected_text(if app.cfg.mic_device.is_empty() {
                    "—"
                } else {
                    &app.cfg.mic_device
                })
                .width(ui.available_width() - 20.0)
                .show_ui(ui, |ui| {
                    for name in &app.device_names {
                        ui.selectable_value(&mut app.cfg.mic_device, name.clone(), name);
                    }
                });
        });
    });

    ui.add_space(6.0);

    // Два окна рядом: слева — живая речь/вопрос, справа — ответ AI.
    // Используем allocate_ui с точными размерами, чтобы блоки не вылезали.
    let available = ui.available_size();
    let col_w = available.x / 2.0 - 6.0;
    let col_h = available.y;
    ui.columns(2, |cols| {
        // Левое окно: распознанная речь / вопрос.
        cols[0].allocate_ui(egui::vec2(col_w, col_h), |ui| {
            glass_panel(ui, |ui| {
                section_heading(ui, app.t("main.transcript"), &app.transcript_hint);
                egui::ScrollArea::vertical()
                    .id_salt("transcript_scroll")
                    .max_height(ui.available_height() - 44.0)
                    .auto_shrink([false; 2])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut app.transcript)
                                .id_salt("transcript")
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, app.t("main.ask"), true, false).clicked() {
                        app.question = app.transcript.clone();
                        app.ask_ai();
                    }
                    if pill_button(ui, app.t("main.clear"), false, false).clicked() {
                        app.transcript.clear();
                        app.question.clear();
                    }
                });
            });
        });

        // Правое окно: ответ AI.
        cols[1].allocate_ui(egui::vec2(col_w, col_h), |ui| {
            glass_panel(ui, |ui| {
                section_heading(ui, app.t("main.answer"), "");
                egui::ScrollArea::vertical()
                    .id_salt("answer_scroll")
                    .max_height(ui.available_height() - 44.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut app.answer)
                                .id_salt("answer")
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, app.t("main.prev_answer"), false, false).clicked() {
                        if !app.prev_answer.is_empty() && !app.prev_answer.starts_with("Ошибка AI:") {
                            app.answer = app.prev_answer.clone();
                            app.transcript = app.prev_question.clone();
                            app.question = app.prev_question.clone();
                            app.set_status("Восстановлен предыдущий вопрос и ответ");
                        } else if app.prev_answer.starts_with("Ошибка AI:") {
                            app.set_status("Предыдущий ответ был ошибочным, не восстанавливаем");
                        } else {
                            app.set_status("Нет предыдущего ответа");
                        }
                    }
                    if pill_button(ui, app.t("main.clear"), false, false).clicked() {
                        app.answer.clear();
                    }
                });

                if !app.last_error.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(&app.last_error)
                            .size(11.0)
                            .color(crate::ui::theme::Theme::DANGER),
                    );
                }
            });
        });
    });
}
