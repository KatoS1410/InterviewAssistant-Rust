use egui::{Color32, FontId, Rounding, Stroke, Style, Visuals};

pub struct Theme;

impl Theme {
    // Фон
    pub const BG: Color32 = Color32::from_rgb(14, 16, 20);
    // Панели
    pub const PANEL: Color32 = Color32::from_rgb(22, 25, 31);
    pub const PANEL_DARK: Color32 = Color32::from_rgb(28, 32, 40);
    // Стеклянная панель
    pub const GLASS: Color32 = Color32::from_rgba_premultiplied(34, 40, 52, 200);
    // Неактивная вкладка
    pub const TAB_INACTIVE: Color32 = Color32::from_rgb(44, 50, 62);
    // Акцент
    pub const ACCENT: Color32 = Color32::from_rgb(0, 122, 255);
    pub const ACCENT_SOFT: Color32 = Color32::from_rgb(10, 96, 210);
    pub const ACCENT_GLOW: Color32 = Color32::from_rgba_premultiplied(0, 122, 255, 60);
    // Опасность/ошибка
    pub const DANGER: Color32 = Color32::from_rgb(255, 69, 58);
    pub const DANGER_DIM: Color32 = Color32::from_rgb(180, 50, 45);
    // Текст
    pub const TEXT: Color32 = Color32::from_rgb(235, 238, 245);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(160, 168, 182);
    pub const TEXT_FAINT: Color32 = Color32::from_rgb(100, 108, 122);
    // Границы
    pub const BORDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 20);
    pub const BORDER_STRONG: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 40);
    // Поле ввода
    pub const INPUT_BG: Color32 = Color32::from_rgb(18, 21, 27);
    // Индикатор записи
    pub const REC: Color32 = Color32::from_rgb(255, 59, 48);
}

pub fn apply_theme(ctx: &egui::Context) {
    // Шрифты с поддержкой кириллицы
    let mut fonts = egui::FontDefinitions::default();
    // Системные шрифты
    let system_font_path = if cfg!(windows) {
        "C:\\Windows\\Fonts\\segoeui.ttf"
    } else if cfg!(target_os = "macos") {
        "/System/Library/Fonts/Helvetica.ttc"
    } else {
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"
    };
    if let Ok(bytes) = std::fs::read(system_font_path) {
        fonts.font_data.insert(
            "SystemFont".to_owned(),
            egui::FontData::from_owned(bytes),
        );
        // Пропорциональный шрифт
        fonts.families.insert(
            egui::FontFamily::Proportional,
            vec!["SystemFont".to_owned(), "Hack".to_owned()],
        );
        // Моноширинный шрифт
        fonts.families.insert(
            egui::FontFamily::Monospace,
            vec!["Hack".to_owned(), "SystemFont".to_owned()],
        );
    }
    ctx.set_fonts(fonts);

    let mut style = Style::default();

    // Отступы
    style.spacing.item_spacing = egui::vec2(5.0, 5.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(4.0);
    style.spacing.text_edit_width = 120.0;
    style.spacing.indent = 16.0;
    style.spacing.icon_width = 20.0;
    style.spacing.icon_spacing = 6.0;

    // Размеры шрифтов
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(19.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(11.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(13.0, egui::FontFamily::Monospace),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(14.0, egui::FontFamily::Proportional),
    );

    let mut visuals = Visuals::dark();
    visuals.dark_mode = true;
    visuals.window_fill = Theme::BG;
    visuals.panel_fill = Theme::BG;
    visuals.extreme_bg_color = Theme::INPUT_BG;
    visuals.faint_bg_color = Theme::PANEL;
    visuals.window_stroke = Stroke::new(1.0, Theme::BORDER);
    visuals.window_rounding = Rounding::same(14.0);

    // Стиль виджетов
    visuals.widgets.noninteractive.bg_fill = Theme::PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Theme::TEXT_DIM);
    visuals.widgets.noninteractive.rounding = Rounding::same(10.0);

    visuals.widgets.inactive.bg_fill = Theme::PANEL_DARK;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Theme::TEXT);
    visuals.widgets.inactive.rounding = Rounding::same(10.0);

    // Наведение курсора
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(20, 28, 22);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::from_rgb(0, 180, 80));
    visuals.widgets.hovered.rounding = Rounding::same(10.0);
    visuals.widgets.hovered.expansion = 1.0;

    // Нажатие
    visuals.widgets.active.bg_fill = Color32::from_rgb(14, 22, 16);
    visuals.widgets.active.fg_stroke = Stroke::new(1.8, Color32::from_rgb(0, 220, 100));
    visuals.widgets.active.rounding = Rounding::same(10.0);

    // Выделение текста
    visuals.selection.bg_fill = Theme::ACCENT_SOFT;
    visuals.selection.stroke = Stroke::new(1.0, Theme::ACCENT);

    // Ссылки
    visuals.hyperlink_color = Theme::ACCENT;

    // Текст по умолчанию
    visuals.override_text_color = Some(Theme::TEXT);

    // Тени
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 32.0,
        spread: 0.0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 120),
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 24.0,
        spread: 0.0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 140),
    };

    // Полосы прокрутки
    visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;

    style.visuals = visuals;
    ctx.set_style(style);
}

/// Заголовок с индикатором записи
pub fn draw_header(ui: &mut egui::Ui, title: &str, subtitle: &str, recording: bool) {
    let width = ui.available_width();
    let height = 60.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter();

    // Градиентный фон заголовка
    let top_color = Color32::from_rgb(18, 21, 28);
    let bottom_color = Color32::from_rgb(26, 30, 38);
    let mid = rect.top() + height * 0.5;
    painter.rect_filled(
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, mid)),
        Rounding {
            nw: 12.0,
            ne: 12.0,
            sw: 0.0,
            se: 0.0,
        },
        top_color,
    );
    painter.rect_filled(
        egui::Rect::from_min_max(egui::pos2(rect.min.x, mid), rect.max),
        Rounding {
            nw: 0.0,
            ne: 0.0,
            sw: 0.0,
            se: 0.0,
        },
        bottom_color,
    );

    // Линия внизу
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, Theme::BORDER),
    );

    // Заголовок
    painter.text(
        rect.left_top() + egui::vec2(20.0, 10.0),
        egui::Align2::LEFT_TOP,
        title,
        FontId::proportional(20.0),
        Theme::TEXT,
    );
    // Подзаголовок
    painter.text(
        rect.left_top() + egui::vec2(20.0, 36.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        FontId::proportional(11.0),
        Theme::TEXT_DIM,
    );

    // Индикатор записи
    let lamp_center = rect.right_center() + egui::vec2(-30.0, 0.0);
    let lamp_color = if recording {
        Theme::REC
    } else {
        Theme::TEXT_FAINT
    };
    // Свечение
    if recording {
        painter.circle_filled(lamp_center, 9.0, Theme::ACCENT_GLOW);
    }
    painter.circle_filled(lamp_center, 5.0, lamp_color);
    // Белая точка в центре
    painter.circle_filled(lamp_center, 2.0, Color32::from_rgba_premultiplied(255, 255, 255, 180));
}
