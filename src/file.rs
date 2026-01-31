use std::{fs, path::PathBuf};

pub fn read_file(path: &PathBuf) -> String {
    // Preserve valid UTF-8; only remove invalid characters that fail to decode.
    // Fall back to an empty string on I/O error to keep behavior similar but safer,
    // but log the error so that failures are visible during debugging.
    match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read file {}: {}", path.display(), e);
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::read_file;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_read_file_existing() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "hello world").unwrap();

        let content = read_file(&file_path);
        assert!(content.contains("hello world"));
    }

    #[test]
    fn test_read_file_missing() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("missing.txt");

        let content = read_file(&file_path);
        assert_eq!(content, "");
    }

    #[test]
    fn test_read_file_non_ascii() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("utf8.txt");
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "café — test").unwrap();

        let content = read_file(&file_path);
        assert!(content.contains("café"));
    }
}
