use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use image::{DynamicImage, ImageReader};

use crate::parser::{parse_config_entries, ConfigEntry, ConfigValue};

const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_DIMENSION: u32 = 4096;
pub const ICON_SIZE: u32 = 96;
pub const EVENT_PICTURE_SIZE: u32 = 320;

#[derive(Debug)]
pub struct IconImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub type Icons = HashMap<String, Arc<IconImage>>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Sprite {
    asset_root: PathBuf,
    texture: PathBuf,
    frames: u32,
}

fn text(value: &ConfigValue) -> Option<&str> {
    match value {
        ConfigValue::String(value) | ConfigValue::Identifier(value) => Some(value),
        _ => None,
    }
}

fn safe_texture_path(value: &str) -> Option<PathBuf> {
    let normalized = value.replace('\\', "/");
    if normalized.contains(':') {
        return None;
    }
    let path = Path::new(&normalized);
    if path
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        Some(path.to_owned()).filter(|path| !path.as_os_str().is_empty())
    } else {
        None
    }
}

fn collect_sprites(
    entries: &[ConfigEntry],
    wanted: &HashSet<String>,
    sprites: &mut BTreeMap<String, Vec<Sprite>>,
    errors: &mut Vec<String>,
    asset_root: &Path,
) {
    for entry in entries {
        let ConfigEntry::Pair(pair) = entry else {
            continue;
        };
        let ConfigValue::Object(fields) = &pair.value else {
            continue;
        };
        if pair.identifier.eq_ignore_ascii_case("spriteType") {
            let mut name = None;
            let mut texture = None;
            let mut frames = 1;
            for entry in fields {
                if let ConfigEntry::Pair(field) = entry {
                    if field.identifier.eq_ignore_ascii_case("name") {
                        name = text(&field.value);
                    } else if field.identifier.eq_ignore_ascii_case("texturefile") {
                        texture = text(&field.value);
                    } else if field.identifier.eq_ignore_ascii_case("noOfFrames") {
                        frames = match field.value {
                            ConfigValue::Number(n)
                                if n >= 1.0 && n <= u32::MAX as f64 && n.fract() == 0.0 =>
                            {
                                n as u32
                            }
                            _ => 0,
                        };
                    }
                }
            }
            if let Some(name) = name.filter(|name| wanted.contains(*name)) {
                if let Some(texture) = texture.and_then(safe_texture_path).filter(|_| frames > 0) {
                    let sprite = Sprite {
                        asset_root: asset_root.to_owned(),
                        texture,
                        frames,
                    };
                    let candidates = sprites.entry(name.to_owned()).or_default();
                    if !candidates.contains(&sprite) {
                        if candidates
                            .iter()
                            .any(|other| other.asset_root == sprite.asset_root)
                        {
                            errors.push(format!("Conflicting sprite definitions for {name}"));
                        }
                        // Roots are visited base first, so a DLC override goes ahead of the
                        // base definition while same-root definitions keep file order.
                        let at = candidates
                            .iter()
                            .take_while(|other| other.asset_root == sprite.asset_root)
                            .count();
                        candidates.insert(at, sprite);
                    }
                } else {
                    errors.push(format!("Invalid texture definition for sprite {name}"));
                }
            }
        } else {
            collect_sprites(fields, wanted, sprites, errors, asset_root);
        }
    }
}

fn mentions_wanted_sprite(contents: &str, wanted: &HashSet<String>) -> bool {
    contents
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '{' | '}' | '=' | '#'))
        .any(|token| wanted.contains(token))
        || wanted.iter().any(|name| {
            name.chars()
                .any(|c| c.is_whitespace() || matches!(c, '"' | '{' | '}' | '=' | '#'))
                && contents.contains(name)
        })
}

fn sprite_files(
    root: &Path,
    cancelled: &AtomicBool,
    errors: &mut Vec<String>,
) -> Vec<(PathBuf, PathBuf)> {
    let mut files = Vec::new();
    let mut roots = vec![PathBuf::new()];
    match fs::read_dir(root.join("dlc")) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => match entry.file_type() {
                        Ok(kind) if kind.is_dir() => {
                            let relative = PathBuf::from("dlc").join(entry.file_name());
                            if fs::symlink_metadata(root.join(&relative).join("interface"))
                                .is_ok_and(|metadata| metadata.file_type().is_dir())
                            {
                                roots.push(relative);
                            }
                        }
                        Ok(_) => {}
                        Err(error) => {
                            errors.push(format!("Could not inspect DLC directory: {error}"))
                        }
                    },
                    Err(error) => errors.push(format!("Could not read DLC directory: {error}")),
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => errors.push(format!("Could not read DLC directory: {error}")),
    }
    roots[1..].sort();
    for asset_root in roots {
        let mut directories = vec![root.join(&asset_root).join("interface")];
        let mut root_files = Vec::new();
        while let Some(directory) = directories.pop() {
            if cancelled.load(Ordering::Relaxed) {
                break;
            }
            match fs::read_dir(&directory) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => match entry.file_type() {
                                Ok(kind) if kind.is_dir() => directories.push(entry.path()),
                                Ok(kind)
                                    if kind.is_file()
                                        && entry
                                            .path()
                                            .extension()
                                            .is_some_and(|ext| ext.eq_ignore_ascii_case("gfx")) =>
                                {
                                    root_files.push(entry.path());
                                }
                                Ok(_) => {}
                                Err(error) => errors.push(format!(
                                    "Could not inspect sprite file {}: {error}",
                                    entry.path().display()
                                )),
                            },
                            Err(error) => errors.push(format!(
                                "Could not read sprite directory {}: {error}",
                                directory.display()
                            )),
                        }
                    }
                }
                Err(error) => errors.push(format!(
                    "Could not read sprite directory {}: {error}",
                    directory.display()
                )),
            }
        }
        root_files.sort();
        files.extend(
            root_files
                .into_iter()
                .map(|path| (path, asset_root.clone())),
        );
    }
    files
}

fn decode_texture(root: &Path, asset_root: &Path, relative: &Path) -> Result<DynamicImage, String> {
    let game_root = root.canonicalize().map_err(|error| error.to_string())?;
    let root = game_root
        .join(asset_root)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !root.starts_with(&game_root) {
        return Err("asset root resolves outside the game folder".into());
    }
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !path.starts_with(&root) {
        return Err("texture resolves outside the game folder".into());
    }
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err("texture is not a regular file or is too large".into());
    }
    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("dds"))
    {
        let dds = image_dds::ddsfile::Dds::read(&mut Cursor::new(bytes))
            .map_err(|error| error.to_string())?;
        if dds.get_width() == 0
            || dds.get_height() == 0
            || dds.get_width() > MAX_DIMENSION
            || dds.get_height() > MAX_DIMENSION
            || dds.get_depth() > 1
        {
            return Err("unsupported texture dimensions".into());
        }
        if dds.get_d3d_format() == Some(image_dds::ddsfile::D3DFormat::X1R5G5B5) {
            let data = &dds.data;
            let pixels = (dds.get_width() as usize) * (dds.get_height() as usize);
            let bytes = data.get(..pixels * 2).ok_or("truncated RGB555 texture")?;
            let mut rgba = Vec::with_capacity(pixels * 4);
            for pixel in bytes.as_chunks::<2>().0 {
                let value = u16::from_le_bytes([pixel[0], pixel[1]]);
                let expand = |v: u16| ((v << 3) | (v >> 2)) as u8;
                rgba.extend_from_slice(&[
                    expand((value >> 10) & 31),
                    expand((value >> 5) & 31),
                    expand(value & 31),
                    255,
                ]);
            }
            return image::RgbaImage::from_raw(dds.get_width(), dds.get_height(), rgba)
                .map(DynamicImage::ImageRgba8)
                .ok_or_else(|| "invalid RGB555 texture".into());
        }
        let surface = image_dds::SurfaceRgba8::decode_layers_mipmaps_dds(&dds, 0..1, 0..1)
            .map_err(|error| error.to_string())?;
        let image = surface.into_image().map_err(|error| error.to_string())?;
        Ok(DynamicImage::ImageRgba8(image))
    } else {
        let format = image::ImageFormat::from_path(&path).map_err(|error| error.to_string())?;
        let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_DIMENSION);
        limits.max_image_height = Some(MAX_DIMENSION);
        limits.max_alloc = Some(MAX_FILE_BYTES);
        reader.limits(limits);
        reader.decode().map_err(|error| error.to_string())
    }
}

pub fn load(
    root: &Path,
    sizes: HashMap<String, u32>,
    optional: &HashSet<String>,
    cancelled: &AtomicBool,
) -> (Icons, Vec<String>) {
    let wanted: HashSet<String> = sizes.keys().cloned().collect();
    if wanted.is_empty() {
        return (Icons::new(), Vec::new());
    }
    let mut errors = Vec::new();
    let mut sprites = BTreeMap::new();
    for (path, asset_root) in sprite_files(root, cancelled, &mut errors) {
        if cancelled.load(Ordering::Relaxed) {
            return (Icons::new(), errors);
        }
        match fs::read_to_string(&path) {
            Ok(contents) if !mentions_wanted_sprite(&contents, &wanted) => continue,
            Ok(contents) => match parse_config_entries(&contents) {
                Ok(entries) => {
                    collect_sprites(&entries, &wanted, &mut sprites, &mut errors, &asset_root)
                }
                Err(error) => errors.push(format!(
                    "Could not parse sprite definitions {}: {error}",
                    path.display()
                )),
            },
            Err(error) => errors.push(format!(
                "Could not read sprite definitions {}: {error}",
                path.display()
            )),
        }
    }
    let mut texture_cache: Option<((PathBuf, PathBuf), Result<DynamicImage, String>)> = None;
    let mut image_cache = HashMap::<(PathBuf, PathBuf, u32, u32), Arc<IconImage>>::new();
    let mut icons = Icons::new();
    let mut names: Vec<_> = wanted.into_iter().collect();
    names.sort_by(|a, b| {
        sprites
            .get(a)
            .and_then(|sprites| sprites.first())
            .map(|sprite| (&sprite.asset_root, &sprite.texture))
            .cmp(
                &sprites
                    .get(b)
                    .and_then(|sprites| sprites.first())
                    .map(|sprite| (&sprite.asset_root, &sprite.texture)),
            )
            .then_with(|| a.cmp(b))
    });
    for name in names {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let Some(candidates) = sprites.get(&name) else {
            if !optional.contains(&name) {
                errors.push(format!("Could not resolve sprite {name}"));
            }
            continue;
        };
        let size = sizes[&name].max(1);
        let mut failures = Vec::new();
        // Later definitions are fallbacks for earlier ones whose texture cannot load.
        for sprite in candidates {
            let key = (
                sprite.asset_root.clone(),
                sprite.texture.clone(),
                sprite.frames,
                size,
            );
            if let Some(image) = image_cache.get(&key) {
                icons.insert(name.clone(), Arc::clone(image));
                failures.clear();
                break;
            }
            if texture_cache
                .as_ref()
                .is_none_or(|(path, _)| path.0 != sprite.asset_root || path.1 != sprite.texture)
            {
                texture_cache = Some((
                    (sprite.asset_root.clone(), sprite.texture.clone()),
                    decode_texture(root, &sprite.asset_root, &sprite.texture),
                ));
            }
            let texture = &texture_cache.as_ref().expect("texture cache initialized").1;
            match texture {
                Ok(texture)
                    if texture.width() >= sprite.frames && texture.width() % sprite.frames == 0 =>
                {
                    let frame =
                        texture.crop_imm(0, 0, texture.width() / sprite.frames, texture.height());
                    let frame = if frame.width() > size || frame.height() > size {
                        frame.thumbnail(size, size)
                    } else {
                        frame
                    }
                    .to_rgba8();
                    let image = Arc::new(IconImage {
                        width: frame.width(),
                        height: frame.height(),
                        pixels: frame.into_raw(),
                    });
                    image_cache.insert(key, Arc::clone(&image));
                    icons.insert(name.clone(), image);
                    failures.clear();
                    break;
                }
                Ok(_) => failures.push(format!("Invalid frame count for sprite {name}")),
                Err(error) => failures.push(format!(
                    "Could not load sprite {name} ({}): {error}",
                    sprite.texture.display()
                )),
            }
        }
        errors.extend(failures);
    }
    (icons, errors)
}

pub fn load_textures(
    root: &Path,
    textures: HashMap<String, String>,
    size: u32,
    cancelled: &AtomicBool,
) -> (Icons, Vec<String>) {
    let mut icons = Icons::new();
    let mut errors = Vec::new();
    for (name, value) in textures {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let Some(relative) = safe_texture_path(&value) else {
            errors.push(format!("Invalid texture path for image {name}"));
            continue;
        };
        match decode_texture(root, Path::new(""), &relative) {
            Ok(texture) => {
                let frame = if texture.width() > size.max(1) || texture.height() > size.max(1) {
                    texture.thumbnail(size.max(1), size.max(1))
                } else {
                    texture
                }
                .to_rgba8();
                icons.insert(
                    name,
                    Arc::new(IconImage {
                        width: frame.width(),
                        height: frame.height(),
                        pixels: frame.into_raw(),
                    }),
                );
            }
            Err(error) => errors.push(format!(
                "Could not load image {name} ({}): {error}",
                relative.display()
            )),
        }
    }
    (icons, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoi4::focus::{self, FocusTree};
    use image::{Rgba, RgbaImage};
    use tempfile::tempdir;

    fn trees(names: &[&str]) -> Vec<FocusTree> {
        let focuses = names
            .iter()
            .enumerate()
            .map(|(index, name)| format!("focus = {{ id = f{index} icon = {name} }}"))
            .collect::<Vec<_>>()
            .join(" ");
        let input = format!("focus_tree = {{ {focuses} }}");
        focus::extract(&BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(&input).unwrap(),
        )]))
    }

    fn load_trees(
        root: &Path,
        trees: &[FocusTree],
        cancelled: &AtomicBool,
    ) -> (Icons, Vec<String>) {
        let wanted = trees
            .iter()
            .flat_map(|tree| &tree.focuses)
            .filter_map(|focus| focus.icon.clone().map(|name| (name, ICON_SIZE)))
            .collect();
        load(root, wanted, &HashSet::new(), cancelled)
    }

    fn fixture(root: &Path, gfx: &str) {
        fs::create_dir_all(root.join("interface")).unwrap();
        fs::create_dir_all(root.join("gfx")).unwrap();
        fs::write(root.join("interface/sprites.gfx"), gfx).unwrap();
    }

    fn dds_rgba() -> Vec<u8> {
        let mut bytes = b"DDS ".to_vec();
        let header = [
            124, 0x100f, 1, 2, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 0x41, 0, 32, 0xff,
            0xff00, 0xff0000, 0xff000000, 0x1000, 0, 0, 0, 0,
        ];
        for value in header {
            bytes.extend_from_slice(&u32::to_le_bytes(value));
        }
        bytes.extend_from_slice(&[255, 0, 0, 255, 0, 255, 0, 255]);
        bytes
    }

    #[test]
    fn decodes_rgb555_and_rejects_truncated_pixels() {
        let dir = tempdir().unwrap();
        fixture(
            dir.path(),
            r#"spriteType = { name = GFX_rgb texturefile = "gfx/rgb.dds" }"#,
        );
        let mut bytes = b"DDS ".to_vec();
        let header: [u32; 31] = [
            124, 0x100f, 1, 2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 0x40, 0, 16, 0x7c00,
            0x03e0, 0x001f, 0, 0x1000, 0, 0, 0, 0,
        ];
        for value in header {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        bytes.extend_from_slice(&[0x00, 0x7c, 0xe0, 0x03]);
        fs::write(dir.path().join("gfx/rgb.dds"), &bytes).unwrap();
        let (icons, errors) = load_trees(dir.path(), &trees(&["GFX_rgb"]), &AtomicBool::new(false));
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(icons["GFX_rgb"].pixels, [255, 0, 0, 255, 0, 255, 0, 255]);
        bytes.pop();
        fs::write(dir.path().join("gfx/rgb.dds"), bytes).unwrap();
        let (icons, errors) = load_trees(dir.path(), &trees(&["GFX_rgb"]), &AtomicBool::new(false));
        assert!(icons.is_empty());
        assert!(errors
            .iter()
            .any(|error| error.contains("truncated RGB555")));
    }

    #[test]
    fn loads_direct_textures_and_rejects_paths_outside_the_root() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("gfx")).unwrap();
        RgbaImage::from_pixel(1, 1, Rgba([3, 4, 5, 255]))
            .save(dir.path().join("gfx/direct.png"))
            .unwrap();
        let (icons, errors) = load_textures(
            dir.path(),
            HashMap::from([
                ("gfx/direct.png".into(), "gfx/direct.png".into()),
                ("unsafe".into(), "../outside.png".into()),
            ]),
            96,
            &AtomicBool::new(false),
        );
        assert_eq!(icons["gfx/direct.png"].pixels, [3, 4, 5, 255]);
        assert!(errors
            .iter()
            .any(|error| error.contains("Invalid texture path")));
    }

    #[test]
    fn loads_dlc_textures_and_lets_dlc_override_valid_base_sprite() {
        let dir = tempdir().unwrap();
        fixture(
            dir.path(),
            r#"spriteType = { name = GFX_shared texturefile = "gfx/base.png" }"#,
        );
        RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255]))
            .save(dir.path().join("gfx/base.png"))
            .unwrap();
        let dlc = dir.path().join("dlc/pack");
        fs::create_dir_all(dlc.join("interface")).unwrap();
        fs::create_dir_all(dlc.join("gfx")).unwrap();
        fs::write(
            dlc.join("interface/sprites.gfx"),
            r#"spriteType = { name = GFX_dlc texturefile = "gfx/image.png" }
                   spriteType = { name = GFX_shared texturefile = "gfx/image.png" }"#,
        )
        .unwrap();
        RgbaImage::from_pixel(1, 1, Rgba([4, 5, 6, 255]))
            .save(dlc.join("gfx/image.png"))
            .unwrap();
        let (icons, errors) = load_trees(
            dir.path(),
            &trees(&["GFX_dlc", "GFX_shared"]),
            &AtomicBool::new(false),
        );
        assert_eq!(icons["GFX_dlc"].pixels, [4, 5, 6, 255]);
        assert_eq!(icons["GFX_shared"].pixels, [4, 5, 6, 255]);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_dlc_texture_symlinks_outside_dlc_root() {
        let dir = tempdir().unwrap();
        fixture(dir.path(), "");
        let dlc = dir.path().join("dlc/pack");
        fs::create_dir_all(dlc.join("interface")).unwrap();
        fs::create_dir_all(dlc.join("gfx")).unwrap();
        RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255]))
            .save(dir.path().join("gfx/base.png"))
            .unwrap();
        std::os::unix::fs::symlink(dir.path().join("gfx/base.png"), dlc.join("gfx/link.png"))
            .unwrap();
        fs::write(
            dlc.join("interface/sprites.gfx"),
            r#"spriteType = { name = GFX_link texturefile = "gfx/link.png" }"#,
        )
        .unwrap();
        let (icons, errors) =
            load_trees(dir.path(), &trees(&["GFX_link"]), &AtomicBool::new(false));
        assert!(icons.is_empty());
        assert!(errors
            .iter()
            .any(|error| error.contains("outside the game folder")));
    }
    #[test]
    fn resolves_sprites_decodes_dds_and_reuses_frames() {
        let dir = tempdir().unwrap();
        fixture(
            dir.path(),
            r#"spriteTypes = {
            spriteType = { name = GFX_first texturefile = "gfx/sheet.dds" noOfFrames = 2 }
            spriteType = { name = GFX_shared texturefile = "gfx/sheet.dds" noOfFrames = 2 }
            spriteType = { name = GFX_full texturefile = "gfx/sheet.dds" }
        }"#,
        );
        fs::write(dir.path().join("gfx/sheet.dds"), dds_rgba()).unwrap();
        let (icons, errors) = load_trees(
            dir.path(),
            &trees(&["GFX_first", "GFX_shared", "GFX_full"]),
            &AtomicBool::new(false),
        );
        assert!(errors.is_empty(), "{errors:?}");
        assert!(Arc::ptr_eq(&icons["GFX_first"], &icons["GFX_shared"]));
        assert_eq!(
            (icons["GFX_first"].width, icons["GFX_first"].height),
            (1, 1)
        );
        assert_eq!(icons["GFX_first"].pixels, [255, 0, 0, 255]);
        assert_eq!((icons["GFX_full"].width, icons["GFX_full"].height), (2, 1));
    }

    #[test]
    fn dlc_definition_with_missing_first_texture_overrides_valid_base_sprite() {
        let dir = tempdir().unwrap();
        fixture(
            dir.path(),
            r#"spriteType = { name = GFX_p texturefile = "gfx/base.png" }"#,
        );
        RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255]))
            .save(dir.path().join("gfx/base.png"))
            .unwrap();
        let dlc = dir.path().join("dlc/pack");
        fs::create_dir_all(dlc.join("interface")).unwrap();
        fs::create_dir_all(dlc.join("gfx")).unwrap();
        fs::write(
            dlc.join("interface/a.gfx"),
            r#"spriteType = { name = GFX_p texturefile = "gfx/missing.png" }"#,
        )
        .unwrap();
        fs::write(
            dlc.join("interface/b.gfx"),
            r#"spriteType = { name = GFX_p texturefile = "gfx/real.png" }"#,
        )
        .unwrap();
        RgbaImage::from_pixel(1, 1, Rgba([7, 8, 9, 255]))
            .save(dlc.join("gfx/real.png"))
            .unwrap();
        let (icons, _) = load_trees(dir.path(), &trees(&["GFX_p"]), &AtomicBool::new(false));
        assert_eq!(icons["GFX_p"].pixels, [7, 8, 9, 255]);
    }

    #[test]
    fn falls_back_to_later_definition_when_first_texture_is_missing() {
        let dir = tempdir().unwrap();
        fixture(
            dir.path(),
            r#"spriteTypes = {
            spriteType = { name = GFX_dup texturefile = "gfx/missing.png" }
            spriteType = { name = GFX_dup texturefile = "gfx/real.png" }
        }"#,
        );
        RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255]))
            .save(dir.path().join("gfx/real.png"))
            .unwrap();
        let (icons, errors) = load_trees(dir.path(), &trees(&["GFX_dup"]), &AtomicBool::new(false));
        assert_eq!(icons["GFX_dup"].pixels, [1, 2, 3, 255]);
        assert!(
            !errors.iter().any(|e| e.contains("Could not load")),
            "{errors:?}"
        );
    }

    #[test]
    fn decodes_png_and_tga_and_preserves_valid_sprite_on_conflict() {
        let dir = tempdir().unwrap();
        fixture(
            dir.path(),
            r#"spriteTypes = {
            spriteType = { name = GFX_png texturefile = "gfx/a.png" }
            spriteType = { name = GFX_png texturefile = "gfx/b.tga" }
            spriteType = { name = GFX_tga texturefile = "gfx/b.tga" }
        }"#,
        );
        RgbaImage::from_pixel(1, 1, Rgba([10, 20, 30, 255]))
            .save(dir.path().join("gfx/a.png"))
            .unwrap();
        RgbaImage::from_pixel(1, 1, Rgba([40, 50, 60, 255]))
            .save(dir.path().join("gfx/b.tga"))
            .unwrap();
        let (icons, errors) = load_trees(
            dir.path(),
            &trees(&["GFX_png", "GFX_tga"]),
            &AtomicBool::new(false),
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Conflicting sprite"));
        assert_eq!(icons["GFX_png"].pixels, [10, 20, 30, 255]);
        assert_eq!(icons["GFX_tga"].pixels, [40, 50, 60, 255]);
    }

    #[test]
    fn reports_bad_assets_and_keeps_good_icons() {
        let dir = tempdir().unwrap();
        fixture(
            dir.path(),
            r#"spriteTypes = {
            spriteType = { name = GFX_good texturefile = "gfx/a.png" }
            spriteType = { name = GFX_unsafe texturefile = "../outside.png" }
            spriteType = { name = GFX_absent texturefile = "gfx/missing.png" }
            spriteType = { name = GFX_bad texturefile = "gfx/bad.dds" }
            spriteType = { name = GFX_frames texturefile = "gfx/a.png" noOfFrames = 3 }
        }"#,
        );
        RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255]))
            .save(dir.path().join("gfx/a.png"))
            .unwrap();
        fs::write(dir.path().join("gfx/bad.dds"), b"invalid").unwrap();
        let (icons, errors) = load_trees(
            dir.path(),
            &trees(&[
                "GFX_good",
                "GFX_unsafe",
                "GFX_absent",
                "GFX_bad",
                "GFX_frames",
                "GFX_unknown",
            ]),
            &AtomicBool::new(false),
        );
        assert_eq!(icons.len(), 1);
        assert!(icons.contains_key("GFX_good"));
        for name in [
            "GFX_unsafe",
            "GFX_absent",
            "GFX_bad",
            "GFX_frames",
            "GFX_unknown",
        ] {
            assert!(
                errors.iter().any(|error| error.contains(name)),
                "{name}: {errors:?}"
            );
        }
        assert!(safe_texture_path("/absolute.png").is_none());
        assert!(safe_texture_path("C:/absolute.png").is_none());
        assert_eq!(
            safe_texture_path(r"gfx\icon.png"),
            Some(PathBuf::from("gfx/icon.png"))
        );
    }

    #[test]
    fn reports_malformed_relevant_definitions_and_honors_cancellation() {
        let dir = tempdir().unwrap();
        fixture(
            dir.path(),
            "spriteType = { name = GFX_bad texturefile = \"unterminated",
        );
        fs::write(
            dir.path().join("interface/unrelated.gfx"),
            "invalid = \"unterminated",
        )
        .unwrap();
        let (icons, errors) = load_trees(dir.path(), &trees(&["GFX_bad"]), &AtomicBool::new(false));
        assert!(icons.is_empty());
        assert!(errors.iter().any(|error| error.contains("sprites.gfx")));
        assert!(!errors.iter().any(|error| error.contains("unrelated.gfx")));
        let (icons, _) = load_trees(dir.path(), &trees(&["GFX_bad"]), &AtomicBool::new(true));
        assert!(icons.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_outside_game_root() {
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fixture(
            root.path(),
            r#"spriteType = { name = GFX_link texturefile = "gfx/link.png" }"#,
        );
        RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255]))
            .save(outside.path().join("icon.png"))
            .unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("icon.png"),
            root.path().join("gfx/link.png"),
        )
        .unwrap();
        let (icons, errors) =
            load_trees(root.path(), &trees(&["GFX_link"]), &AtomicBool::new(false));
        assert!(icons.is_empty());
        assert!(errors
            .iter()
            .any(|error| error.contains("outside the game folder")));
    }
}
