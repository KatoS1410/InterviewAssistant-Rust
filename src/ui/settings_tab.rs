use crate::app::InterviewApp;
use crate::config::PROVIDERS;
use crate::ui::theme::Theme;
use crate::ui::widgets::{
    copyable_text_edit, glass_panel, labeled_combo, pill_button, section_heading,
};
use egui::RichText;

pub fn show(ui: &mut egui::Ui, app: &mut InterviewApp) {
    // Используем columns с min spacing для выравнивания.
    ui.spacing_mut().item_spacing.x = 12.0;
    ui.columns(2, |cols| {
        // === Колонка 0 ===
        cols[0].vertical(|ui| {
            glass_panel(ui, |ui| {
                section_heading(ui, "AI / Provider", "");
                let providers: Vec<String> = PROVIDERS.iter().map(|(p, _, _)| (*p).into()).collect();
                let prev_provider = app.cfg.provider.clone();
                labeled_combo(ui, "Provider", &mut app.cfg.provider, &providers);
                if app.cfg.provider != prev_provider {
                    app.cfg.apply_provider_preset(&app.cfg.provider.clone());
                }

                ui.end_row();
                match app.cfg.provider.to_lowercase().as_str() {
                    "gigachat" => {
                        ui.label(RichText::new("Authorization Key").color(Theme::TEXT_DIM));
                        ui.add(
                            egui::TextEdit::singleline(&mut app.cfg.gigachat_auth_key)
                                .password(true)
                                .desired_width(ui.available_width()),
                        );
                        ui.end_row();

                        ui.label(RichText::new("Scope").color(Theme::TEXT_DIM));
                        ui.text_edit_singleline(&mut app.cfg.gigachat_scope);
                        ui.end_row();
                    }
                    "local llms" => {
                        ui.label(RichText::new("Address").color(Theme::TEXT_DIM));
                        ui.text_edit_singleline(&mut app.cfg.local_llm_address);
                        ui.end_row();

                        ui.label(RichText::new("Port").color(Theme::TEXT_DIM));
                        ui.add(egui::DragValue::new(&mut app.cfg.local_llm_port).speed(1.0));
                        ui.end_row();
                    }
                    _ => {
                        ui.label(RichText::new("API Key").color(Theme::TEXT_DIM));
                        ui.add(
                            egui::TextEdit::singleline(&mut app.cfg.api_key)
                                .password(true)
                                .desired_width(ui.available_width()),
                        );
                        ui.end_row();

                        ui.label(RichText::new("Base URL").color(Theme::TEXT_DIM));
                        ui.text_edit_singleline(&mut app.cfg.base_url);
                        ui.end_row();
                    }
                }

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Model").color(Theme::TEXT_DIM));
                    ui.text_edit_singleline(&mut app.cfg.model);
                    if pill_button(ui, "Проверить", true, false).clicked() {
                        app.test_ai();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Должность").color(Theme::TEXT_DIM));
                    ui.text_edit_singleline(&mut app.cfg.position);
                });
            });

            ui.add_space(6.0);

            glass_panel(ui, |ui| {
                section_heading(ui, "System Prompt", "");
                copyable_text_edit(ui, "prompt", &mut app.cfg.system_prompt, 12);
            });
        });

        // === Колонка 1 ===
        cols[1].vertical(|ui| {
            glass_panel(ui, |ui| {
                section_heading(ui, "VOSK / Аудио", "");
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Model path").color(Theme::TEXT_DIM));
                    ui.text_edit_singleline(&mut app.cfg.vosk_model_path);
                    if pill_button(ui, "...", false, false).clicked() {
                        app.browse_vosk_model();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Chunk ms").color(Theme::TEXT_DIM));
                    ui.text_edit_singleline(&mut app.chunk_ms_text);
                    ui.label(RichText::new("Auto ask sec").color(Theme::TEXT_DIM));
                    ui.text_edit_singleline(&mut app.auto_ask_text);
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Tail ms").color(Theme::TEXT_DIM));
                    ui.text_edit_singleline(&mut app.tail_ms_text);
                    ui.label(
                        RichText::new("Задержка после остановки для сбора хвоста loopback")
                            .size(10.0)
                            .color(Theme::TEXT_FAINT),
                    );
                });

                ui.horizontal(|ui| {
                    if pill_button(ui, "Загрузить VOSK", true, false).clicked() {
                        app.load_vosk();
                    }
                    if pill_button(ui, "Перезагрузить", false, false).clicked() {
                        app.reload_vosk();
                    }
                });

                ui.add_space(4.0);

                ui.label(
                    RichText::new("Скачать VOSK модель можно здесь:")
                        .size(12.0)
                        .color(Theme::TEXT_DIM),
                );
                ui.hyperlink_to(
                    "https://alphacephei.com/vosk/models",
                    "https://alphacephei.com/vosk/models",
                );
                ui.label(
                    RichText::new("Распакуйте архив и укажите путь к папке с моделью.")
                        .size(10.0)
                        .color(Theme::TEXT_FAINT),
                );
                ui.label(
                    RichText::new("DLL скачается автоматически при загрузке модели.")
                        .size(10.0)
                        .color(Theme::TEXT_FAINT),
                );

                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("VOSK: {}", app.vosk_status))
                        .size(11.0)
                        .color(Theme::TEXT_DIM),
                );
            });

            ui.add_space(6.0);

            // Конфиг скрыт за чекбоксом. Раскрывается вниз.
            glass_panel(ui, |ui| {
                ui.horizontal(|ui| {
                    let label = if app.config_edit_mode {
                        "Скрыть конфиг"
                    } else {
                        "Показать конфиг для редактирования"
                    };
                    if pill_button(ui, label, !app.config_edit_mode, app.config_edit_mode).clicked() {
                        app.config_edit_mode = !app.config_edit_mode;
                    }
                });
                if app.config_edit_mode {
                    ui.add_space(4.0);
                    section_heading(ui, "Конфиг", "");
                    ui.horizontal(|ui| {
                        if pill_button(ui, "Сохранить", true, false).clicked() {
                            app.save_config();
                        }
                        if pill_button(ui, "Экспорт", false, false).clicked() {
                            app.export_config();
                        }
                        if pill_button(ui, "Импорт", false, false).clicked() {
                            app.import_config();
                        }
                    });
                    copyable_text_edit(ui, "cfg_preview", &mut app.config_preview, 10);
                }
            });
        });
    });
}