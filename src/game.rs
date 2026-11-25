use std::{
    collections::{BTreeMap, HashMap},
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
};

use crate::{
    file::read_file,
    hoi4::{
        self,
        decisions::{self, DecisionFile},
        events::{self, EventFile},
        focus::{self, FocusTree},
        icons::{self, Icons},
        ideas::{self, IdeaFile},
    },
    parser::{parse_config_entries, ConfigEntry},
};

type ParsedFile = Result<Option<Vec<ConfigEntry>>, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameProfile {
    HeartsOfIronIv,
    Stellaris,
    #[default]
    Unknown,
    Ambiguous,
}

impl GameProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::HeartsOfIronIv => "Hearts of Iron IV",
            Self::Stellaris => "Stellaris",
            Self::Unknown => "Unknown game",
            Self::Ambiguous => "Ambiguous game",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedGame {
    pub profile: GameProfile,
    pub files: BTreeMap<String, Vec<ConfigEntry>>,
    pub focus_trees: Vec<FocusTree>,
    pub decision_files: Vec<DecisionFile>,
    pub event_files: Vec<EventFile>,
    pub idea_files: Vec<IdeaFile>,
    pub hoi4_icons: Icons,
    pub errors: Vec<String>,
}

fn detect_game(root: &Path) -> GameProfile {
    let has_directories =
        |paths: &[&str]| paths.iter().all(|relative| root.join(relative).is_dir());
    let focus_game = has_directories(&["common/national_focus", "common/units/names_divisions"]);
    let space_game = has_directories(&["common/ship_sizes", "common/species_classes"]);
    match (focus_game, space_game) {
        (true, false) => GameProfile::HeartsOfIronIv,
        (false, true) => GameProfile::Stellaris,
        (true, true) => GameProfile::Ambiguous,
        (false, false) => GameProfile::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadProgress {
    Discovering { files: usize },
    Parsing { completed: usize, total: usize },
}

pub fn parse_game(
    path: &Path,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(LoadProgress) + Send,
) -> Result<Option<ParsedGame>, String> {
    log::debug!("Scanning {}", path.display());
    progress(LoadProgress::Discovering { files: 0 });
    let files = find_txt_files(path, cancelled, &mut progress).map_err(|error| {
        let message = format!("Could not scan {}: {error}", path.display());
        log::error!("{message}");
        message
    })?;
    if cancelled.load(Ordering::Relaxed) {
        return Ok(None);
    }
    let profile = detect_game(path);
    log::debug!("Detected profile: {}", profile.label());
    log::debug!("Found {} .txt files in {}", files.len(), path.display());
    let total = files.len();
    progress(LoadProgress::Parsing {
        completed: 0,
        total,
    });
    let results = parse_files(path, &files, cancelled, &mut progress);
    if cancelled.load(Ordering::Relaxed) {
        return Ok(None);
    }
    let mut parsed_files = BTreeMap::new();
    let mut errors = Vec::new();
    for (file, result) in files.iter().zip(results) {
        let file_name = file
            .strip_prefix(path)
            .unwrap_or(file)
            .to_string_lossy()
            .into_owned();
        match result {
            Ok(Some(parsed)) => {
                parsed_files.insert(file_name, parsed);
            }
            Ok(None) => {}
            Err(message) => {
                log::warn!("{message}");
                errors.push(message);
            }
        }
    }
    let mut focus_trees = if profile == GameProfile::HeartsOfIronIv {
        focus::extract(&parsed_files)
    } else {
        Vec::new()
    };
    let mut decision_files = if profile == GameProfile::HeartsOfIronIv {
        decisions::extract(&parsed_files)
    } else {
        Vec::new()
    };
    let mut event_files = if profile == GameProfile::HeartsOfIronIv {
        events::extract(&parsed_files)
    } else {
        Vec::new()
    };
    let mut idea_files = if profile == GameProfile::HeartsOfIronIv {
        ideas::extract(&parsed_files)
    } else {
        Vec::new()
    };
    for message in hoi4::localize(
        &mut focus_trees,
        &mut decision_files,
        &mut event_files,
        &mut idea_files,
        path,
        cancelled,
    ) {
        log::warn!("{message}");
        errors.push(message);
    }
    let mut wanted: HashMap<String, u32> = focus_trees
        .iter()
        .flat_map(|tree| &tree.focuses)
        .filter_map(|focus| focus.icon.clone())
        .chain(
            decision_files
                .iter()
                .flat_map(|file| &file.decisions)
                .filter_map(|decision| decision.icon.clone()),
        )
        .chain(
            idea_files
                .iter()
                .flat_map(|file| &file.ideas)
                .filter_map(|idea| idea.picture.clone()),
        )
        .map(|name| (name, icons::ICON_SIZE))
        .collect();
    for picture in event_files
        .iter()
        .flat_map(|file| &file.events)
        .filter_map(|event| event.picture.as_ref())
    {
        wanted.insert(picture.clone(), icons::EVENT_PICTURE_SIZE);
    }
    let (hoi4_icons, icon_errors) = icons::load(path, wanted, cancelled);
    for message in icon_errors {
        log::warn!("{message}");
        errors.push(message);
    }
    if cancelled.load(Ordering::Relaxed) {
        return Ok(None);
    }
    log::info!(
        "Loaded {} non-empty files from {}; {} file errors",
        parsed_files.len(),
        path.display(),
        errors.len()
    );
    Ok(Some(ParsedGame {
        profile,
        files: parsed_files,
        focus_trees,
        decision_files,
        event_files,
        idea_files,
        hoi4_icons,
        errors,
    }))
}

fn parse_files(
    _root: &Path,
    files: &[PathBuf],
    cancelled: &AtomicBool,
    progress: &mut (impl FnMut(LoadProgress) + Send),
) -> Vec<ParsedFile> {
    if files.is_empty() {
        return Vec::new();
    }
    let worker_count = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(8)
        .min(files.len());
    let next = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<ParsedFile>>> =
        Mutex::new(std::iter::repeat_with(|| None).take(files.len()).collect());
    let progress = Mutex::new(progress);

    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            scope.spawn(|| loop {
                if cancelled.load(Ordering::Relaxed) {
                    break;
                }
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(file) = files.get(index) else {
                    break;
                };
                let result = match read_file(file) {
                    Ok(contents) => parse_config_entries(&contents)
                        .map(|parsed| (!parsed.is_empty()).then_some(parsed))
                        .map_err(|error| format!("Could not parse {}: {error}", file.display())),
                    Err(error) => Err(format!("Could not read {}: {error}", file.display())),
                };
                results.lock().unwrap_or_else(|error| error.into_inner())[index] = Some(result);
                let mut progress = progress.lock().unwrap_or_else(|error| error.into_inner());
                let completed = completed.fetch_add(1, Ordering::Relaxed) + 1;
                progress(LoadProgress::Parsing {
                    completed,
                    total: files.len(),
                });
            });
        }
    });

    results
        .into_inner()
        .unwrap_or_else(|error| error.into_inner())
        .into_iter()
        .map(|result| {
            result
                .unwrap_or_else(|| Err("Loading cancelled before this file was parsed".to_string()))
        })
        .collect()
}

fn find_txt_files(
    root: &Path,
    cancelled: &AtomicBool,
    progress: &mut impl FnMut(LoadProgress),
) -> Result<Vec<PathBuf>, io::Error> {
    let mut files = Vec::new();
    let mut directories = vec![root.to_owned()];
    while let Some(directory) = directories.pop() {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        for entry in fs::read_dir(directory)? {
            if cancelled.load(Ordering::Relaxed) {
                return Ok(files);
            }
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                directories.push(path);
            } else if path.extension().is_some_and(|extension| extension == "txt")
                && !is_known_non_config_text_file(&path, root)
            {
                files.push(path);
                progress(LoadProgress::Discovering { files: files.len() });
            }
        }
    }
    files.sort();
    Ok(files)
}

fn is_known_non_config_text_file(path: &Path, root: &Path) -> bool {
    let Ok(relative_path) = path.strip_prefix(root) else {
        return false;
    };
    let known_non_config_files = [
        Path::new("licenses.txt"),
        Path::new("changelog.txt"),
        Path::new("checksum_manifest.txt"),
        Path::new("clausewitz_branch.txt"),
        Path::new("clausewitz_rev.txt"),
        Path::new("unit_test.txt"),
        Path::new("steam_appid.txt"),
        Path::new("interface/credits.txt"),
        Path::new("wiki/robots.txt"),
        Path::new("previewer_assets/previewer_filefilter.txt"),
        Path::new("pdx_launcher/motd.txt"),
        Path::new("pdx_launcher/game/motd.txt"),
        Path::new("common/HOW_TO_MAKE_NEW_SHIPS.txt"),
        Path::new("interface/reference.txt"),
        Path::new("localisation/99_README_GRAMMAR.txt"),
    ];
    if known_non_config_files.contains(&relative_path) {
        return true;
    }

    let path_components = relative_path
        .components()
        .map(|component| component.as_os_str().to_string_lossy());
    if path_components
        .clone()
        .any(|component| component.eq_ignore_ascii_case("licenses"))
    {
        return true;
    }

    let file_name = relative_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if file_name.to_ascii_lowercase().starts_with("license")
        || file_name.eq_ignore_ascii_case("OFL.txt")
    {
        return true;
    }

    relative_path.parent() == Some(Path::new("interface"))
        && file_name.to_ascii_lowercase().starts_with("credits")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_complete_distinct_signatures_and_rejects_ambiguous_folders() {
        let dir = tempdir().unwrap();
        assert_eq!(detect_game(dir.path()), GameProfile::Unknown);

        fs::create_dir_all(dir.path().join("common/national_focus")).unwrap();
        assert_eq!(detect_game(dir.path()), GameProfile::Unknown);
        fs::create_dir_all(dir.path().join("common/units/names_divisions")).unwrap();
        assert_eq!(detect_game(dir.path()), GameProfile::HeartsOfIronIv);

        fs::create_dir_all(dir.path().join("common/ship_sizes")).unwrap();
        assert_eq!(detect_game(dir.path()), GameProfile::HeartsOfIronIv);
        fs::create_dir_all(dir.path().join("common/species_classes")).unwrap();
        assert_eq!(detect_game(dir.path()), GameProfile::Ambiguous);

        let parsed = parse_game(dir.path(), &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();
        assert_eq!(parsed.profile, GameProfile::Ambiguous);
    }

    #[test]
    fn detects_second_signature_without_using_folder_name() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("common/ship_sizes")).unwrap();
        fs::create_dir_all(dir.path().join("common/species_classes")).unwrap();
        fs::write(dir.path().join("sample.txt"), "key = yes").unwrap();

        let parsed = parse_game(dir.path(), &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();
        assert_eq!(parsed.profile, GameProfile::Stellaris);
        assert!(parsed.files.contains_key("sample.txt"));
    }

    #[test]
    fn ignores_files_that_only_have_marker_directory_names() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("common/units")).unwrap();
        fs::write(dir.path().join("common/national_focus"), "").unwrap();
        fs::create_dir_all(dir.path().join("common/units/names_divisions")).unwrap();
        assert_eq!(detect_game(dir.path()), GameProfile::Unknown);
    }

    #[test]
    fn loads_focus_trees_with_localized_names() {
        let dir = tempdir().unwrap();
        let focus_dir = dir.path().join("common/national_focus");
        fs::create_dir_all(&focus_dir).unwrap();
        fs::create_dir_all(dir.path().join("common/units/names_divisions")).unwrap();
        let localization = dir.path().join("localisation/english");
        fs::create_dir_all(&localization).unwrap();
        fs::write(
            focus_dir.join("sample.txt"),
            "focus_tree = { id = first focus = { id = alpha } }",
        )
        .unwrap();
        fs::write(
            localization.join("sample.yml"),
            "l_english:\n alpha:0 \"First\"\n",
        )
        .unwrap();
        let parsed = parse_game(dir.path(), &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();
        assert_eq!(parsed.focus_trees.len(), 1);
        assert_eq!(parsed.focus_trees[0].focuses[0].name, "First");
        assert!(parsed.errors.is_empty());
    }

    #[test]
    fn loads_focus_icons_and_retains_focuses_with_missing_assets() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        for folder in [
            "common/national_focus",
            "common/units/names_divisions",
            "localisation/english",
            "interface",
            "gfx",
        ] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        fs::write(root.join("common/national_focus/sample.txt"),
            "focus_tree = { focus = { id = a icon = GFX_a x = 0 y = 0 } focus = { id = b icon = GFX_missing } }").unwrap();
        fs::write(
            root.join("localisation/english/sample.yml"),
            "l_english:\n a:0 \"First\"\n",
        )
        .unwrap();
        fs::write(
            root.join("interface/sample.gfx"),
            r#"spriteType = { name = GFX_a texturefile = "gfx/a.png" }"#,
        )
        .unwrap();
        image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]))
            .save(root.join("gfx/a.png"))
            .unwrap();
        let parsed = parse_game(root, &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();
        assert_eq!(parsed.focus_trees[0].focuses.len(), 2);
        assert_eq!(parsed.focus_trees[0].focuses[0].name, "First");
        assert_eq!(parsed.hoi4_icons["GFX_a"].pixels, [1, 2, 3, 255]);
        assert!(parsed
            .errors
            .iter()
            .any(|error| error.contains("GFX_missing")));
    }

    #[test]
    fn loads_event_localization_picture_and_nested_options() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        for folder in [
            "common/national_focus",
            "common/units/names_divisions",
            "events",
            "localisation/english",
            "interface",
            "gfx",
        ] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        fs::write(root.join("events/sample.txt"), r#"
            country_event = {
                id = demo.1 title = demo.title desc = { text = demo.desc trigger = { active = yes } }
                picture = GFX_demo
                option = { name = demo.choice trigger = { allowed = yes } country_event = { id = demo.2 } }
            }
            news_event = { id = demo.2 title = absent picture = GFX_missing }
        "#).unwrap();
        fs::write(root.join("localisation/english/sample.yml"),
            "l_english:\n demo.title:0 \"Title\"\n demo.desc:0 \"Description\"\n demo.choice:0 \"Choice\"\n").unwrap();
        fs::write(
            root.join("interface/sample.gfx"),
            r#"spriteType = { name = GFX_demo texturefile = "gfx/demo.png" }"#,
        )
        .unwrap();
        image::RgbaImage::from_pixel(400, 200, image::Rgba([1, 2, 3, 255]))
            .save(root.join("gfx/demo.png"))
            .unwrap();
        let parsed = parse_game(root, &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();
        assert_eq!(parsed.event_files.len(), 1);
        assert_eq!(parsed.event_files[0].events.len(), 2);
        let first = &parsed.event_files[0].events[0];
        assert_eq!(first.titles[0].text, "Title");
        assert_eq!(first.descriptions[0].text, "Description");
        assert!(first.descriptions[0]
            .condition
            .as_ref()
            .unwrap()
            .contains("active"));
        assert_eq!(first.options[0].names[0].text, "Choice");
        assert_eq!(
            (
                parsed.hoi4_icons["GFX_demo"].width,
                parsed.hoi4_icons["GFX_demo"].height
            ),
            (320, 160)
        );
        assert_eq!(parsed.event_files[0].events[1].titles[0].text, "absent");
        assert!(parsed
            .errors
            .iter()
            .any(|error| error.contains("GFX_missing")));
    }

    #[test]
    fn loads_ideas_with_localization_and_pictures() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        for folder in [
            "common/national_focus",
            "common/units/names_divisions",
            "common/ideas",
            "localisation/english",
            "interface",
            "gfx",
        ] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        fs::write(
            root.join("common/ideas/sample.txt"),
            "ideas = { country = { alpha = { picture = example modifier = { value = 2 } } } advisor = { beta = { picture = GFX_missing } } }",
        )
        .unwrap();
        fs::write(
            root.join("localisation/english/sample.yml"),
            "l_english:\n alpha:0 \"First\"\n alpha_desc:0 \"Details\"\n",
        )
        .unwrap();
        fs::write(
            root.join("interface/sample.gfx"),
            r#"spriteType = { name = GFX_idea_example texturefile = "gfx/a.png" }"#,
        )
        .unwrap();
        image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]))
            .save(root.join("gfx/a.png"))
            .unwrap();
        let parsed = parse_game(root, &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();
        let ideas = &parsed.idea_files[0].ideas;
        assert_eq!(ideas.len(), 2);
        assert_eq!(ideas[0].name, "First");
        assert_eq!(ideas[0].description.as_deref(), Some("Details"));
        assert!(ideas[0].script[1].to_string().contains("value = 2"));
        assert_eq!(parsed.hoi4_icons["GFX_idea_example"].pixels, [1, 2, 3, 255]);
        assert_eq!(ideas[1].name, "beta");
        assert!(ideas[1].description.is_none());
        assert!(parsed
            .errors
            .iter()
            .any(|error| error.contains("GFX_missing")));
    }

    #[test]
    fn loads_decisions_localization_and_icons_with_fallbacks() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        for folder in [
            "common/national_focus",
            "common/units/names_divisions",
            "common/decisions/categories",
            "localisation/english",
            "interface",
            "gfx",
        ] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }

        fs::write(
            root.join("common/decisions/sample.txt"),
            "group = { alpha = { icon = test cost = 5 } beta = { icon = missing } }",
        )
        .unwrap();
        fs::write(
            root.join("common/decisions/categories/meta.txt"),
            "group = { visible = { enabled = yes } }",
        )
        .unwrap();
        fs::write(
            root.join("localisation/english/sample.yml"),
            "l_english:\n alpha:0 \"First\"\n alpha_desc:0 \"Details\"\n",
        )
        .unwrap();
        fs::write(
            root.join("interface/sample.gfx"),
            r#"spriteType = { name = GFX_decision_test texturefile = "gfx/a.png" }"#,
        )
        .unwrap();
        image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]))
            .save(root.join("gfx/a.png"))
            .unwrap();

        let parsed = parse_game(root, &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();
        assert_eq!(parsed.decision_files.len(), 1);
        let decisions = &parsed.decision_files[0].decisions;
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].name, "First");
        assert!(decisions[0].tooltip.contains("Details"));
        assert!(decisions[0].tooltip.contains("cost = 5"));
        assert_eq!(
            parsed.hoi4_icons["GFX_decision_test"].pixels,
            [1, 2, 3, 255]
        );
        assert_eq!(decisions[1].name, "beta");
        assert!(parsed
            .errors
            .iter()
            .any(|error| error.contains("GFX_decision_missing")));
    }

    #[test]
    fn parse_game_keeps_files_with_matching_stems() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(first.join("shared.txt"), "first = yes").unwrap();
        fs::write(second.join("shared.txt"), "second = yes").unwrap();
        fs::write(first.join("tokens.txt"), "alpha beta").unwrap();

        let parsed = parse_game(dir.path(), &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();

        assert_eq!(parsed.files.len(), 3);
        assert!(parsed.files.contains_key("first/shared.txt"));
        assert!(parsed.files.contains_key("second/shared.txt"));
        assert_eq!(
            parsed.files["first/tokens.txt"],
            [
                ConfigEntry::Value(crate::parser::ConfigValue::Identifier("alpha".into())),
                ConfigEntry::Value(crate::parser::ConfigValue::Identifier("beta".into())),
            ]
        );
    }

    #[test]
    fn skips_known_document_text_files() {
        let dir = tempdir().unwrap();
        let interface = dir.path().join("interface");
        fs::create_dir_all(&interface).unwrap();
        let wiki = dir.path().join("wiki");
        fs::create_dir_all(&wiki).unwrap();
        let previewer_assets = dir.path().join("previewer_assets");
        fs::create_dir_all(&previewer_assets).unwrap();
        let pdx_launcher_game = dir.path().join("pdx_launcher/game");
        fs::create_dir_all(&pdx_launcher_game).unwrap();
        let common = dir.path().join("common");
        fs::create_dir_all(&common).unwrap();
        let localisation = dir.path().join("localisation");
        fs::create_dir_all(&localisation).unwrap();
        let font_license = dir.path().join("gfx/fonts/arimo");
        fs::create_dir_all(&font_license).unwrap();
        let nested_license = dir.path().join("licenses/fonts/jura");
        fs::create_dir_all(&nested_license).unwrap();
        fs::write(dir.path().join("licenses.txt"), "document text").unwrap();
        fs::write(dir.path().join("changelog.txt"), "document text").unwrap();
        fs::write(dir.path().join("checksum_manifest.txt"), "manifest text").unwrap();
        fs::write(dir.path().join("clausewitz_branch.txt"), "branch metadata").unwrap();
        fs::write(dir.path().join("clausewitz_rev.txt"), "revision metadata").unwrap();
        fs::write(dir.path().join("unit_test.txt"), "test metadata").unwrap();
        fs::write(dir.path().join("steam_appid.txt"), "application metadata").unwrap();
        fs::write(interface.join("credits.txt"), "more document text").unwrap();
        fs::write(wiki.join("robots.txt"), "crawler instructions").unwrap();
        fs::write(
            previewer_assets.join("previewer_filefilter.txt"),
            "asset filter data",
        )
        .unwrap();
        fs::write(dir.path().join("pdx_launcher/motd.txt"), "launcher message").unwrap();
        fs::write(pdx_launcher_game.join("motd.txt"), "game message").unwrap();
        fs::write(common.join("HOW_TO_MAKE_NEW_SHIPS.txt"), "documentation").unwrap();
        fs::write(
            interface.join("credits_l_simp_chinese.txt"),
            "localized credits",
        )
        .unwrap();
        fs::write(interface.join("reference.txt"), "editor config").unwrap();
        fs::write(localisation.join("99_README_GRAMMAR.txt"), "grammar guide").unwrap();
        fs::write(font_license.join("LICENSE.txt"), "font license").unwrap();
        fs::write(nested_license.join("OFL.txt"), "font license").unwrap();
        fs::write(dir.path().join("settings.txt"), "enabled = yes").unwrap();

        let parsed = parse_game(dir.path(), &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();

        assert_eq!(parsed.files.len(), 1);
        assert!(parsed.files.contains_key("settings.txt"));
        assert!(parsed.errors.is_empty());
    }

    #[test]
    fn parse_game_reports_invalid_files_and_keeps_valid_ones() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("valid.txt"), "key = yes").unwrap();
        fs::write(dir.path().join("invalid.txt"), r#"key = "invalid\q""#).unwrap();

        let parsed = parse_game(dir.path(), &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();

        assert!(parsed.files.contains_key("valid.txt"));
        assert_eq!(parsed.errors.len(), 1);
        assert!(parsed.errors[0].contains("invalid.txt"));
    }

    #[test]
    fn parse_game_reports_directory_errors() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing");

        assert!(parse_game(&missing, &AtomicBool::new(false), |_| {}).is_err());
    }

    #[test]
    fn reports_progress_and_cancels_without_returning_partial_data() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "a = yes").unwrap();
        fs::write(dir.path().join("b.txt"), "b = yes").unwrap();
        let cancelled = AtomicBool::new(false);
        let mut updates = Vec::new();
        let result = parse_game(dir.path(), &cancelled, |update| {
            if update
                == (LoadProgress::Parsing {
                    completed: 1,
                    total: 2,
                })
            {
                cancelled.store(true, Ordering::Relaxed);
            }
            updates.push(update);
        })
        .unwrap();
        assert!(result.is_none());
        assert_eq!(
            updates.first(),
            Some(&LoadProgress::Discovering { files: 0 })
        );
        assert!(updates.contains(&LoadProgress::Parsing {
            completed: 0,
            total: 2
        }));
        let parsing_progress: Vec<_> = updates
            .iter()
            .filter_map(|update| match update {
                LoadProgress::Parsing { completed, .. } => Some(*completed),
                LoadProgress::Discovering { .. } => None,
            })
            .collect();
        assert!(parsing_progress.contains(&1));
        assert!(parsing_progress.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn can_cancel_during_discovery() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "a = yes").unwrap();
        let cancelled = AtomicBool::new(false);
        let result = parse_game(dir.path(), &cancelled, |update| {
            if update == (LoadProgress::Discovering { files: 1 }) {
                cancelled.store(true, Ordering::Relaxed);
            }
        })
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parallel_parse_keeps_path_order_and_file_errors() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("b.txt"), "second = yes").unwrap();
        fs::write(dir.path().join("a.txt"), "first = yes").unwrap();
        fs::write(dir.path().join("c.txt"), r#"invalid = "bad\q""#).unwrap();
        let parsed = parse_game(dir.path(), &AtomicBool::new(false), |_| {})
            .unwrap()
            .unwrap();
        assert_eq!(
            parsed.files.keys().map(String::as_str).collect::<Vec<_>>(),
            ["a.txt", "b.txt"]
        );
        assert_eq!(parsed.errors.len(), 1);
        assert!(parsed.errors[0].contains("c.txt"));
    }

    #[test]
    fn parser_worker_count_is_bounded() {
        assert!(
            std::thread::available_parallelism()
                .map_or(1, usize::from)
                .min(8)
                <= 8
        );
    }
}
