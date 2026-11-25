use std::cell::{Ref, RefCell};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Column, Row, Space,
};
use iced::{Alignment, Element, Length, Subscription, Task};

use super::hoi4;
use crate::game::{parse_game, GameProfile, LoadProgress, ParsedGame};
use crate::parser::{ConfigEntry, ConfigValue};

const FILE_ROW_HEIGHT: f32 = 30.0;
const TREE_ROW_HEIGHT: f32 = 28.0;
const TREE_VIEW_MAX_WIDTH: f32 = 1200.0;
const TREE_CHARACTER_WIDTH: f32 = 10.0;
const ROW_OVERSCAN: usize = 3;

#[derive(Debug, Clone)]
pub enum Message {
    FilterChanged(String),
    Selected(String),
    PollLoad,
    CancelLoad,
    Collapse(String),
    Expand(String),
    CollapseAll,
    ExpandAll,
    FilesScrolled { offset: f32, height: f32 },
    TreeScrolled { offset: f32, height: f32 },
    ShowFiles,
    ShowFocuses,
    ShowDecisions,
    ShowEvents,
    ShowIdeas,
    HoI4(hoi4::Message),
    HoI4Decisions(hoi4::decisions::Message),
    HoI4Events(hoi4::events::Message),
    HoI4Ideas(hoi4::ideas::Message),
}

#[derive(Debug, Default, PartialEq, Eq)]
enum Browser {
    #[default]
    Files,
    Focuses,
    Decisions,
    Events,
    Ideas,
}

#[derive(Debug)]
struct WorkerState {
    progress: LoadProgress,
    result: Option<Result<Option<ParsedGame>, String>>,
}

#[derive(Debug)]
struct LoadJob {
    cancelled: Arc<AtomicBool>,
    state: Arc<Mutex<WorkerState>>,
}

impl Drop for LoadJob {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl LoadJob {
    fn start(path: PathBuf) -> Result<Self, String> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let state = Arc::new(Mutex::new(WorkerState {
            progress: LoadProgress::Discovering { files: 0 },
            result: None,
        }));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_state = Arc::clone(&state);
        std::thread::Builder::new()
            .name("game-loader".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(|| {
                    parse_game(&path, &worker_cancelled, |progress| {
                        worker_state
                            .lock()
                            .unwrap_or_else(|error| error.into_inner())
                            .progress = progress;
                    })
                })
                .unwrap_or_else(|_| Err("Game loading failed unexpectedly".to_string()));
                worker_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .result = Some(result);
            })
            .map_err(|error| format!("Could not start game loader: {error}"))?;
        Ok(Self { cancelled, state })
    }
}

#[derive(Debug)]
pub struct DataView {
    job: Option<LoadJob>,
    progress: LoadProgress,
    cancelled: bool,
    profile: GameProfile,
    data: BTreeMap<String, Vec<ConfigEntry>>,
    browser: Browser,
    hoi4: hoi4::FocusBrowser,
    hoi4_decisions: hoi4::decisions::DecisionBrowser,
    hoi4_events: hoi4::events::EventBrowser,
    hoi4_ideas: hoi4::ideas::IdeaBrowser,
    errors: Vec<String>,
    load_error: Option<String>,
    expanded: HashSet<String>,
    collapsed: HashSet<String>,
    expand_all: bool,
    tree_rows: RefCell<Option<TreeRowCache>>,
    search: FileSearchCache,
    selected_file: Option<String>,
    file_viewport: Viewport,
    tree_viewport: Viewport,
}

#[derive(Debug, Default)]
struct Viewport {
    offset: f32,
    height: f32,
}

#[derive(Debug, Default)]
struct FileSearchCache {
    normalized_paths: Vec<(String, String)>,
    query: String,
    matches: Vec<String>,
}

impl FileSearchCache {
    fn set_files(&mut self, files: impl Iterator<Item = String>) {
        self.normalized_paths = files.map(|path| (path.to_lowercase(), path)).collect();
        self.recompute();
    }

    fn set_query(&mut self, query: String) {
        if self.query != query {
            self.query = query;
            self.recompute();
        }
    }

    fn recompute(&mut self) {
        let query = self.query.to_lowercase();
        self.matches = self
            .normalized_paths
            .iter()
            .filter(|(normalized, _)| normalized.contains(&query))
            .map(|(_, path)| path.clone())
            .collect();
    }
}

impl DataView {
    pub fn new(path: PathBuf) -> (Self, Task<Message>) {
        let mut view = Self {
            job: None,
            progress: LoadProgress::Discovering { files: 0 },
            cancelled: false,
            profile: GameProfile::Unknown,
            data: BTreeMap::new(),
            browser: Browser::Files,
            hoi4: hoi4::FocusBrowser::default(),
            hoi4_decisions: hoi4::decisions::DecisionBrowser::default(),
            hoi4_events: hoi4::events::EventBrowser::default(),
            hoi4_ideas: hoi4::ideas::IdeaBrowser::default(),
            errors: Vec::new(),
            load_error: None,
            expanded: HashSet::new(),
            collapsed: HashSet::new(),
            expand_all: false,
            tree_rows: RefCell::new(None),
            search: FileSearchCache::default(),
            selected_file: None,
            file_viewport: Viewport {
                offset: 0.0,
                height: 600.0,
            },
            tree_viewport: Viewport {
                offset: 0.0,
                height: 600.0,
            },
        };
        match LoadJob::start(path) {
            Ok(job) => view.job = Some(job),
            Err(error) => view.load_error = Some(error),
        }
        (view, Task::none())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.job.is_some() {
            iced::time::every(Duration::from_millis(100)).map(|_| Message::PollLoad)
        } else {
            Subscription::none()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PollLoad => {
                if let Some(job) = &self.job {
                    let mut state = job.state.lock().unwrap_or_else(|error| error.into_inner());
                    self.progress = state.progress.clone();
                    let result = state.result.take();
                    drop(state);
                    if let Some(result) = result {
                        self.job = None;
                        match result {
                            Ok(Some(data)) => {
                                self.profile = data.profile;
                                self.data = data.files;
                                self.hoi4 =
                                    hoi4::FocusBrowser::new(data.focus_trees, data.hoi4_icons);
                                self.hoi4_decisions = hoi4::decisions::DecisionBrowser::new(
                                    data.decision_files,
                                    self.hoi4.icon_handles().clone(),
                                );
                                self.hoi4_events = hoi4::events::EventBrowser::new(
                                    data.event_files,
                                    self.hoi4.icon_handles().clone(),
                                );
                                self.hoi4_ideas = hoi4::ideas::IdeaBrowser::new(
                                    data.idea_files,
                                    self.hoi4.icon_handles().clone(),
                                );
                                self.browser = Browser::Files;
                                self.errors = data.errors;
                                self.invalidate_tree_rows();
                                self.search.set_files(self.data.keys().cloned());
                            }
                            Ok(None) => self.cancelled = true,
                            Err(error) => self.load_error = Some(error),
                        }
                    }
                }
            }
            Message::CancelLoad => {
                self.job = None;
                self.cancelled = true;
                log::info!("Game loading cancelled");
            }
            Message::FilterChanged(filter) => {
                self.search.set_query(filter);
                self.file_viewport.offset = 0.0;
            }
            Message::Selected(file) => {
                if self.data.contains_key(&file) {
                    self.selected_file = Some(file);
                    self.expanded.clear();
                    self.collapsed.clear();
                    self.expand_all = false;
                    self.tree_viewport.offset = 0.0;
                    self.invalidate_tree_rows();
                }
            }
            Message::Collapse(item) => {
                let changed = if self.expand_all {
                    self.collapsed.insert(item)
                } else {
                    self.expanded.remove(&item)
                };
                if changed {
                    self.invalidate_tree_rows();
                }
            }
            Message::Expand(item) => {
                let changed = if self.expand_all {
                    self.collapsed.remove(&item)
                } else {
                    self.expanded.insert(item)
                };
                if changed {
                    self.invalidate_tree_rows();
                }
            }
            Message::CollapseAll => {
                self.expand_all = false;
                self.expanded.clear();
                self.collapsed.clear();
                self.invalidate_tree_rows();
            }
            Message::ExpandAll => {
                self.expand_all = true;
                self.expanded.clear();
                self.collapsed.clear();
                self.invalidate_tree_rows();
            }
            Message::FilesScrolled { offset, height } => {
                self.file_viewport = Viewport { offset, height };
            }
            Message::TreeScrolled { offset, height } => {
                self.tree_viewport = Viewport { offset, height };
            }
            Message::ShowFiles => self.browser = Browser::Files,
            Message::ShowFocuses => {
                if self.profile == GameProfile::HeartsOfIronIv {
                    self.browser = Browser::Focuses;
                }
            }
            Message::ShowDecisions => {
                if self.profile == GameProfile::HeartsOfIronIv {
                    self.browser = Browser::Decisions;
                }
            }
            Message::ShowEvents => {
                if self.profile == GameProfile::HeartsOfIronIv {
                    self.browser = Browser::Events;
                }
            }
            Message::ShowIdeas => {
                if self.profile == GameProfile::HeartsOfIronIv {
                    self.browser = Browser::Ideas;
                }
            }
            Message::HoI4(message) => self.hoi4.update(message),
            Message::HoI4Decisions(message) => self.hoi4_decisions.update(message),
            Message::HoI4Events(message) => self.hoi4_events.update(message),
            Message::HoI4Ideas(message) => self.hoi4_ideas.update(message),
        }
        Task::none()
    }

    fn invalidate_tree_rows(&mut self) {
        *self.tree_rows.get_mut() = None;
    }

    fn tree_rows_cache(&self) -> Ref<'_, TreeRowCache> {
        let is_missing = self.tree_rows.borrow().is_none();
        if is_missing {
            let rows = self
                .selected_file
                .as_ref()
                .and_then(|file| self.data.get(file))
                .map(|entries| {
                    flatten_tree_rows(entries, self.expand_all, &self.expanded, &self.collapsed)
                })
                .unwrap_or_default();
            self.tree_rows.replace(Some(rows));
        }
        Ref::map(self.tree_rows.borrow(), |rows| {
            rows.as_ref().expect("tree row cache was just initialized")
        })
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.job.is_some() {
            let progress = match self.progress {
                LoadProgress::Discovering { files } => format!("Discovering files: {files} found"),
                LoadProgress::Parsing { completed, total } => {
                    format!("Parsing files: {completed} / {total}")
                }
            };
            return container(
                column![
                    text("Loading...").size(30),
                    text(progress),
                    button("Cancel").on_press(Message::CancelLoad)
                ]
                .align_x(Alignment::Center)
                .spacing(15),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }

        if self.cancelled || self.load_error.is_some() {
            let message = self
                .load_error
                .as_deref()
                .unwrap_or("Loading cancelled. Open a folder to try again.");
            return container(text(message)).padding(20).into();
        }

        if self.profile == GameProfile::HeartsOfIronIv {
            let content = match self.browser {
                Browser::Files => self.files_view(),
                Browser::Focuses => self.hoi4.view().map(Message::HoI4),
                Browser::Decisions => self.hoi4_decisions.view().map(Message::HoI4Decisions),
                Browser::Events => self.hoi4_events.view().map(Message::HoI4Events),
                Browser::Ideas => self.hoi4_ideas.view().map(Message::HoI4Ideas),
            };
            return column![
                row![
                    button("Files").on_press(Message::ShowFiles),
                    button("National Focus").on_press(Message::ShowFocuses),
                    button("Decisions").on_press(Message::ShowDecisions),
                    button("Events").on_press(Message::ShowEvents),
                    button("Ideas").on_press(Message::ShowIdeas),
                ]
                .spacing(10)
                .padding([0, 20]),
                content
            ]
            .height(Length::Fill)
            .into();
        }

        self.files_view()
    }

    fn files_view(&self) -> Element<'_, Message> {
        let matches = &self.search.matches;
        let file_start = window_start(
            self.file_viewport.offset,
            self.file_viewport.height,
            FILE_ROW_HEIGHT,
            matches.len(),
        );
        let file_end = window_end(
            self.file_viewport.offset,
            self.file_viewport.height,
            FILE_ROW_HEIGHT,
            matches.len(),
        );
        let mut file_rows: Column<'_, Message> = Column::new().spacing(0);
        for file in &matches[file_start..file_end] {
            file_rows = file_rows.push(
                button(text(file.clone()).wrapping(iced::widget::text::Wrapping::None))
                    .width(Length::Shrink)
                    .height(FILE_ROW_HEIGHT as u32)
                    .on_press(Message::Selected(file.clone())),
            );
        }
        let file_content = column![
            Space::new().height(file_start as f32 * FILE_ROW_HEIGHT),
            file_rows,
            Space::new().height(matches.len().saturating_sub(file_end) as f32 * FILE_ROW_HEIGHT),
        ];
        let mut file_list: Column<'_, Message> = Column::new().spacing(4);
        if matches.is_empty() {
            file_list = file_list.push(text(if self.data.is_empty() {
                "No non-empty files were loaded."
            } else {
                "No matching files."
            }));
        }
        let file_content: Element<'_, Message> = if matches.is_empty() {
            file_list.into()
        } else {
            file_content.into()
        };
        let sidebar = column![
            text(format!("Game: {}", self.profile.label())),
            text_input("Filter file paths", &self.search.query).on_input(Message::FilterChanged),
            scrollable(file_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .direction(scrollable::Direction::Both {
                    vertical: scrollable::Scrollbar::default(),
                    horizontal: scrollable::Scrollbar::default(),
                })
                .on_scroll(|viewport| Message::FilesScrolled {
                    offset: viewport.absolute_offset().y,
                    height: viewport.bounds().height,
                }),
        ]
        .width(300)
        .height(Length::Fill)
        .spacing(10);

        let mut details = Column::new().spacing(10);
        if !self.errors.is_empty() {
            let mut error_list = Column::new().spacing(5);
            for error in &self.errors {
                error_list = error_list.push(text(error));
            }
            details = details
                .push(text(format!(
                    "Some files could not be loaded ({}):",
                    self.errors.len()
                )))
                .push(scrollable(error_list).height(120));
        }
        if let Some(file) = &self.selected_file {
            let cache = self.tree_rows_cache();
            let rows = visible_tree_rows(
                &self.data[file],
                &cache,
                self.tree_viewport.offset,
                self.tree_viewport.height,
            );
            let mut tree: Column<'_, Message> = Column::new().spacing(0).width(Length::Fill);
            for row_data in &rows.rows {
                tree = tree.push(create_tree_row(row_data));
            }
            let tree_content = column![
                Space::new().height(rows.start as f32 * TREE_ROW_HEIGHT),
                tree,
                Space::new().height(rows.total.saturating_sub(rows.end) as f32 * TREE_ROW_HEIGHT),
            ]
            .width(Length::Fixed(rows.content_width));
            details = details
                .push(text(file))
                .push(
                    row![
                        button("Expand all").on_press(Message::ExpandAll),
                        button("Collapse all").on_press(Message::CollapseAll),
                    ]
                    .spacing(10),
                )
                .push(
                    container(
                        scrollable(tree_content)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .direction(scrollable::Direction::Both {
                                vertical: scrollable::Scrollbar::default(),
                                horizontal: scrollable::Scrollbar::default(),
                            })
                            .on_scroll(|viewport| Message::TreeScrolled {
                                offset: viewport.absolute_offset().y,
                                height: viewport.bounds().height,
                            }),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .max_width(TREE_VIEW_MAX_WIDTH),
                );
        } else {
            details = details.push(text("Select a file to view its contents"));
        }
        container(row![sidebar, details.width(Length::Fill).height(Length::Fill)].spacing(20))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }
}

fn window_start(offset: f32, _height: f32, row_height: f32, count: usize) -> usize {
    ((offset.max(0.0) / row_height) as usize)
        .saturating_sub(ROW_OVERSCAN)
        .min(count)
}

fn window_end(offset: f32, height: f32, row_height: f32, count: usize) -> usize {
    (((offset.max(0.0) + height.max(row_height)) / row_height).ceil() as usize)
        .saturating_add(ROW_OVERSCAN)
        .min(count)
}

#[derive(Debug, Default)]
struct TreeRowCache {
    rows: Vec<TreeRowIndex>,
    content_width: f32,
}

#[derive(Debug, Clone)]
struct TreeRowIndex {
    parent: Option<usize>,
    child_index: usize,
    depth: usize,
    expandable: bool,
    open: bool,
}

#[derive(Debug, Clone)]
struct TreeRow {
    id: String,
    depth: usize,
    label: String,
    expandable: bool,
    open: bool,
}

struct TreeRows {
    rows: Vec<TreeRow>,
    start: usize,
    end: usize,
    total: usize,
    content_width: f32,
}

fn visible_tree_rows(
    entries: &[ConfigEntry],
    cache: &TreeRowCache,
    offset: f32,
    height: f32,
) -> TreeRows {
    let start = window_start(offset, height, TREE_ROW_HEIGHT, cache.rows.len());
    let end = window_end(offset, height, TREE_ROW_HEIGHT, cache.rows.len());
    TreeRows {
        rows: (start..end)
            .map(|index| tree_row(entries, cache, index))
            .collect(),
        start,
        end,
        total: cache.rows.len(),
        content_width: cache.content_width,
    }
}

fn tree_row(entries: &[ConfigEntry], cache: &TreeRowCache, index: usize) -> TreeRow {
    let mut path = Vec::new();
    let mut current = Some(index);
    while let Some(row_index) = current {
        let row = &cache.rows[row_index];
        path.push(row.child_index);
        current = row.parent;
    }
    path.reverse();

    let row_index = &cache.rows[index];
    let mut identifier = String::new();
    let mut sign = String::new();
    let mut value = match &entries[path[0]] {
        ConfigEntry::Pair(pair) => {
            identifier.clone_from(&pair.identifier);
            sign.clone_from(&pair.sign);
            &pair.value
        }
        ConfigEntry::Value(value) => value,
    };

    for &child_index in &path[1..] {
        match value {
            ConfigValue::Object(children) | ConfigValue::Template(_, children) => {
                match &children[child_index] {
                    ConfigEntry::Pair(pair) => {
                        identifier.clone_from(&pair.identifier);
                        sign.clone_from(&pair.sign);
                        value = &pair.value;
                    }
                    ConfigEntry::Value(child) => {
                        identifier.clear();
                        sign.clear();
                        value = child;
                    }
                }
            }
            ConfigValue::Array(children) | ConfigValue::Named(_, children) => {
                identifier = format!("[{child_index}]");
                sign.clear();
                value = &children[child_index];
            }
            _ => unreachable!("tree row path must point to a child collection"),
        }
    }

    let display = match value {
        ConfigValue::Object(_) | ConfigValue::Array(_) => "{ ... }".to_string(),
        ConfigValue::Template(name, _) => format!("[[{name}] ... ]"),
        ConfigValue::Empty => String::new(),
        ConfigValue::Named(name, _) => format!("{name} {{ ... }}"),
        _ => value.to_string(),
    };
    let label = if identifier.is_empty() {
        display
    } else if matches!(value, ConfigValue::Empty) {
        format!("{identifier} {sign}")
    } else if sign.is_empty() {
        format!("{identifier} {display}")
    } else {
        format!("{identifier} {sign} {display}")
    };
    TreeRow {
        id: path
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("/"),
        depth: row_index.depth,
        label,
        expandable: row_index.expandable,
        open: row_index.open,
    }
}

fn flatten_tree_rows(
    entries: &[ConfigEntry],
    expand_all: bool,
    expanded: &HashSet<String>,
    collapsed: &HashSet<String>,
) -> TreeRowCache {
    struct Collector<'a> {
        expand_all: bool,
        expanded: &'a HashSet<String>,
        collapsed: &'a HashSet<String>,
        content_width: f32,
        rows: Vec<TreeRowIndex>,
    }

    impl Collector<'_> {
        fn visit(
            &mut self,
            path: &mut Vec<usize>,
            parent: Option<usize>,
            child_index: usize,
            identifier: &str,
            sign: &str,
            value: &ConfigValue,
        ) {
            path.push(child_index);
            let depth = path.len() - 1;
            let id = path
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join("/");
            let children = match value {
                ConfigValue::Object(entries) | ConfigValue::Template(_, entries) => {
                    !entries.is_empty()
                }
                ConfigValue::Array(values) | ConfigValue::Named(_, values) => !values.is_empty(),
                _ => false,
            };
            let display_width = match value {
                ConfigValue::Object(_) | ConfigValue::Array(_) => 6,
                ConfigValue::Named(name, _) => name.chars().count() + 7,
                ConfigValue::Template(name, _) => name.chars().count() + 7,
                ConfigValue::Empty => 0,
                ConfigValue::Expression(expression) => expression.chars().count(),
                ConfigValue::String(value) => {
                    value.chars().count()
                        + value.chars().filter(|ch| matches!(ch, '"' | '\\')).count()
                        + 2
                }
                ConfigValue::Number(value) => formatted_char_count(value),
                ConfigValue::Identifier(value) | ConfigValue::Date(value) => value.chars().count(),
            };
            let label_width = identifier.chars().count()
                + usize::from(!sign.is_empty()) * (sign.chars().count() + 2)
                + 1
                + display_width;
            self.content_width = self
                .content_width
                .max(depth as f32 * 25.0 + 30.0 + label_width as f32 * TREE_CHARACTER_WIDTH);
            let open = self.expand_all && !self.collapsed.contains(&id)
                || !self.expand_all && self.expanded.contains(&id);
            let row_index = self.rows.len();
            self.rows.push(TreeRowIndex {
                parent,
                child_index,
                depth,
                expandable: children,
                open,
            });
            if !open || !children {
                path.pop();
                return;
            }
            match value {
                ConfigValue::Object(entries) | ConfigValue::Template(_, entries) => {
                    for (child_index, entry) in entries.iter().enumerate() {
                        match entry {
                            crate::parser::ConfigEntry::Pair(pair) => self.visit(
                                path,
                                Some(row_index),
                                child_index,
                                &pair.identifier,
                                &pair.sign,
                                &pair.value,
                            ),
                            crate::parser::ConfigEntry::Value(value) => {
                                self.visit(path, Some(row_index), child_index, "", "", value)
                            }
                        }
                    }
                }
                ConfigValue::Array(values) | ConfigValue::Named(_, values) => {
                    for (child_index, child) in values.iter().enumerate() {
                        self.visit(
                            path,
                            Some(row_index),
                            child_index,
                            &format!("[{child_index}]"),
                            "",
                            child,
                        );
                    }
                }
                _ => {}
            }
            path.pop();
        }
    }

    let mut collector = Collector {
        expand_all,
        expanded,
        collapsed,
        content_width: 0.0,
        rows: Vec::new(),
    };
    let mut path = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        match entry {
            ConfigEntry::Pair(pair) => collector.visit(
                &mut path,
                None,
                index,
                &pair.identifier,
                &pair.sign,
                &pair.value,
            ),
            ConfigEntry::Value(value) => collector.visit(&mut path, None, index, "", "", value),
        }
    }
    TreeRowCache {
        rows: collector.rows,
        content_width: collector.content_width.max(1.0),
    }
}

fn formatted_char_count(value: &impl std::fmt::Display) -> usize {
    struct CharCounter(usize);

    impl std::fmt::Write for CharCounter {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            self.0 += text.chars().count();
            Ok(())
        }
    }

    let mut counter = CharCounter(0);
    let _ = std::fmt::write(&mut counter, format_args!("{value}"));
    counter.0
}

fn create_tree_row(value: &TreeRow) -> Element<'static, Message> {
    let mut row = Row::new()
        .width(Length::Fill)
        .push(Space::new().width((value.depth as f32) * 25.0));
    if value.expandable {
        let action = if value.open {
            Message::Collapse(value.id.clone())
        } else {
            Message::Expand(value.id.clone())
        };
        row = row.push(
            button(if value.open { "-" } else { "+" })
                .width(20)
                .height(TREE_ROW_HEIGHT as u32)
                .on_press(action),
        );
    } else {
        row = row.push(Space::new().width(20));
    }
    row.push(Space::new().width(10))
        .push(text(value.label.clone()).wrapping(iced::widget::text::Wrapping::None))
        .height(TREE_ROW_HEIGHT)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Instant;
    use tempfile::tempdir;

    fn rows_for(
        entries: &[ConfigEntry],
        expand_all: bool,
        expanded: &HashSet<String>,
        collapsed: &HashSet<String>,
        offset: f32,
        height: f32,
    ) -> TreeRows {
        let cache = flatten_tree_rows(entries, expand_all, expanded, collapsed);
        visible_tree_rows(entries, &cache, offset, height)
    }

    #[test]
    fn preserves_duplicate_order_and_expands_nested_values() {
        let pairs = crate::parser::parse_config_entries(
            "z = 1 a = { 2 { child = yes } } z = rgb { 1 2 3 }",
        )
        .unwrap();
        let rows = rows_for(&pairs, true, &HashSet::new(), &HashSet::new(), 0.0, 1000.0);
        assert_eq!(rows.total, 9);
        assert_eq!(
            rows.rows
                .iter()
                .take(3)
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            ["z = 1", "a = { ... }", "[0] 2"]
        );
        assert!(rows.rows.iter().any(|row| row.label == "z = rgb { ... }"));
        assert!(rows.rows.iter().any(|row| row.label == "[2] 3"));
    }

    #[test]
    fn filters_relative_paths_case_insensitively() {
        let mut cache = FileSearchCache::default();
        cache.set_files(
            ["common/Alpha.txt", "events/alpha.txt"]
                .map(str::to_owned)
                .into_iter(),
        );
        assert_eq!(cache.matches.len(), 2);
        cache.set_query("events/".into());
        assert_eq!(cache.matches, ["events/alpha.txt"]);
        let normalized = cache.normalized_paths[0].0.clone();
        cache.set_query("ALPHA".into());
        assert_eq!(cache.matches.len(), 2);
        assert_eq!(cache.normalized_paths[0].0, normalized);
        cache.set_query("missing".into());
        assert!(cache.matches.is_empty());
        cache.set_files(["other/file.txt".to_owned()].into_iter());
        assert!(cache.matches.is_empty());
    }

    #[test]
    fn selecting_file_preserves_source_order_and_duplicates() {
        let dir = tempdir().unwrap();
        let (mut view, _) = DataView::new(dir.path().to_owned());
        let _ = view.update(Message::CancelLoad);
        view.data.insert(
            "events/a.txt".into(),
            crate::parser::parse_config_entries("z = 1 a = 2 z = 3").unwrap(),
        );
        let _ = view.update(Message::FilterChanged("EVENTS".into()));
        let _ = view.update(Message::Selected("events/a.txt".into()));
        assert_eq!(view.selected_file.as_deref(), Some("events/a.txt"));
        assert_eq!(
            rows_for(
                &view.data["events/a.txt"],
                false,
                &view.expanded,
                &view.collapsed,
                0.0,
                1000.0,
            )
            .rows
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
            ["z = 1", "a = 2", "z = 3"]
        );
    }

    #[test]
    fn caches_flattened_rows_until_expansion_changes() {
        let dir = tempdir().unwrap();
        let (mut view, _) = DataView::new(dir.path().to_owned());
        let _ = view.update(Message::CancelLoad);
        view.cancelled = false;
        view.data.insert(
            "sample.txt".into(),
            crate::parser::parse_config_entries("outer = { child = yes }").unwrap(),
        );
        let _ = view.update(Message::Selected("sample.txt".into()));

        assert!(view.tree_rows.borrow().is_none());
        let _ = view.view();
        assert_eq!(view.tree_rows_cache().rows.len(), 1);
        assert!(view.tree_rows.borrow().is_some());

        let _ = view.update(Message::TreeScrolled {
            offset: TREE_ROW_HEIGHT,
            height: 100.0,
        });
        let _ = view.view();
        assert!(view.tree_rows.borrow().is_some());

        let _ = view.update(Message::Expand("0".into()));
        assert!(view.tree_rows.borrow().is_none());
        let _ = view.view();
        assert_eq!(view.tree_rows_cache().rows.len(), 2);
    }

    #[test]
    fn virtual_windows_cover_top_middle_bottom_and_resize() {
        let count = 10_000;
        assert_eq!(window_start(0.0, 280.0, TREE_ROW_HEIGHT, count), 0);
        assert_eq!(window_end(0.0, 280.0, TREE_ROW_HEIGHT, count), 13);
        assert_eq!(window_start(1400.0, 280.0, TREE_ROW_HEIGHT, count), 47);
        assert_eq!(window_end(1400.0, 280.0, TREE_ROW_HEIGHT, count), 63);
        assert_eq!(
            window_start(1_000_000.0, 280.0, TREE_ROW_HEIGHT, count),
            count
        );
        assert_eq!(
            window_end(1_000_000.0, 280.0, TREE_ROW_HEIGHT, count),
            count
        );
        assert_eq!(window_end(0.0, 560.0, TREE_ROW_HEIGHT, count), 23);
    }

    #[test]
    fn tree_materializes_only_the_viewport_window() {
        let input = (0..1000)
            .map(|index| format!("key{index} = yes"))
            .collect::<Vec<_>>()
            .join("\n");
        let pairs = crate::parser::parse_config_entries(&input).unwrap();
        let rows = rows_for(
            &pairs,
            false,
            &HashSet::new(),
            &HashSet::new(),
            10_000.0,
            280.0,
        );
        assert_eq!(rows.total, 1000);
        assert!(rows.rows.len() < 20);
        assert_eq!(
            rows.start,
            window_start(10_000.0, 280.0, TREE_ROW_HEIGHT, 1000)
        );
    }

    #[test]
    fn tree_content_width_stays_stable_across_virtual_windows() {
        let mut lines = (0..20)
            .map(|index| format!("key{index} = yes"))
            .collect::<Vec<_>>();
        lines.push(format!("long = \"{}\"", "x".repeat(160)));
        let pairs = crate::parser::parse_config_entries(&lines.join("\n")).unwrap();

        let top = rows_for(&pairs, false, &HashSet::new(), &HashSet::new(), 0.0, 100.0);
        let bottom = rows_for(
            &pairs,
            false,
            &HashSet::new(),
            &HashSet::new(),
            500.0,
            100.0,
        );

        assert!(top.content_width > TREE_VIEW_MAX_WIDTH);
        assert_eq!(top.content_width, bottom.content_width);
        assert!(!top.rows.iter().any(|row| row.label.starts_with("long")));
    }

    #[test]
    fn displays_top_level_values_without_keys() {
        let entries = crate::parser::parse_config_entries("first_token second_token").unwrap();
        let rows = rows_for(
            &entries,
            false,
            &HashSet::new(),
            &HashSet::new(),
            0.0,
            100.0,
        );
        assert_eq!(
            rows.rows
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            ["first_token", "second_token"]
        );
    }

    #[test]
    fn displays_and_expands_template_blocks() {
        let entries =
            crate::parser::parse_config_entries("conditional = [[!FLAG] active = yes ]").unwrap();
        let rows = rows_for(&entries, true, &HashSet::new(), &HashSet::new(), 0.0, 100.0);
        assert_eq!(rows.rows[0].label, "conditional = [[!FLAG] ... ]");
        assert_eq!(rows.rows[1].label, "active = yes");
    }

    #[test]
    fn worker_delivers_results_and_errors() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("good.txt"), "a = yes").unwrap();
        fs::write(dir.path().join("bad.txt"), r#"a = "bad\q""#).unwrap();
        let (mut view, _) = DataView::new(dir.path().to_owned());
        let deadline = Instant::now() + Duration::from_secs(5);
        while view.job.is_some() && Instant::now() < deadline {
            let _ = view.update(Message::PollLoad);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(view.job.is_none(), "worker did not finish");
        assert!(view.data.contains_key("good.txt"));
        assert_eq!(view.errors.len(), 1);
        assert!(view.load_error.is_none());
    }

    #[test]
    fn worker_passes_detected_profile_to_view() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("common/national_focus")).unwrap();
        fs::create_dir_all(dir.path().join("common/units/names_divisions")).unwrap();
        fs::write(dir.path().join("sample.txt"), "key = yes").unwrap();
        let (mut view, _) = DataView::new(dir.path().to_owned());
        let deadline = Instant::now() + Duration::from_secs(5);
        while view.job.is_some() && Instant::now() < deadline {
            let _ = view.update(Message::PollLoad);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(view.job.is_none(), "worker did not finish");
        assert_eq!(view.profile, GameProfile::HeartsOfIronIv);
        assert!(view.data.contains_key("sample.txt"));
        let _ = view.view();
    }

    #[test]
    fn focus_view_is_profile_gated_and_preserves_file_selection() {
        let dir = tempdir().unwrap();
        let (mut view, _) = DataView::new(dir.path().to_owned());
        let _ = view.update(Message::CancelLoad);
        view.cancelled = false;
        view.data.insert(
            "sample.txt".into(),
            crate::parser::parse_config_entries("key = yes").unwrap(),
        );
        let _ = view.update(Message::Selected("sample.txt".into()));
        let _ = view.update(Message::ShowFocuses);
        assert_eq!(view.browser, Browser::Files);
        view.profile = GameProfile::HeartsOfIronIv;
        let _ = view.update(Message::ShowFocuses);
        assert_eq!(view.browser, Browser::Focuses);
        let _ = view.view();
        let _ = view.update(Message::ShowFiles);
        assert_eq!(view.selected_file.as_deref(), Some("sample.txt"));
    }

    #[test]
    fn decisions_view_is_profile_gated() {
        let dir = tempdir().unwrap();
        let (mut view, _) = DataView::new(dir.path().to_owned());
        let _ = view.update(Message::CancelLoad);
        view.cancelled = false;
        let _ = view.update(Message::ShowDecisions);
        assert_eq!(view.browser, Browser::Files);
    }

    #[test]
    fn events_view_is_profile_gated_and_has_no_error_panel() {
        let dir = tempdir().unwrap();
        let (mut view, _) = DataView::new(dir.path().to_owned());
        let _ = view.update(Message::CancelLoad);
        view.cancelled = false;
        let _ = view.update(Message::ShowEvents);
        assert_eq!(view.browser, Browser::Files);
        view.profile = GameProfile::HeartsOfIronIv;
        let _ = view.update(Message::ShowEvents);
        assert_eq!(view.browser, Browser::Events);
        let _ = view.view();
    }

    #[test]
    fn ideas_view_is_profile_gated() {
        let dir = tempdir().unwrap();
        let (mut view, _) = DataView::new(dir.path().to_owned());
        let _ = view.update(Message::CancelLoad);
        view.cancelled = false;
        let _ = view.update(Message::ShowIdeas);
        assert_eq!(view.browser, Browser::Files);
        view.profile = GameProfile::HeartsOfIronIv;
        let _ = view.update(Message::ShowIdeas);
        assert_eq!(view.browser, Browser::Ideas);
        let _ = view.view();
        let _ = view.update(Message::ShowFiles);
        assert_eq!(view.browser, Browser::Files);
    }

    #[test]
    fn file_errors_are_visible_only_in_the_files_view() {
        use iced::advanced::graphics::core::renderer::Headless;
        use iced::advanced::{layout, mouse, renderer, widget::Tree, Layout};
        use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

        fn render(view: &DataView) -> Vec<u8> {
            let mut element = view.view();
            let mut renderer = iced::futures::executor::block_on(<Renderer as Headless>::new(
                iced::Font::default(),
                16.into(),
                Some("tiny-skia"),
            ))
            .expect("software renderer");
            let mut tree = Tree::new(element.as_widget());
            let size = Size::new(800.0, 600.0);
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, size),
            );
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &Theme::Dark,
                &renderer::Style {
                    text_color: Color::WHITE,
                },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::new(Point::ORIGIN, size),
            );
            renderer.screenshot(Size::new(800, 600), 1.0, Color::BLACK)
        }

        let dir = tempdir().unwrap();
        let (mut view, _) = DataView::new(dir.path().to_owned());
        let _ = view.update(Message::CancelLoad);
        view.cancelled = false;
        view.profile = GameProfile::HeartsOfIronIv;
        for browser in [
            Browser::Focuses,
            Browser::Decisions,
            Browser::Events,
            Browser::Ideas,
            Browser::Files,
        ] {
            view.browser = browser;
            view.errors.clear();
            let without_errors = render(&view);
            view.errors.push("Synthetic file error".into());
            let with_errors = render(&view);
            if view.browser == Browser::Files {
                assert_ne!(without_errors, with_errors);
            } else {
                assert_eq!(without_errors, with_errors);
            }
        }
        view.profile = GameProfile::HeartsOfIronIv;
        let _ = view.update(Message::ShowDecisions);
        assert_eq!(view.browser, Browser::Decisions);
        let _ = view.view();
        let _ = view.update(Message::ShowFiles);
        assert_eq!(view.browser, Browser::Files);
        let _ = view.view();
    }

    #[test]
    fn cancelling_discards_active_job() {
        let dir = tempdir().unwrap();
        let (mut view, _) = DataView::new(dir.path().to_owned());
        let cancelled = Arc::clone(&view.job.as_ref().unwrap().cancelled);
        let _ = view.update(Message::CancelLoad);
        assert!(cancelled.load(Ordering::Relaxed));
        assert!(view.job.is_none());
        assert!(view.cancelled);
    }

    #[test]
    fn worker_reports_scan_failure() {
        let dir = tempdir().unwrap();
        let (mut view, _) = DataView::new(dir.path().join("missing"));
        let deadline = Instant::now() + Duration::from_secs(5);
        while view.job.is_some() && Instant::now() < deadline {
            let _ = view.update(Message::PollLoad);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(view.job.is_none(), "worker did not finish");
        assert!(view
            .load_error
            .as_deref()
            .unwrap()
            .contains("Could not scan"));
    }
}
