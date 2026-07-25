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

pub(crate) struct StateDiagramData {
    pub(crate) waiting_ids: Vec<usize>,
    pub(crate) ready_ids: Vec<usize>,
    pub(crate) running_ids: Vec<usize>,
    pub(crate) completed_ids: Vec<usize>,
    pub(crate) cached_functions: Vec<CachedFunction>,
}

#[derive(Default)]
pub(crate) struct DiagramState {
    chip_bounds: Vec<(Rectangle, usize)>,
    hovered: Option<usize>,
}

pub(crate) struct StateDiagramCanvas {
    pub(crate) data: StateDiagramData,
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
        state.chip_bounds = compute_all_chip_bounds(&self.data);
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
                } else if let Some(pos) = cursor.position_in(bounds) {
                    let data = state.hovered.and_then(|id| {
                        self.data
                            .cached_functions
                            .iter()
                            .find(|f| f.id == id)
                            .map(|f| {
                                (
                                    format!("#{} '{}' @ {}", f.id, f.name, f.route),
                                    pos.x,
                                    pos.y,
                                )
                            })
                    });
                    Some(canvas::Action::publish(Message::DiagramHover(data)))
                } else {
                    Some(canvas::Action::publish(Message::DiagramHover(None)))
                }
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
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

fn compute_all_chip_bounds(data: &StateDiagramData) -> Vec<(Rectangle, usize)> {
    let boxes = build_boxes(data);
    let mut bounds = Vec::new();
    let chips_per_row = ((BOX_W - CHIP_PAD * 2.0) / (CHIP_W + CHIP_GAP)) as usize;
    for sb in &boxes {
        let r = sb.rect();
        let chip_start_y = r.y + 26.0;
        for (i, &id) in sb.ids.iter().take(MAX_CHIPS).enumerate() {
            let col = i % chips_per_row;
            let row = i / chips_per_row;
            let cx = r.x + CHIP_PAD + col as f32 * (CHIP_W + CHIP_GAP);
            let cy = chip_start_y + row as f32 * (CHIP_H + CHIP_GAP);
            bounds.push((
                Rectangle::new(Point::new(cx, cy), Size::new(CHIP_W, CHIP_H)),
                id,
            ));
        }
    }
    bounds
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

        let chip_path = rounded_rect(chip_rect.position(), chip_rect.size(), CHIP_H / 2.0);
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
            size: 12.0.into(),
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
            a: 0.8,
            ..Color::WHITE
        },
        size: 14.0.into(),
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
            a: 0.8,
            ..Color::WHITE
        },
        size: 13.0.into(),
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

pub(crate) fn canvas_height(data: &StateDiagramData) -> f32 {
    let boxes = build_boxes(data);
    if let Some(last) = boxes.last() {
        last.bottom() + 20.0
    } else {
        500.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::*;

    fn empty_data() -> StateDiagramData {
        StateDiagramData {
            waiting_ids: vec![],
            ready_ids: vec![],
            running_ids: vec![],
            completed_ids: vec![],
            cached_functions: vec![],
        }
    }

    fn sample_data() -> StateDiagramData {
        StateDiagramData {
            waiting_ids: vec![0, 1, 2],
            ready_ids: vec![3],
            running_ids: vec![4, 5],
            completed_ids: vec![],
            cached_functions: vec![],
        }
    }

    #[test]
    fn state_box_empty_has_minimum_height() {
        let sb = StateBox::new("Test", Color::WHITE, vec![], 0.0);
        assert!((sb.height - BOX_H_MIN).abs() < f32::EPSILON);
    }

    #[test]
    fn state_box_with_chips_is_taller() {
        let sb = StateBox::new("Test", Color::WHITE, vec![1, 2, 3], 0.0);
        assert!(sb.height > BOX_H_MIN);
    }

    #[test]
    fn state_box_bottom_equals_y_plus_height() {
        let sb = StateBox::new("Test", Color::WHITE, vec![1], 10.0);
        assert!((sb.bottom() - (10.0 + sb.height)).abs() < f32::EPSILON);
    }

    #[test]
    fn state_box_rect_position_and_size() {
        let sb = StateBox::new("Test", Color::WHITE, vec![], 50.0);
        let r = sb.rect();
        assert!((r.x - LEFT_X).abs() < f32::EPSILON);
        assert!((r.y - 50.0).abs() < f32::EPSILON);
        assert!((r.width - BOX_W).abs() < f32::EPSILON);
        assert!((r.height - sb.height).abs() < f32::EPSILON);
    }

    #[test]
    fn build_boxes_returns_four_boxes() {
        let boxes = build_boxes(&empty_data());
        assert_eq!(boxes.len(), 4);
        assert_eq!(boxes[0].label, "Waiting");
        assert_eq!(boxes[1].label, "Ready");
        assert_eq!(boxes[2].label, "Running");
        assert_eq!(boxes[3].label, "Completed");
    }

    #[test]
    fn build_boxes_are_vertically_ordered() {
        let boxes = build_boxes(&sample_data());
        for i in 1..boxes.len() {
            assert!(
                boxes[i].y > boxes[i - 1].bottom(),
                "{} should be below {}",
                boxes[i].label,
                boxes[i - 1].label
            );
        }
    }

    #[test]
    fn build_boxes_propagate_ids() {
        let boxes = build_boxes(&sample_data());
        assert_eq!(boxes[0].ids, vec![0, 1, 2]); // waiting
        assert_eq!(boxes[1].ids, vec![3]); // ready
        assert_eq!(boxes[2].ids, vec![4, 5]); // running
        assert!(boxes[3].ids.is_empty()); // completed
    }

    #[test]
    fn compute_chip_bounds_empty_data() {
        let bounds = compute_all_chip_bounds(&empty_data());
        assert!(bounds.is_empty());
    }

    #[test]
    fn compute_chip_bounds_matches_ids() {
        let data = sample_data();
        let bounds = compute_all_chip_bounds(&data);
        let ids: Vec<usize> = bounds.iter().map(|(_, id)| *id).collect();
        // All IDs from all state boxes should appear
        assert_eq!(ids, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn compute_chip_bounds_within_boxes() {
        let data = sample_data();
        let bounds = compute_all_chip_bounds(&data);
        let boxes = build_boxes(&data);

        for (rect, id) in &bounds {
            // Find which box this chip belongs to
            let parent_box = boxes
                .iter()
                .find(|sb| sb.ids.contains(id))
                .expect("chip should belong to a box");
            let box_rect = parent_box.rect();
            assert!(
                rect.x >= box_rect.x && rect.x + rect.width <= box_rect.x + box_rect.width,
                "Chip for id {id} should be horizontally within its box"
            );
        }
    }

    #[test]
    fn canvas_height_empty_data() {
        let h = canvas_height(&empty_data());
        assert!(h > 0.0);
    }

    #[test]
    fn canvas_height_increases_with_more_ids() {
        let small = canvas_height(&empty_data());
        let large = canvas_height(&sample_data());
        assert!(large > small, "More IDs should produce a taller canvas");
    }

    #[test]
    fn many_chips_respects_max_limit() {
        let data = StateDiagramData {
            waiting_ids: (0..50).collect(),
            ready_ids: vec![],
            running_ids: vec![],
            completed_ids: vec![],
            cached_functions: vec![],
        };
        let bounds = compute_all_chip_bounds(&data);
        // Only MAX_CHIPS chips should get bounds, not all 50
        let waiting_bounds: Vec<_> = bounds.iter().filter(|(_, id)| *id < 50).collect();
        assert_eq!(waiting_bounds.len(), MAX_CHIPS);
    }
}
