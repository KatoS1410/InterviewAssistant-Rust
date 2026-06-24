use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};

use crate::ui::theme::Theme;

/// Стеклянная панель с закруглёнными углами
pub fn glass_panel(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    Frame::none()
        .fill(Theme::GLASS)
        .stroke(Stroke::new(1.0, Theme::BORDER_STRONG))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::symmetric(12.0, 10.0))
        .show(ui, |ui| {
            ui.set_clip_rect(ui.max_rect());
            add_contents(ui);
        });
}

/// Заголовок секции
pub fn section_heading(ui: &mut Ui, title: &str, hint: &str) {
    ui.horizontal(|ui| {
        // Акцентная полоска слева
        let (rect, _) = ui.allocate_exact_size(Vec2::new(3.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, Rounding::same(1.5), Theme::ACCENT);
        ui.add_space(6.0);
        ui.label(RichText::new(title).size(14.0).strong().color(Theme::TEXT));
        if !hint.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(hint).size(10.0).color(Theme::TEXT_FAINT));
        }
    });
    ui.add_space(4.0);
}

/// Кнопка-пилюля
pub fn pill_button(ui: &mut Ui, label: &str, accent: bool, danger: bool) -> egui::Response {
    let (fill, text_color, stroke_color) = if danger {
        (Theme::DANGER_DIM, Color32::WHITE, Theme::DANGER)
    } else if accent {
        (Theme::ACCENT_SOFT, Color32::WHITE, Theme::ACCENT)
    } else {
        (Theme::PANEL_DARK, Theme::TEXT, Theme::BORDER_STRONG)
    };

    ui.add(
        egui::Button::new(RichText::new(label).color(text_color).size(12.0))
            .fill(fill)
            .stroke(Stroke::new(1.2, stroke_color))
            .rounding(Rounding::same(20.0))
            .min_size(Vec2::new(40.0, 28.0)),
    )
}

/// ComboBox с меткой
pub fn labeled_combo(ui: &mut Ui, label: &str, value: &mut String, options: &[String]) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(Theme::TEXT_DIM));
        egui::ComboBox::from_id_salt(label)
            .selected_text(value.as_str())
            .width(ui.available_width() - 20.0)
            .show_ui(ui, |ui| {
                for opt in options {
                    ui.selectable_value(value, opt.clone(), opt);
                }
            });
    });
}

/// Многострочное текстовое поле
pub fn copyable_text_edit(ui: &mut Ui, id: &str, text: &mut String, rows: usize) {
    let mut content = text.clone();
    let response = ui.add(
        egui::TextEdit::multiline(&mut content)
            .id_salt(id)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .desired_rows(rows)
            .interactive(true),
    );
    if response.changed() {
        *text = content;
    }
}

/// Статус-бар
pub fn status_bar(ui: &mut Ui, status: &str, ai_info: &str, vosk_info: &str) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new(status).size(11.0).color(Theme::TEXT_DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(vosk_info).size(11.0).color(Theme::TEXT_FAINT));
            ui.separator();
            ui.label(RichText::new(ai_info).size(11.0).color(Theme::TEXT_FAINT));
        });
    });
}
