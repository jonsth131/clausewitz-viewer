use std::collections::{HashMap, HashSet};

use iced::advanced::graphics::geometry::Renderer as GeometryRenderer;
use iced::advanced::renderer::Renderer as _;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell, Widget};
use iced::widget::canvas::{self, Path, Stroke};
use iced::{Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

use crate::hoi4::focus::FocusTree;

use super::Message;

pub(super) struct GridRow {
    pub child: usize,
    pub cells: Vec<GridCell>,
}

pub(super) struct GridCell {
    pub child: usize,
    pub focuses: Vec<usize>,
}

#[derive(Debug, PartialEq, Eq)]
struct Edge {
    from: usize,
    to: usize,
    muted: bool,
}

fn edges(tree: &FocusTree) -> Vec<Edge> {
    let mut ids = HashMap::new();
    for (index, focus) in tree
        .focuses
        .iter()
        .enumerate()
        .filter(|(_, f)| !f.id.is_empty())
    {
        ids.entry(focus.id.as_str())
            .and_modify(|entry| *entry = None)
            .or_insert(Some(index));
    }
    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    for (to, focus) in tree.focuses.iter().enumerate() {
        if focus.position.is_none() {
            continue;
        }
        for id in focus.prerequisites.iter().flatten() {
            let Some(from) = ids.get(id.as_str()).copied().flatten() else {
                continue;
            };
            if from != to && tree.focuses[from].position.is_some() && seen.insert((from, to)) {
                edges.push(Edge {
                    from,
                    to,
                    muted: focus.hidden || tree.focuses[from].hidden,
                });
            }
        }
    }
    edges
}

pub(super) fn muted_color(theme: &Theme) -> Color {
    let palette = theme.palette();
    let mix = |text: f32, background: f32| text * 0.55 + background * 0.45;
    Color {
        r: mix(palette.text.r, palette.background.r),
        g: mix(palette.text.g, palette.background.g),
        b: mix(palette.text.b, palette.background.b),
        a: palette.text.a,
    }
}

pub(super) struct ArrowGrid<'a> {
    content: Element<'a, Message>,
    rows: Vec<GridRow>,
    edges: Vec<Edge>,
    count: usize,
}

impl<'a> ArrowGrid<'a> {
    pub fn new(content: Element<'a, Message>, rows: Vec<GridRow>, tree: &FocusTree) -> Self {
        Self {
            content,
            rows,
            edges: edges(tree),
            count: tree.focuses.len(),
        }
    }
}

fn node_rectangles(layout: Layout<'_>, rows: &[GridRow], count: usize) -> Vec<Option<Rectangle>> {
    let mut rectangles = vec![None; count];
    let row_layouts: Vec<_> = layout.children().collect();
    for row in rows {
        let cells: Vec<_> = row_layouts[row.child].children().collect();
        for cell in &row.cells {
            for (index, node) in cell.focuses.iter().zip(cells[cell.child].children()) {
                rectangles[*index] = Some(node.bounds());
            }
        }
    }
    rectangles
}

fn orthogonal_route(from: Rectangle, to: Rectangle) -> Option<Vec<Point>> {
    let a = from.center();
    let b = to.center();
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dx.abs() < 0.01 && dy.abs() < 0.01 {
        return None;
    }

    let mut points = Vec::with_capacity(3);
    if dy.abs() < 0.01 {
        let direction = dx.signum();
        points.push(Point::new(a.x + direction * from.width / 2.0, a.y));
        points.push(Point::new(b.x - direction * to.width / 2.0, b.y));
    } else if dy > 0.0 {
        if dx.abs() < 0.01 {
            points.push(Point::new(a.x, from.y + from.height));
            points.push(Point::new(b.x, to.y));
        } else if to.y >= from.y + from.height {
            let start_y = from.y + from.height;
            let bend_y = (start_y + to.y) / 2.0;
            points.push(Point::new(a.x, start_y));
            points.push(Point::new(a.x, bend_y));
            points.push(Point::new(b.x, bend_y));
            points.push(Point::new(b.x, to.y));
        } else {
            let direction = dx.signum();
            points.push(Point::new(a.x + direction * from.width / 2.0, a.y));
            points.push(Point::new(b.x, a.y));
            points.push(Point::new(b.x, to.y));
        }
    } else if dx.abs() < 0.01 {
        points.push(Point::new(a.x, from.y));
        points.push(Point::new(b.x, to.y + to.height));
    } else {
        let direction = dx.signum();
        points.push(Point::new(a.x + direction * from.width / 2.0, a.y));
        points.push(Point::new(b.x, a.y));
        points.push(Point::new(b.x, to.y + to.height));
    }

    (points.len() >= 2).then_some(points)
}

impl Widget<Message, Theme, Renderer> for ArrowGrid<'_> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.content.as_widget().tag()
    }
    fn state(&self) -> iced::advanced::widget::tree::State {
        self.content.as_widget().state()
    }
    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }
    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let rectangles = node_rectangles(layout, &self.rows, self.count);
        if !self.edges.is_empty() && bounds.width > 0.0 && bounds.height > 0.0 {
            let mut frame = canvas::Frame::new(renderer, bounds.size());
            for edge in &self.edges {
                let (Some(from), Some(to)) = (rectangles[edge.from], rectangles[edge.to]) else {
                    continue;
                };
                let Some(points) = orthogonal_route(from, to) else {
                    continue;
                };
                let points: Vec<_> = points
                    .into_iter()
                    .map(|point| Point::new(point.x - bounds.x, point.y - bounds.y))
                    .collect();
                let from = points[points.len() - 2];
                let to = points[points.len() - 1];
                let angle = (to.y - from.y).atan2(to.x - from.x);
                let left = Point::new(
                    to.x - 8.0 * (angle - 0.45).cos(),
                    to.y - 8.0 * (angle - 0.45).sin(),
                );
                let right = Point::new(
                    to.x - 8.0 * (angle + 0.45).cos(),
                    to.y - 8.0 * (angle + 0.45).sin(),
                );
                let path = Path::new(|builder| {
                    builder.move_to(points[0]);
                    for point in points.iter().skip(1) {
                        builder.line_to(*point);
                    }
                    builder.move_to(left);
                    builder.line_to(to);
                    builder.line_to(right);
                });
                let color = if edge.muted {
                    muted_color(theme)
                } else {
                    style.text_color
                };
                frame.stroke(&path, Stroke::default().with_width(2.0).with_color(color));
            }

            renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
                renderer.draw_geometry(frame.into_geometry());
            });
        }
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hoi4::focus, parser::parse_config_entries};
    use std::collections::BTreeMap;

    #[test]
    fn resolves_branching_edges_and_omits_ambiguous_or_unplaced_nodes() {
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(
                "focus_tree = {
                    focus = { id = a x = 0 y = 0 }
                    focus = { id = b x = 1 y = 0 hidden = yes }
                    focus = { id = c x = 2 y = 1 prerequisite = { focus = a focus = b } prerequisite = { focus = a focus = missing focus = loose focus = duplicate } }
                    focus = { id = d x = 2 y = 2 hidden = yes prerequisite = { focus = c } }
                    focus = { id = loose prerequisite = { focus = a } }
                    focus = { id = duplicate x = 4 y = 0 }
                    focus = { id = duplicate x = 5 y = 0 }
                }",
            )
            .unwrap(),
        )]);
        let tree = &focus::extract(&files)[0];
        assert_eq!(
            edges(tree),
            [
                Edge {
                    from: 0,
                    to: 2,
                    muted: false
                },
                Edge {
                    from: 1,
                    to: 2,
                    muted: true
                },
                Edge {
                    from: 2,
                    to: 3,
                    muted: true
                }
            ]
        );
    }

    #[test]
    fn routes_focus_arrows_orthogonally_around_node_edges() {
        let from = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 40.0,
        };
        let to = Rectangle {
            x: 0.0,
            y: 100.0,
            width: 100.0,
            height: 40.0,
        };
        assert_eq!(
            orthogonal_route(from, to),
            Some(vec![Point::new(50.0, 40.0), Point::new(50.0, 100.0)])
        );

        let right = Rectangle {
            x: 200.0,
            y: 100.0,
            width: 100.0,
            height: 40.0,
        };
        assert_eq!(
            orthogonal_route(from, right),
            Some(vec![
                Point::new(50.0, 40.0),
                Point::new(50.0, 70.0),
                Point::new(250.0, 70.0),
                Point::new(250.0, 100.0)
            ])
        );

        let same_row = Rectangle {
            x: 200.0,
            y: 0.0,
            width: 100.0,
            height: 40.0,
        };
        assert_eq!(
            orthogonal_route(from, same_row),
            Some(vec![Point::new(100.0, 20.0), Point::new(200.0, 20.0)])
        );

        let above = Rectangle {
            x: 200.0,
            y: -100.0,
            width: 100.0,
            height: 40.0,
        };
        assert_eq!(
            orthogonal_route(from, above),
            Some(vec![
                Point::new(100.0, 20.0),
                Point::new(250.0, 20.0),
                Point::new(250.0, -60.0)
            ])
        );
        assert_eq!(orthogonal_route(from, from), None);
    }

    #[test]
    fn finds_node_bounds_in_nested_grid_and_tracks_translation() {
        let size = Size::new(200.0, 30.0);
        let cell = layout::Node::with_children(
            Size::new(200.0, 66.0),
            vec![
                layout::Node::new(size),
                layout::Node::new(size).move_to(Point::new(0.0, 36.0)),
            ],
        )
        .move_to(Point::new(20.0, 0.0));
        let row = layout::Node::with_children(
            Size::new(220.0, 66.0),
            vec![layout::Node::new(Size::new(20.0, 1.0)), cell],
        );
        let grid = layout::Node::with_children(Size::new(220.0, 66.0), vec![row]);
        let rows = [GridRow {
            child: 0,
            cells: vec![GridCell {
                child: 1,
                focuses: vec![1, 0],
            }],
        }];
        let bounds = node_rectangles(
            Layout::with_offset(Vector::new(12.0, 25.0), &grid),
            &rows,
            3,
        );
        assert_eq!(bounds[1].unwrap().position(), Point::new(32.0, 25.0));
        assert_eq!(bounds[0].unwrap().position(), Point::new(32.0, 61.0));
        assert!(bounds[2].is_none());
    }

    #[test]
    fn muted_colors_follow_light_and_dark_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            let palette = theme.palette();
            let muted = muted_color(&theme);
            assert_ne!(muted, palette.text);
            assert_ne!(muted, palette.background);
            assert!(
                (muted.r - (palette.text.r * 0.55 + palette.background.r * 0.45)).abs() < 0.0001
            );
        }
    }

    #[test]
    fn renders_arrow_over_actual_widget_layout() {
        use iced::advanced::graphics::core::renderer::Headless;
        use iced::widget::{column, container, row, text, Space};

        fn render(with_edge: bool) -> Vec<u8> {
            let input = if with_edge {
                "focus_tree = { focus = { id = a x = 0 y = 0 } focus = { id = b x = 0 y = 1 prerequisite = { focus = a } } }"
            } else {
                "focus_tree = { focus = { id = a x = 0 y = 0 } focus = { id = b x = 0 y = 1 } }"
            };
            let files = BTreeMap::from([(
                "common/national_focus/sample.txt".into(),
                parse_config_entries(input).unwrap(),
            )]);
            let trees = focus::extract(&files);
            let node = |name| container(text(name)).width(200).padding(5);
            let grid = column![
                row![column![node("a")]],
                Space::new().height(40),
                row![column![node("b")]]
            ];
            let rows = vec![
                GridRow {
                    child: 0,
                    cells: vec![GridCell {
                        child: 0,
                        focuses: vec![0],
                    }],
                },
                GridRow {
                    child: 2,
                    cells: vec![GridCell {
                        child: 0,
                        focuses: vec![1],
                    }],
                },
            ];
            let mut widget = ArrowGrid::new(grid.into(), rows, &trees[0]);
            let mut renderer = iced::futures::executor::block_on(<Renderer as Headless>::new(
                iced::Font::default(),
                16.into(),
                Some("tiny-skia"),
            ))
            .expect("software renderer");
            let mut state = Tree::new(&widget as &dyn Widget<Message, Theme, Renderer>);
            let limits = layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0));
            let node = widget.layout(&mut state, &renderer, &limits);
            widget.draw(
                &state,
                &mut renderer,
                &Theme::Dark,
                &renderer::Style {
                    text_color: Color::WHITE,
                },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::new(Point::ORIGIN, Size::new(300.0, 200.0)),
            );
            renderer.screenshot(Size::new(300, 200), 1.0, Color::BLACK)
        }

        assert_ne!(render(true), render(false));
    }
}
