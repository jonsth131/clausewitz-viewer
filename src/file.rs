use std::{fs, io, path::Path};

pub fn read_file(path: &Path) -> Result<String, io::Error> {
    fs::read_to_string(path)
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

        let content = read_file(&file_path).unwrap();
        assert!(content.contains("hello world"));
    }

    #[test]
    fn test_read_file_missing() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("missing.txt");

        assert!(read_file(&file_path).is_err());
    }

    #[test]
    fn test_read_file_non_ascii() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("utf8.txt");
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "café — test").unwrap();

        let content = read_file(&file_path).unwrap();
        assert!(content.contains("café"));
    }
}
