//! Спільна палітра й формати для xlsx-документів у стилі 3 ОШБр / ЗСУ.
//! Кольори взяті з еталонного файлу замовника (таблиця з `source_files`).

use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder};

pub const FONT_NAME: &str = "Calibri";

pub const BG_DARK_OLIVE: Color = Color::RGB(0x453F2E);
pub const BG_WARM_LIGHT: Color = Color::RGB(0xF5F2EC);
pub const BG_AMBER: Color = Color::RGB(0xF3E3AE);
pub const BG_BEIGE: Color = Color::RGB(0xE7DDBA);
pub const BG_GREEN: Color = Color::RGB(0xDCE8D1);
pub const BG_SALMON: Color = Color::RGB(0xE9C4B8);
pub const BG_ORANGE: Color = Color::RGB(0xF39200);

pub const TEXT_DARK: Color = Color::RGB(0x2A2620);
pub const TEXT_WHITE: Color = Color::White;
pub const TEXT_MUTED: Color = Color::RGB(0x7A6F55);
pub const TEXT_ORANGE_DARK: Color = Color::RGB(0x855000);
pub const TEXT_RED: Color = Color::RGB(0xA13A2A);

pub const BORDER_THIN: Color = Color::RGB(0xC7C1B0);
pub const BORDER_HAIR: Color = Color::RGB(0xE3DCC9);

fn base() -> Format {
    Format::new()
        .set_font_name(FONT_NAME)
        .set_font_size(10)
        .set_font_color(TEXT_DARK)
}

fn with_borders(f: Format) -> Format {
    f.set_border(FormatBorder::Thin)
        .set_border_color(BORDER_THIN)
}

pub fn title_format() -> Format {
    base()
        .set_font_size(14)
        .set_bold()
        .set_font_color(TEXT_DARK)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_background_color(BG_BEIGE)
}

pub fn header_format() -> Format {
    with_borders(
        base()
            .set_font_size(10)
            .set_bold()
            .set_font_color(TEXT_WHITE)
            .set_background_color(BG_DARK_OLIVE)
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_text_wrap(),
    )
}

pub fn subheader_format() -> Format {
    with_borders(
        base()
            .set_font_size(11)
            .set_bold()
            .set_font_color(TEXT_DARK)
            .set_background_color(BG_BEIGE)
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_text_wrap(),
    )
}

pub fn total_format() -> Format {
    with_borders(
        base()
            .set_font_size(11)
            .set_bold()
            .set_font_color(TEXT_WHITE)
            .set_background_color(BG_ORANGE)
    )
}

pub fn total_number_format() -> Format {
    with_borders(
        base()
            .set_font_size(11)
            .set_bold()
            .set_font_color(TEXT_WHITE)
            .set_background_color(BG_ORANGE)
            .set_align(FormatAlign::Center)
    )
}

pub fn data_format() -> Format {
    with_borders(
        base()
            .set_font_size(10)
            .set_font_color(TEXT_DARK)
            .set_background_color(BG_WARM_LIGHT)
    )
}

pub fn data_number_format() -> Format {
    with_borders(
        base()
            .set_font_size(10)
            .set_font_color(TEXT_DARK)
            .set_background_color(BG_WARM_LIGHT)
            .set_align(FormatAlign::Center)
    )
}

pub fn data_alt_format() -> Format {
    with_borders(
        base()
            .set_font_size(10)
            .set_font_color(TEXT_DARK)
            .set_background_color(Color::White)
    )
}

pub fn data_alt_number_format() -> Format {
    with_borders(
        base()
            .set_font_size(10)
            .set_font_color(TEXT_DARK)
            .set_background_color(Color::White)
            .set_align(FormatAlign::Center)
    )
}

pub fn section_format() -> Format {
    with_borders(
        base()
            .set_font_size(11)
            .set_bold()
            .set_font_color(TEXT_DARK)
            .set_background_color(BG_AMBER)
            .set_align(FormatAlign::Center)
            .set_text_wrap(),
    )
}

pub fn muted_format() -> Format {
    with_borders(
        base()
            .set_font_size(9)
            .set_font_color(TEXT_MUTED)
            .set_background_color(BG_WARM_LIGHT)
    )
}

pub fn accent_format() -> Format {
    with_borders(
        base()
            .set_font_size(10)
            .set_bold()
            .set_font_color(TEXT_ORANGE_DARK)
            .set_background_color(BG_WARM_LIGHT)
    )
}

pub fn green_number_format() -> Format {
    with_borders(
        base()
            .set_font_size(10)
            .set_font_color(TEXT_DARK)
            .set_background_color(BG_GREEN)
            .set_align(FormatAlign::Center)
    )
}

pub fn red_number_format() -> Format {
    with_borders(
        base()
            .set_font_size(10)
            .set_font_color(TEXT_RED)
            .set_background_color(BG_SALMON)
            .set_align(FormatAlign::Center)
    )
}
