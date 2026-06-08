//! Design system for flowrgui — centralized colors, spacing, typography, and styling.
#![allow(dead_code)]

use iced::widget::button;
use iced::{Background, Border, Color, Shadow, Theme, Vector};

// ── Surface colors ──────────────────────────────────────────────────────────

pub const SURFACE_BUTTON: Color = Color {
    r: 0.25,
    g: 0.25,
    b: 0.35,
    a: 1.0,
};

pub const SURFACE_TOOLTIP: Color = Color {
    r: 0.15,
    g: 0.15,
    b: 0.2,
    a: 0.95,
};

// ── Text colors ─────────────────────────────────────────────────────────────

pub const TEXT_SECONDARY: Color = Color {
    r: 0.6,
    g: 0.6,
    b: 0.6,
    a: 1.0,
};

pub const TEXT_LINK: Color = Color {
    r: 0.3,
    g: 0.6,
    b: 1.0,
    a: 1.0,
};

pub const TEXT_ERROR: Color = Color {
    r: 0.8,
    g: 0.4,
    b: 0.4,
    a: 1.0,
};

// ── Accent ───────────────────────────────────────────────────────────────────

pub const ACCENT: Color = Color {
    r: 0.424,
    g: 0.361,
    b: 0.906,
    a: 1.0,
};

// ── Entity type colors (consistent across the app) ──────────────────────────

#[allow(dead_code)]
pub mod entity_colors {
    use iced::Color;

    pub const FUNCTION: Color = Color {
        r: 0.2,
        g: 0.55,
        b: 1.0,
        a: 1.0,
    };
    pub const FLOW: Color = Color {
        r: 0.65,
        g: 0.35,
        b: 1.0,
        a: 1.0,
    };
    pub const JOB: Color = Color {
        r: 0.2,
        g: 0.85,
        b: 0.35,
        a: 1.0,
    };
    pub const INPUT: Color = Color {
        r: 1.0,
        g: 0.55,
        b: 0.15,
        a: 1.0,
    };
    pub const OUTPUT: Color = Color {
        r: 0.1,
        g: 0.85,
        b: 0.75,
        a: 1.0,
    };
}

// ── Debug event colors ──────────────────────────────────────────────────────

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

// ── Spacing scale ───────────────────────────────────────────────────────────

pub const SPACE_XS: f32 = 2.0;
pub const SPACE_SM: f32 = 4.0;
pub const SPACE_MD: f32 = 8.0;
pub const SPACE_LG: f32 = 16.0;

// ── Font sizes ──────────────────────────────────────────────────────────────

pub const FONT_SM: f32 = 12.0;
pub const FONT_MD: f32 = 14.0;
pub const FONT_DEFAULT: f32 = 16.0;

// ── Border radii ────────────────────────────────────────────────────────────

pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 8.0;

// ── Button padding ─────────────────────────────────────────────────────────

pub const BUTTON_PAD: [f32; 2] = [3.0, 6.0];
pub const BUTTON_PAD_SM: [f32; 2] = [3.0, 5.0];

// ── Button styles ───────────────────────────────────────────────────────────

pub fn styled_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let base = button::Style {
        border: Border {
            radius: RADIUS_MD.into(),
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
            background: Some(Background::Color(SURFACE_BUTTON)),
            text_color: palette.text,
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(ACCENT)),
            text_color: Color::WHITE,
            border: Border {
                radius: RADIUS_MD.into(),
                width: 2.0,
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

pub fn pill_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let border = Border {
        radius: 12.0.into(),
        width: 1.0,
        color: Color {
            a: 0.3,
            ..palette.text
        },
    };
    match status {
        button::Status::Active => button::Style {
            background: None,
            text_color: palette.text,
            border,
            ..button::Style::default()
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(ACCENT)),
            text_color: Color::WHITE,
            border: Border {
                color: lighten(ACCENT, 0.3),
                ..border
            },
            ..button::Style::default()
        },
        _ => styled_button(theme, status),
    }
}

pub fn pill_button_active(theme: &Theme, status: button::Status) -> button::Style {
    let _palette = theme.palette();
    let border = Border {
        radius: 12.0.into(),
        width: 1.0,
        color: lighten(ACCENT, 0.3),
    };
    match status {
        button::Status::Active | button::Status::Disabled => button::Style {
            background: Some(Background::Color(ACCENT)),
            text_color: Color::WHITE,
            border,
            ..button::Style::default()
        },
        _ => pill_button(theme, status),
    }
}

pub(crate) fn list_button(theme: &Theme, status: button::Status) -> button::Style {
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

// ── Toggler style ───────────────────────────────────────────────────────────

pub fn accent_toggler(
    theme: &Theme,
    status: iced::widget::toggler::Status,
) -> iced::widget::toggler::Style {
    let palette = theme.palette();
    let make_style = |bg: Color, fg: Color| iced::widget::toggler::Style {
        background: Background::Color(bg),
        foreground: Background::Color(fg),
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        background_border_width: 0.0,
        background_border_color: Color::TRANSPARENT,
        text_color: Some(palette.text),
        border_radius: None,
        padding_ratio: 0.1,
    };

    match status {
        iced::widget::toggler::Status::Active { is_toggled } => {
            if is_toggled {
                make_style(ACCENT, palette.text)
            } else {
                make_style(
                    Color {
                        a: 0.2,
                        ..palette.text
                    },
                    palette.text,
                )
            }
        }
        iced::widget::toggler::Status::Hovered { is_toggled } => {
            if is_toggled {
                make_style(lighten(ACCENT, 0.15), Color::WHITE)
            } else {
                make_style(
                    Color {
                        a: 0.3,
                        ..palette.text
                    },
                    Color::WHITE,
                )
            }
        }
        iced::widget::toggler::Status::Disabled { .. } => make_style(
            Color {
                a: 0.1,
                ..palette.text
            },
            Color {
                a: 0.3,
                ..palette.text
            },
        ),
    }
}

// ── Text input style ────────────────────────────────────────────────────────

pub fn pill_input(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let palette = theme.palette();
    let base = iced::widget::text_input::Style {
        background: iced::Background::Color(Color::TRANSPARENT),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: Color {
                a: 0.3,
                ..palette.text
            },
        },
        icon: palette.text,
        placeholder: Color {
            a: 0.4,
            ..palette.text
        },
        value: palette.text,
        selection: Color { a: 0.3, ..ACCENT },
    };

    match status {
        iced::widget::text_input::Status::Focused { .. } => iced::widget::text_input::Style {
            border: Border {
                color: lighten(ACCENT, 0.3),
                ..base.border
            },
            ..base
        },
        iced::widget::text_input::Status::Hovered => iced::widget::text_input::Style {
            border: Border {
                color: Color {
                    a: 0.5,
                    ..palette.text
                },
                ..base.border
            },
            ..base
        },
        _ => base,
    }
}

// ── Card/popup style ────────────────────────────────────────────────────────

pub fn popup_card(theme: &Theme, _status: iced_aw::style::Status) -> iced_aw::style::card::Style {
    let palette = theme.palette();
    iced_aw::style::card::Style {
        background: Background::Color(SURFACE_BUTTON),
        border_radius: RADIUS_MD,
        border_width: 1.5,
        border_color: Color { a: 0.5, ..ACCENT },
        head_background: Background::Color(Color { a: 0.3, ..ACCENT }),
        head_text_color: Color::WHITE,
        body_background: Background::Color(SURFACE_BUTTON),
        body_text_color: palette.text,
        foot_background: Background::Color(SURFACE_BUTTON),
        foot_text_color: palette.text,
        close_color: Color::WHITE,
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

pub fn lighten(c: Color, amount: f32) -> Color {
    Color {
        r: (c.r + amount).min(1.0),
        g: (c.g + amount).min(1.0),
        b: (c.b + amount).min(1.0),
        a: c.a,
    }
}

pub fn darken(c: Color, amount: f32) -> Color {
    Color {
        r: (c.r - amount).max(0.0),
        g: (c.g - amount).max(0.0),
        b: (c.b - amount).max(0.0),
        a: c.a,
    }
}
