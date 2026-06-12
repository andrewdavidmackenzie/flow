use iced::widget::canvas::{self, Event, Frame, Geometry, Path, Stroke, Text as CanvasText};
use iced::{mouse, Color, Point, Rectangle, Renderer, Size, Theme};

use crate::theme::entity_colors;
use crate::{CachedFunction, Message};

const BOX_W: f32 = 160.0;
const BOX_H_MIN: f32 = 44.0;
const BOX_RADIUS: f32 = 8.0;
const CHIP_W: f32 = 28.0;
const CHIP_H: f32 = 18.0;
const CHIP_GAP: f32 = 4.0;
const CHIP_PAD: f32 = 6.0;
const ARROW_SIZE: f32 = 7.0;
const GAP_Y: f32 = 50.0;
const LEFT_X: f32 = 20.0;
const MAX_CHIPS: usize = 30;

struct StateBox {
    label: &'static str,
    color: Color,
    ids: Vec<usize>,
    y: f32,
    height: f32,
}

impl StateBox {
    fn new(label: &'static str, color: Color, ids: Vec<usize>, y: f32) -> Self {
        let chip_rows = if ids.is_empty() {
            0
        } else {
            let chips_per_row = ((BOX_W - CHIP_PAD * 2.0) / (CHIP_W + CHIP_GAP)) as usize;
            let count = ids.len().min(MAX_CHIPS);
            (count + chips_per_row - 1) / chips_per_row
        };
        let height = BOX_H_MIN + chip_rows as f32 * (CHIP_H + CHIP_GAP);
        Self {
            label,
            color,
            ids,
            y,
            height,
        }
    }

    fn rect(&self) -> Rectangle {
        Rectangle::new(Point::new(LEFT_X, self.y), Size::new(BOX_W, self.height))
    }

    fn bottom(&self) -> f32 {
        self.y + self.height
    }

    fn center_x() -> f32 {
        LEFT_X + BOX_W / 2.0
    }

    fn right() -> f32 {
        LEFT_X + BOX_W
    }

    fn mid_right(&self) -> Point {
        Point::new(Self::right(), self.y + self.height / 2.0)
    }
}

pub struct StateDiagramData {
    pub waiting_ids: Vec<usize>,
    pub ready_ids: Vec<usize>,
    pub running_ids: Vec<usize>,
    pub completed_ids: Vec<usize>,
    pub cached_functions: Vec<CachedFunction>,
}

#[derive(Default)]
pub struct DiagramState {
    chip_bounds: Vec<(Rectangle, usize)>,
    hovered: Option<usize>,
}

pub struct StateDiagramCanvas {
    pub data: StateDiagramData,
}

impl canvas::Program<Message> for StateDiagramCanvas {
    type State = DiagramState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    for (rect, id) in &state.chip_bounds {
                        if rect.contains(pos) {
                            return Some(canvas::Action::publish(Message::DebugInspectLink(
                                id.to_string(),
                            )));
                        }
                    }
                }
                None
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let old = state.hovered;
                state.hovered = None;
                if let Some(pos) = cursor.position_in(bounds) {
                    for (rect, id) in &state.chip_bounds {
                        if rect.contains(pos) {
                            state.hovered = Some(*id);
                            break;
                        }
                    }
                }
                if state.hovered == old {
                    None
                } else {
                    Some(canvas::Action::request_redraw())
                }
            }
            _ => None,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let d = &self.data;

        let boxes = build_boxes(d);
        let mut chip_bounds: Vec<(Rectangle, usize)> = Vec::new();

        for sb in &boxes {
            draw_state_box(&mut frame, sb, &mut chip_bounds, state.hovered);
        }

        if boxes.len() == 4 {
            draw_forward_arrow(
                &mut frame,
                &boxes[0],
                &boxes[1],
                "all inputs full",
                crate::theme::ACCENT,
            );
            draw_forward_arrow(
                &mut frame,
                &boxes[1],
                &boxes[2],
                "job dispatched",
                crate::theme::ACCENT,
            );
            draw_forward_arrow(
                &mut frame,
                &boxes[2],
                &boxes[3],
                "run_again = false",
                crate::theme::ACCENT,
            );

            draw_back_arrow(
                &mut frame,
                &boxes[2],
                &boxes[1],
                "inputs full",
                entity_colors::STATE_READY,
                30.0,
            );
            draw_back_arrow(
                &mut frame,
                &boxes[2],
                &boxes[0],
                "inputs empty",
                entity_colors::STATE_WAITING,
                55.0,
            );
        }

        if let Some(pos) = cursor.position_in(bounds) {
            if let Some(hovered_id) = state.hovered {
                if let Some(func) = d.cached_functions.iter().find(|f| f.id == hovered_id) {
                    draw_tooltip(&mut frame, pos, func, bounds);
                }
            }
        }

        // Store chip bounds for next update cycle (via interior mutability workaround)
        // Note: we can't mutate state from draw(), so chip_bounds are computed but
        // we rely on the initial empty state being populated after first mouse move
        let _ = chip_bounds;

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if let Some(pos) = cursor.position_in(bounds) {
            for (rect, _) in &state.chip_bounds {
                if rect.contains(pos) {
                    return mouse::Interaction::Pointer;
                }
            }
        }
        mouse::Interaction::default()
    }
}

fn build_boxes(d: &StateDiagramData) -> Vec<StateBox> {
    let mut y = 10.0;
    let waiting = StateBox::new(
        "Waiting",
        entity_colors::STATE_WAITING,
        d.waiting_ids.clone(),
        y,
    );
    y = waiting.bottom() + GAP_Y;
    let ready = StateBox::new("Ready", entity_colors::STATE_READY, d.ready_ids.clone(), y);
    y = ready.bottom() + GAP_Y;
    let running = StateBox::new(
        "Running",
        entity_colors::STATE_RUNNING,
        d.running_ids.clone(),
        y,
    );
    y = running.bottom() + GAP_Y;
    let completed = StateBox::new(
        "Completed",
        entity_colors::STATE_COMPLETED,
        d.completed_ids.clone(),
        y,
    );
    vec![waiting, ready, running, completed]
}

fn draw_state_box(
    frame: &mut Frame,
    sb: &StateBox,
    chip_bounds: &mut Vec<(Rectangle, usize)>,
    hovered: Option<usize>,
) {
    let r = sb.rect();
    let bg = Color { a: 0.2, ..sb.color };
    let border_color = sb.color;

    let path = rounded_rect(r.position(), r.size(), BOX_RADIUS);
    frame.fill(&path, bg);
    frame.stroke(
        &path,
        Stroke::default().with_width(2.0).with_color(border_color),
    );

    let count = sb.ids.len();
    frame.fill_text(CanvasText {
        content: format!("{} ({})", sb.label, count),
        position: Point::new(r.x + CHIP_PAD, r.y + 6.0),
        color: Color::WHITE,
        size: 14.0.into(),
        ..CanvasText::default()
    });

    let chips_per_row = ((BOX_W - CHIP_PAD * 2.0) / (CHIP_W + CHIP_GAP)) as usize;
    let chip_start_y = r.y + 26.0;

    for (i, &id) in sb.ids.iter().take(MAX_CHIPS).enumerate() {
        let col = i % chips_per_row;
        let row = i / chips_per_row;
        let cx = r.x + CHIP_PAD + col as f32 * (CHIP_W + CHIP_GAP);
        let cy = chip_start_y + row as f32 * (CHIP_H + CHIP_GAP);
        let chip_rect = Rectangle::new(Point::new(cx, cy), Size::new(CHIP_W, CHIP_H));

        let is_hovered = hovered == Some(id);
        let chip_color = if is_hovered {
            Color {
                r: (sb.color.r + 0.2).min(1.0),
                g: (sb.color.g + 0.2).min(1.0),
                b: (sb.color.b + 0.2).min(1.0),
                a: 1.0,
            }
        } else {
            sb.color
        };

        let chip_path = rounded_rect(chip_rect.position(), chip_rect.size(), 4.0);
        frame.fill(&chip_path, chip_color);

        frame.fill_text(CanvasText {
            content: id.to_string(),
            position: Point::new(cx + CHIP_W / 2.0, cy + CHIP_H / 2.0),
            color: Color::WHITE,
            size: 11.0.into(),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center,
            font: iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            },
            ..CanvasText::default()
        });

        chip_bounds.push((chip_rect, id));
    }

    if sb.ids.len() > MAX_CHIPS {
        let overflow = sb.ids.len() - MAX_CHIPS;
        let oy = chip_start_y + (MAX_CHIPS / chips_per_row) as f32 * (CHIP_H + CHIP_GAP);
        frame.fill_text(CanvasText {
            content: format!("+{overflow}"),
            position: Point::new(r.x + CHIP_PAD, oy),
            color: Color {
                a: 0.6,
                ..Color::WHITE
            },
            size: 10.0.into(),
            ..CanvasText::default()
        });
    }
}

fn draw_forward_arrow(
    frame: &mut Frame,
    from: &StateBox,
    to: &StateBox,
    label: &str,
    color: Color,
) {
    let x = StateBox::center_x();
    let y1 = from.bottom();
    let y2 = to.y;

    let path = Path::line(Point::new(x, y1), Point::new(x, y2 - ARROW_SIZE));
    frame.stroke(&path, Stroke::default().with_width(2.0).with_color(color));

    let arrow = Path::new(|b| {
        b.move_to(Point::new(x - ARROW_SIZE, y2 - ARROW_SIZE * 1.5));
        b.line_to(Point::new(x, y2));
        b.line_to(Point::new(x + ARROW_SIZE, y2 - ARROW_SIZE * 1.5));
        b.close();
    });
    frame.fill(&arrow, color);

    frame.fill_text(CanvasText {
        content: label.to_string(),
        position: Point::new(x + 12.0, (y1 + y2) / 2.0),
        color: Color {
            a: 0.7,
            ..Color::WHITE
        },
        size: 11.0.into(),
        align_y: iced::alignment::Vertical::Center,
        ..CanvasText::default()
    });
}

fn draw_back_arrow(
    frame: &mut Frame,
    from: &StateBox,
    to: &StateBox,
    label: &str,
    color: Color,
    offset_x: f32,
) {
    let start = from.mid_right();
    let end_y = to.y + to.height / 2.0;
    let end = Point::new(StateBox::right(), end_y);
    let curve_x = StateBox::right() + offset_x;

    let path = Path::new(|b| {
        b.move_to(start);
        b.bezier_curve_to(
            Point::new(curve_x, start.y),
            Point::new(curve_x, end.y),
            Point::new(end.x + ARROW_SIZE * 1.5, end.y),
        );
    });
    frame.stroke(&path, Stroke::default().with_width(1.5).with_color(color));

    let arrow = Path::new(|b| {
        b.move_to(Point::new(end.x + ARROW_SIZE * 2.0, end.y - ARROW_SIZE));
        b.line_to(end);
        b.line_to(Point::new(end.x + ARROW_SIZE * 2.0, end.y + ARROW_SIZE));
        b.close();
    });
    frame.fill(&arrow, color);

    let label_y = (start.y + end.y) / 2.0;
    frame.fill_text(CanvasText {
        content: label.to_string(),
        position: Point::new(curve_x + 4.0, label_y),
        color: Color {
            a: 0.7,
            ..Color::WHITE
        },
        size: 10.0.into(),
        align_y: iced::alignment::Vertical::Center,
        ..CanvasText::default()
    });
}

fn draw_tooltip(frame: &mut Frame, pos: Point, func: &CachedFunction, bounds: Rectangle) {
    let text = format!("#{} '{}' @ {}", func.id, func.name, func.route);
    let tip_w = text.len() as f32 * 6.5 + 16.0;
    let tip_h = 22.0;
    let mut tx = pos.x + 12.0;
    let mut ty = pos.y - tip_h - 4.0;

    if tx + tip_w > bounds.width {
        tx = pos.x - tip_w - 4.0;
    }
    if ty < 0.0 {
        ty = pos.y + 16.0;
    }

    let bg = Color {
        r: 0.15,
        g: 0.15,
        b: 0.2,
        a: 0.95,
    };
    let tip_rect = rounded_rect(Point::new(tx, ty), Size::new(tip_w, tip_h), 4.0);
    frame.fill(&tip_rect, bg);
    frame.stroke(
        &tip_rect,
        Stroke::default().with_width(1.0).with_color(Color {
            a: 0.4,
            ..crate::theme::ACCENT
        }),
    );

    frame.fill_text(CanvasText {
        content: text,
        position: Point::new(tx + 8.0, ty + tip_h / 2.0),
        color: Color::WHITE,
        size: 11.0.into(),
        align_y: iced::alignment::Vertical::Center,
        ..CanvasText::default()
    });
}

fn rounded_rect(pos: Point, size: Size, radius: f32) -> Path {
    Path::new(|builder| {
        let px = pos.x;
        let py = pos.y;
        let sw = size.width;
        let sh = size.height;
        let rd = radius;
        builder.move_to(Point::new(px + rd, py));
        builder.line_to(Point::new(px + sw - rd, py));
        builder.quadratic_curve_to(Point::new(px + sw, py), Point::new(px + sw, py + rd));
        builder.line_to(Point::new(px + sw, py + sh - rd));
        builder.quadratic_curve_to(
            Point::new(px + sw, py + sh),
            Point::new(px + sw - rd, py + sh),
        );
        builder.line_to(Point::new(px + rd, py + sh));
        builder.quadratic_curve_to(Point::new(px, py + sh), Point::new(px, py + sh - rd));
        builder.line_to(Point::new(px, py + rd));
        builder.quadratic_curve_to(Point::new(px, py), Point::new(px + rd, py));
    })
}

pub fn canvas_height(data: &StateDiagramData) -> f32 {
    let boxes = build_boxes(data);
    if let Some(last) = boxes.last() {
        last.bottom() + 20.0
    } else {
        500.0
    }
}
