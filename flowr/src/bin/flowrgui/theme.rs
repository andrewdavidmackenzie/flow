//! Custom styling for flowrgui widgets.

use iced::widget::button;
use iced::{Background, Border, Color, Shadow, Theme, Vector};

/// Accent color used for active/hover states
const ACCENT: Color = Color {
    r: 0.424,
    g: 0.361,
    b: 0.906,
    a: 1.0,
};

/// Border radius for buttons and tabs
const RADIUS: f32 = 8.0;

/// Border width for hover highlight
const BORDER_WIDTH: f32 = 2.0;

/// Custom button style with rounded corners and hover border highlight.
pub fn styled_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let base = button::Style {
        border: Border {
            radius: RADIUS.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow {
            color: Color::BLACK,
            offset: Vector::new(0.0, 1.0),
            blur_radius: 3.0,
        },
        snap: true,
        ..button::Style::default()
    };

    match status {
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color {
                r: 0.25,
                g: 0.25,
                b: 0.35,
                a: 1.0,
            })),
            text_color: palette.text,
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(ACCENT)),
            text_color: Color::WHITE,
            border: Border {
                radius: RADIUS.into(),
                width: BORDER_WIDTH,
                color: lighten(ACCENT, 0.3),
            },
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(darken(ACCENT, 0.2))),
            text_color: Color::WHITE,
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color {
                a: 0.3,
                ..palette.primary
            })),
            text_color: Color {
                a: 0.5,
                ..palette.text
            },
            ..base
        },
    }
}

/// Transparent button style for list items — no background, hover highlight only.
pub fn list_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    match status {
        button::Status::Active => button::Style {
            background: None,
            text_color: palette.text,
            ..button::Style::default()
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color {
                a: 0.1,
                ..palette.text
            })),
            text_color: palette.text,
            ..button::Style::default()
        },
        _ => styled_button(theme, status),
    }
}

/// Debug event colors
#[cfg(feature = "debugger")]
pub mod debug_colors {
    use iced::Color;

    pub const ERROR: Color = Color {
        r: 0.9,
        g: 0.3,
        b: 0.3,
        a: 1.0,
    };
    pub const BREAKPOINT: Color = Color {
        r: 0.9,
        g: 0.7,
        b: 0.2,
        a: 1.0,
    };
    pub const COMPLETION: Color = Color {
        r: 0.4,
        g: 0.8,
        b: 0.4,
        a: 1.0,
    };
    pub const DATA_FLOW: Color = Color {
        r: 0.4,
        g: 0.7,
        b: 0.9,
        a: 1.0,
    };
    pub const STATUS: Color = Color {
        r: 0.6,
        g: 0.6,
        b: 0.6,
        a: 1.0,
    };
    pub const SEPARATOR: Color = Color {
        r: 0.4,
        g: 0.6,
        b: 1.0,
        a: 1.0,
    };
}

fn lighten(c: Color, amount: f32) -> Color {
    Color {
        r: (c.r + amount).min(1.0),
        g: (c.g + amount).min(1.0),
        b: (c.b + amount).min(1.0),
        a: c.a,
    }
}

fn darken(c: Color, amount: f32) -> Color {
    Color {
        r: (c.r - amount).max(0.0),
        g: (c.g - amount).max(0.0),
        b: (c.b - amount).max(0.0),
        a: c.a,
    }
}
