use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use iced::widget::{
    column, container, image, pick_list, scrollable, text, tooltip, Column, Row, Space,
};
use iced::{Element, Length};

use crate::hoi4::focus::{Focus, FocusTree, TreeChoice};
use crate::hoi4::icons::{IconImage, Icons};

mod arrows;
pub mod decisions;
pub mod events;
pub mod ideas;
mod pan;

const CELL_WIDTH: f32 = 200.0;
const COLUMN_GAP: f32 = 20.0;
const ROW_GAP: f32 = 30.0;
const MAX_EMPTY_CELLS: i64 = 20;
const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 2.5;
const ZOOM_STEP: f32 = 1.12;

#[derive(Default)]
struct Layout {
    rows: BTreeMap<i32, BTreeMap<i32, Vec<usize>>>,
    unplaced: Vec<usize>,
}

impl Layout {
    fn new(tree: &FocusTree) -> Self {
        let mut layout = Self::default();
        for (index, focus) in tree.focuses.iter().enumerate() {
            if let Some((x, y)) = focus.position {
                layout
                    .rows
                    .entry(y)
                    .or_default()
                    .entry(x)
                    .or_default()
                    .push(index);
            } else {
                layout.unplaced.push(index);
            }
        }
        layout
    }
}

fn column_offsets(layout: &Layout) -> (BTreeMap<i32, f32>, bool) {
    let columns: BTreeSet<i32> = layout
        .rows
        .values()
        .flat_map(|row| row.keys().copied())
        .collect();
    let mut offsets = BTreeMap::new();
    let mut previous = None;
    let mut offset = 0.0;
    let mut compressed = false;
    for x in columns {
        if let Some(previous_x) = previous {
            let distance = i64::from(x) - i64::from(previous_x);
            compressed |= distance - 1 > MAX_EMPTY_CELLS;
            offset += (CELL_WIDTH + COLUMN_GAP) * distance.min(MAX_EMPTY_CELLS + 1) as f32;
        }
        offsets.insert(x, offset);
        previous = Some(x);
    }
    (offsets, compressed)
}

fn prerequisites_text(focus: &Focus) -> Option<String> {
    if focus.prerequisites.is_empty() {
        return None;
    }
    let groups = focus
        .prerequisites
        .iter()
        .map(|group| {
            if group.len() > 1 {
                format!("({})", group.join(" OR "))
            } else {
                group.join("")
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ");
    Some(format!("Requires: {groups}"))
}

fn focus_node<'a>(
    focus: &'a Focus,
    icons: &HashMap<String, image::Handle>,
    zoom: f32,
) -> Element<'a, Message> {
    let node_text = |value| {
        text(value).style(move |theme: &iced::Theme| text::Style {
            color: focus.hidden.then(|| arrows::muted_color(theme)),
        })
    };
    let mut label = Column::new().spacing(4).width(Length::Fill);
    if let Some(handle) = focus.icon.as_ref().and_then(|icon| icons.get(icon)) {
        label = label.push(
            container(
                image(handle.clone())
                    .width(96.0 * zoom)
                    .height(96.0 * zoom)
                    .content_fit(iced::ContentFit::Contain),
            )
            .width(Length::Fill)
            .center_x(Length::Fill),
        );
    }
    label = label.push(node_text(focus.name.clone()).size(16.0 * zoom));
    if let Some(prerequisites) = prerequisites_text(focus) {
        label = label.push(node_text(prerequisites).size(12.0 * zoom));
    }
    tooltip(
        container(label)
            .padding(8.0 * zoom)
            .width(CELL_WIDTH * zoom)
            .style(container::rounded_box),
        container(text(&focus.tooltip))
            .padding(10)
            .max_width(500)
            .style(container::rounded_box),
        tooltip::Position::FollowCursor,
    )
    .into()
}

fn focus_layout<'a>(
    tree: &'a FocusTree,
    icons: &HashMap<String, image::Handle>,
    zoom: f32,
) -> Element<'a, Message> {
    let layout = Layout::new(tree);
    let mut content = Column::new().spacing(10.0 * zoom);
    let (offsets, mut compressed) = column_offsets(&layout);
    let mut grid = Column::new();
    let mut grid_rows = Vec::new();
    let mut grid_child = 0;
    let mut previous_y = None;
    for (y, cells) in &layout.rows {
        if let Some(previous) = previous_y {
            let empty = i64::from(*y) - i64::from(previous) - 1;
            compressed |= empty > MAX_EMPTY_CELLS;
            grid = grid.push(
                Space::new().height(ROW_GAP * (1 + empty.min(MAX_EMPTY_CELLS)) as f32 * zoom),
            );
            grid_child += 1;
        }
        let mut row = Row::new();
        let mut row_child = 0;
        let mut grid_cells = Vec::new();
        let mut row_end = 0.0;
        for (x, indexes) in cells {
            let gap = (offsets[x] - row_end) * zoom;
            if gap > 0.0 {
                row = row.push(Space::new().width(gap));
                row_child += 1;
            }
            let mut cell = Column::new().spacing(6.0 * zoom).width(CELL_WIDTH * zoom);
            for index in indexes {
                cell = cell.push(focus_node(&tree.focuses[*index], icons, zoom));
            }
            row = row.push(cell);
            grid_cells.push(arrows::GridCell {
                child: row_child,
                focuses: indexes.clone(),
            });
            row_child += 1;
            row_end = offsets[x] + CELL_WIDTH;
        }
        grid = grid.push(row);
        grid_rows.push(arrows::GridRow {
            child: grid_child,
            cells: grid_cells,
        });
        grid_child += 1;
        previous_y = Some(*y);
    }
    if compressed {
        content = content.push(text("Large gaps between positions are compressed."));
    }
    content = content.push(Element::new(arrows::ArrowGrid::new(
        grid.into(),
        grid_rows,
        tree,
    )));
    if !layout.unplaced.is_empty() {
        content = content.push(text("Focuses with unresolved positions:"));
        let mut unplaced = Column::new().spacing(6);
        for index in layout.unplaced {
            unplaced = unplaced.push(focus_node(&tree.focuses[index], icons, zoom));
        }
        content = content.push(unplaced);
    }
    let scrollable = scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Both {
            vertical: scrollable::Scrollbar::default(),
            horizontal: scrollable::Scrollbar::default(),
        });
    Element::new(pan::MousePan::new(scrollable))
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectTree(TreeChoice),
    Zoom(f32),
}

#[derive(Debug)]
pub struct FocusBrowser {
    trees: Vec<FocusTree>,
    choices: Vec<TreeChoice>,
    selected: Option<TreeChoice>,
    icons: HashMap<String, image::Handle>,
    zoom: f32,
}

impl Default for FocusBrowser {
    fn default() -> Self {
        Self {
            trees: Vec::new(),
            choices: Vec::new(),
            selected: None,
            icons: HashMap::new(),
            zoom: 1.0,
        }
    }
}

impl FocusBrowser {
    pub fn new(trees: Vec<FocusTree>, images: Icons) -> Self {
        let choices = trees.iter().map(|tree| tree.choice.clone()).collect();
        let mut handles = HashMap::<*const IconImage, image::Handle>::new();
        let icons = images
            .into_iter()
            .map(|(name, image)| {
                let handle = handles.entry(Arc::as_ptr(&image)).or_insert_with(|| {
                    image::Handle::from_rgba(image.width, image.height, image.pixels.clone())
                });
                (name, handle.clone())
            })
            .collect();
        Self {
            trees,
            choices,
            selected: None,
            icons,
            zoom: 1.0,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SelectTree(choice) if self.choices.contains(&choice) => {
                self.selected = Some(choice);
            }

            Message::SelectTree(_) => {}
            Message::Zoom(delta) if delta.is_finite() && delta != 0.0 => {
                self.zoom = (self.zoom * ZOOM_STEP.powf(delta.signum())).clamp(MIN_ZOOM, MAX_ZOOM);
            }
            Message::Zoom(_) => {}
        }
    }

    pub(super) fn icon_handles(&self) -> &HashMap<String, image::Handle> {
        &self.icons
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut content = column![text("Game: Hearts of Iron IV")].spacing(10);
        if self.choices.is_empty() {
            return container(content.push(text("No focus trees were loaded.")))
                .padding(20)
                .into();
        }
        content = content.push(
            pick_list(
                &self.choices[..],
                self.selected.as_ref(),
                Message::SelectTree,
            )
            .placeholder("Choose a focus tree"),
        );
        if let Some(tree) = self
            .selected
            .as_ref()
            .and_then(|choice| self.trees.get(choice.index))
        {
            if tree.focuses.is_empty() {
                content = content.push(text("No focuses defined in this tree."));
            } else {
                content = content.push(focus_layout(tree, &self.icons, self.zoom));
            }
        }
        container(content).padding(20).height(Length::Fill).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoi4::focus;
    use crate::parser::parse_config_entries;
    use std::collections::BTreeMap;

    #[test]
    fn selects_distinct_trees_and_ignores_invalid_choices() {
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(
                "focus_tree = { id = alpha focus = { id = one } } \
                 focus_tree = { id = beta focus = { id = two } }",
            )
            .unwrap(),
        )]);
        let mut browser = FocusBrowser::new(focus::extract(&files), Icons::new());
        assert_eq!(browser.choices.len(), 2);
        assert!(browser.selected.is_none());
        browser.update(Message::SelectTree(browser.choices[1].clone()));
        assert_eq!(browser.selected.as_ref().unwrap().index, 1);
        let _ = browser.view();
        browser.update(Message::SelectTree(TreeChoice {
            index: 10,
            label: "missing".into(),
        }));
        assert_eq!(browser.selected.as_ref().unwrap().index, 1);
    }

    #[test]
    fn empty_focus_browser_renders() {
        let browser = FocusBrowser::default();
        let _ = browser.view();
    }

    #[test]
    fn zoom_changes_in_bounded_steps_and_ignores_invalid_deltas() {
        let mut browser = FocusBrowser::default();
        browser.update(Message::Zoom(1.0));
        assert!((browser.zoom - ZOOM_STEP).abs() < f32::EPSILON);
        browser.update(Message::Zoom(f32::NAN));
        assert!((browser.zoom - ZOOM_STEP).abs() < f32::EPSILON);
        for _ in 0..100 {
            browser.update(Message::Zoom(1.0));
        }
        assert_eq!(browser.zoom, MAX_ZOOM);
        for _ in 0..100 {
            browser.update(Message::Zoom(-1.0));
        }
        assert_eq!(browser.zoom, MIN_ZOOM);
    }

    #[test]
    fn renders_focuses_with_available_and_missing_icons() {
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries("focus_tree = { focus = { id = a icon = GFX_a x = 0 y = 0 } focus = { id = b icon = GFX_missing } }").unwrap(),
        )]);
        let image = Arc::new(IconImage {
            width: 1,
            height: 1,
            pixels: vec![1, 2, 3, 255],
        });
        let icons = Icons::from([
            ("GFX_a".into(), Arc::clone(&image)),
            ("GFX_shared".into(), image),
        ]);
        let mut browser = FocusBrowser::new(focus::extract(&files), icons);
        assert!(browser.icons.contains_key("GFX_a"));
        assert!(browser.icons.contains_key("GFX_shared"));
        assert!(!browser.icons.contains_key("GFX_missing"));
        browser.update(Message::SelectTree(browser.choices[0].clone()));
        let _ = browser.view();
    }

    #[test]
    fn groups_overlapping_cells_and_keeps_unplaced_focuses_in_source_order() {
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(
                "focus_tree = {
                    focus = { id = loose_a }
                    focus = { id = first x = -2 y = 4 }
                    focus = { id = second x = -2 y = 4 visible = { always = no } }
                    focus = { id = loose_b x = 1 }
                    focus = { id = later x = 1 y = 6 prerequisite = { focus = first focus = second } prerequisite = { focus = loose_a } }
                }",
            )
            .unwrap(),
        )]);
        let trees = focus::extract(&files);
        let tree = &trees[0];
        let layout = Layout::new(tree);
        assert_eq!(layout.rows[&4][&-2], [1, 2]);
        assert_eq!(layout.rows[&6][&1], [4]);
        assert_eq!(layout.unplaced, [0, 3]);
        assert_eq!(
            prerequisites_text(&tree.focuses[4]).as_deref(),
            Some("Requires: (first OR second) AND loose_a")
        );
        assert!(prerequisites_text(&tree.focuses[1]).is_none());
        let _ = focus_layout(tree, &HashMap::new(), 1.0);
    }

    #[test]
    fn renders_extreme_coordinate_gaps_without_overflow() {
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(
                "focus_tree = {
                    focus = { id = low x = -2147483648 y = -2147483648 }
                    focus = { id = high x = 2147483647 y = 2147483647 }
                }",
            )
            .unwrap(),
        )]);
        let trees = focus::extract(&files);
        let (offsets, compressed) = column_offsets(&Layout::new(&trees[0]));
        assert!(compressed);
        assert_eq!(offsets[&i32::MIN], 0.0);
        assert_eq!(
            offsets[&i32::MAX],
            (MAX_EMPTY_CELLS + 1) as f32 * (CELL_WIDTH + COLUMN_GAP)
        );
        let _ = focus_layout(&trees[0], &HashMap::new(), 1.0);
    }

    #[test]
    fn aligns_columns_across_rows_when_large_gaps_are_compressed() {
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(
                "focus_tree = {
                    focus = { id = a x = 0 y = 0 }
                    focus = { id = b x = 100 y = 0 }
                    focus = { id = c x = 50 y = 1 }
                    focus = { id = d x = 100 y = 1 }
                }",
            )
            .unwrap(),
        )]);
        let trees = focus::extract(&files);
        let (offsets, compressed) = column_offsets(&Layout::new(&trees[0]));
        assert!(compressed);
        assert_eq!(
            offsets[&100],
            2.0 * (MAX_EMPTY_CELLS + 1) as f32 * (CELL_WIDTH + COLUMN_GAP)
        );
        let _ = focus_layout(&trees[0], &HashMap::new(), 1.0);
    }
}
