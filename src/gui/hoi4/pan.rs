use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

use super::Message;

const SCROLLBAR_HIT_AREA: f32 = 16.0;

#[derive(Debug, Default)]
struct State {
    dragging: bool,
    last_position: Option<Point>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Action {
    Ignored,
    Started,
    Moved(Vector),
    Zoom(f32),
    Ended,
}

fn is_scrollbar_area(position: Point, bounds: Rectangle) -> bool {
    position.x >= bounds.x + bounds.width - SCROLLBAR_HIT_AREA
        || position.y >= bounds.y + bounds.height - SCROLLBAR_HIT_AREA
}

fn update_state(
    state: &mut State,
    event: &Event,
    cursor: mouse::Cursor,
    bounds: Rectangle,
) -> Action {
    match event {
        Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
            let y = match delta {
                mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => *y,
            };
            if y != 0.0
                && cursor.position().is_some_and(|position| {
                    bounds.contains(position) && !is_scrollbar_area(position, bounds)
                })
            {
                Action::Zoom(y.signum())
            } else {
                Action::Ignored
            }
        }
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
            if let Some(position) = cursor.position().filter(|position| {
                bounds.contains(*position) && !is_scrollbar_area(*position, bounds)
            }) {
                state.dragging = true;
                state.last_position = Some(position);
                Action::Started
            } else {
                Action::Ignored
            }
        }
        Event::Mouse(mouse::Event::CursorMoved { position }) if state.dragging => {
            let delta = state
                .last_position
                .replace(*position)
                .map(|last| Vector::new(position.x - last.x, position.y - last.y));
            delta.map_or(Action::Ignored, Action::Moved)
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
            state.dragging = false;
            state.last_position = None;
            Action::Ended
        }
        _ => Action::Ignored,
    }
}

fn scroll_event(delta: Vector) -> Event {
    Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels {
            x: delta.x,
            y: delta.y,
        },
    })
}

pub(super) struct MousePan<'a> {
    content: Element<'a, Message>,
}

impl<'a> MousePan<'a> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl Widget<Message, Theme, Renderer> for MousePan<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
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
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        let state = tree.state.downcast_mut::<State>();
        match update_state(state, event, cursor, layout.bounds()) {
            Action::Started | Action::Ended => {
                shell.capture_event();
                return;
            }
            Action::Moved(delta) => {
                if delta != Vector::ZERO {
                    let event = scroll_event(delta);
                    self.content.as_widget_mut().update(
                        &mut tree.children[0],
                        &event,
                        layout,
                        mouse::Cursor::Available(layout.bounds().center()),
                        renderer,
                        clipboard,
                        shell,
                        viewport,
                    );
                }
                shell.capture_event();
                return;
            }
            Action::Zoom(delta) => {
                shell.publish(Message::Zoom(delta));
                shell.capture_event();
                return;
            }
            Action::Ignored => {}
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
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
        let state = tree.state.downcast_ref::<State>();
        if state.dragging {
            mouse::Interaction::Grabbing
        } else {
            let interaction = self.content.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            );
            if interaction == mouse::Interaction::None
                && cursor.position().is_some_and(|position| {
                    layout.bounds().contains(position)
                        && !is_scrollbar_area(position, layout.bounds())
                })
            {
                mouse::Interaction::Grab
            } else {
                interaction
            }
        }
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> Rectangle {
        Rectangle {
            x: 10.0,
            y: 20.0,
            width: 200.0,
            height: 100.0,
        }
    }

    #[test]
    fn left_drag_pans_and_release_stops_the_gesture() {
        let mut state = State::default();
        assert_eq!(
            update_state(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                mouse::Cursor::Available(Point::new(30.0, 40.0)),
                bounds(),
            ),
            Action::Started
        );
        assert!(state.dragging);
        assert_eq!(
            update_state(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(42.0, 35.0)
                }),
                mouse::Cursor::Available(Point::new(42.0, 35.0)),
                bounds(),
            ),
            Action::Moved(Vector::new(12.0, -5.0))
        );
        assert_eq!(
            scroll_event(Vector::new(12.0, -5.0)),
            Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 12.0, y: -5.0 }
            })
        );
        assert_eq!(
            update_state(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(250.0, 160.0)
                }),
                mouse::Cursor::Available(Point::new(250.0, 160.0)),
                bounds(),
            ),
            Action::Moved(Vector::new(208.0, 125.0))
        );
        assert_eq!(
            update_state(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                mouse::Cursor::Unavailable,
                bounds(),
            ),
            Action::Ended
        );
        assert!(!state.dragging);
        assert_eq!(state.last_position, None);
    }

    #[test]
    fn scrollbar_clicks_and_other_buttons_do_not_start_panning() {
        let mut state = State::default();
        for position in [Point::new(205.0, 50.0), Point::new(50.0, 115.0)] {
            assert_eq!(
                update_state(
                    &mut state,
                    &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    mouse::Cursor::Available(position),
                    bounds(),
                ),
                Action::Ignored
            );
        }
        assert_eq!(
            update_state(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
                mouse::Cursor::Available(Point::new(40.0, 40.0)),
                bounds(),
            ),
            Action::Ignored
        );
        assert!(!state.dragging);
    }

    #[test]
    fn wheel_over_tree_zooms_and_wheel_over_scrollbars_remains_unhandled() {
        let event = Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
        });
        let mut state = State::default();
        assert_eq!(
            update_state(
                &mut state,
                &event,
                mouse::Cursor::Available(Point::new(40.0, 40.0)),
                bounds(),
            ),
            Action::Zoom(1.0)
        );
        assert_eq!(
            update_state(
                &mut state,
                &event,
                mouse::Cursor::Available(Point::new(205.0, 50.0)),
                bounds(),
            ),
            Action::Ignored
        );
        assert_eq!(
            update_state(&mut state, &event, mouse::Cursor::Unavailable, bounds()),
            Action::Ignored
        );
    }
}
