//! Playlists: M3U8 files in ~/.waver/playlists/. Format and design
//! notes live in agent/playlist-design.md.
//!
//! Disk functions take the FOLDER as a parameter (same seam as
//! settings.rs): the short wrappers at the bottom pass playlists_dir()
//! for the app, the tests pass a temp dir, so tests never touch real
//! data.

use std::path::{Path, PathBuf};

/// Where playlists live: ~/.waver/playlists/
pub fn playlists_dir() -> PathBuf {
    // home_dir() is Option<PathBuf>; "." is a harmless never-panic
    // fallback, exactly like settings_path()
    let home = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".waver").join("playlists")
}

/// Names of every saved playlist in `dir`, alphabetically sorted.
/// read_dir order is arbitrary on every OS, so sorting is not cosmetic:
/// without it the list jumps around between openings.
pub fn list_in(dir: &Path) -> Vec<String> {
    let mut names = Vec::new(); // accumulator: starts empty
    // if the folder can't be read, the loop simply never runs: an
    // unreadable playlists folder = empty list, never a panic
    if let Ok(entries) = std::fs::read_dir(dir) {
        // read_dir yields a Result<DirEntry> per item (even listing a
        // folder can fail half-way): flatten() keeps only the Ok ones
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "m3u8") {
                if let Some(stem) = p.file_stem() {
                    names.push(stem.to_string_lossy().into_owned());
                }
            }
        }
    }
    names.sort();
    names // last expression, no semicolon = the return value
}

/// Write `tracks` as an M3U8 file named `name` inside `dir`.
/// The tracks do NOT need to exist on disk: M3U8 stores path strings.
pub fn save_to(dir: &Path, name: &str, tracks: &[PathBuf]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut text = String::from("#EXTM3U\n");
    for t in tracks {
        text.push_str(&format!("{}\n", t.display()));
    }
    // no semicolon: the Result of write IS the return value
    std::fs::write(dir.join(format!("{name}.m3u8")), text)
}

/// Read back the TRACKS of a playlist written by save_to. Returns None
/// (never panics!) when the file is missing, unreadable or not UTF-8:
/// a broken playlist file must not crash the app, it just doesn't load.
/// The caller already knows the name, so only the tracks come back.
pub fn load_from(dir: &Path, name: &str) -> Option<Vec<PathBuf>> {
    let text = std::fs::read_to_string(dir.join(format!("{name}.m3u8"))).ok()?;
    // a UTF-8 BOM (Windows editors and PS5 love it) must not turn the
    // first line into a phantom track: strip it when present
    let text = text.strip_prefix('\u{feff}').unwrap_or(text.as_str());
    // '#' lines (#EXTM3U, #EXTINF) and blanks are metadata, not tracks
    let tracks: Vec<PathBuf> = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(PathBuf::from)
        .collect();
    Some(tracks)
}

/// Delete the playlist file for `name` inside `dir`.
pub fn delete_in(dir: &Path, name: &str) -> std::io::Result<()> {
    // no semicolon: the Result IS the return value
    std::fs::remove_file(dir.join(format!("{name}.m3u8")))
}

/// Append ONE track to a playlist: read, push, rewrite the file.
/// A missing playlist is created on the spot: add-to doubles as a
/// create path (right-click on a song with zero playlists saved
/// would otherwise be a dead end).
pub fn add_track_to(dir: &Path, name: &str, track: &Path) -> std::io::Result<()> {
    // unwrap_or_default: None (missing file) becomes an empty Vec,
    // so the very first add creates the playlist with one track
    let mut tracks = load_from(dir, name).unwrap_or_default();
    tracks.push(track.to_path_buf()); // to_path_buf: &Path -> owned PathBuf
    save_to(dir, name, &tracks)
}

// --- app-facing wrappers: the UI never builds playlist paths itself ---

pub fn list() -> Vec<String> {
    list_in(&playlists_dir())
}

pub fn save(name: &str, tracks: &[PathBuf]) -> std::io::Result<()> {
    save_to(&playlists_dir(), name, tracks)
}

pub fn load(name: &str) -> Option<Vec<PathBuf>> {
    load_from(&playlists_dir(), name)
}

pub fn delete(name: &str) -> std::io::Result<()> {
    delete_in(&playlists_dir(), name)
}

pub fn add_track(name: &str, track: &Path) -> std::io::Result<()> {
    add_track_to(&playlists_dir(), name, track)
}

#[cfg(test)]
mod tests {
    use super::*;

    // tests must never touch the real home: temp dirs only. ONE folder
    // PER TEST: the harness runs tests in parallel, and a shared folder
    // would let one test's clean() erase another test's files mid-flight
    // (this actually bit us: list_is_sorted_and_filtered failed only in
    // the full run). Same reason settings.rs names every temp file.
    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("waver-test-playlists-{name}"))
    }

    fn clean(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn roundtrip_playlist() {
        let dir = temp_dir("roundtrip");
        clean(&dir);
        let tracks = vec![
            PathBuf::from("D:\\Music\\a.mp3"),
            PathBuf::from("D:\\Music\\b.flac"),
        ];
        save_to(&dir, "roundtrip", &tracks).expect("save must work");
        let loaded = load_from(&dir, "roundtrip").expect("just saved it");
        assert_eq!(loaded, tracks);
        clean(&dir);
    }

    #[test]
    fn list_is_sorted_and_filtered() {
        let dir = temp_dir("list");
        clean(&dir);
        save_to(&dir, "bravo", &[]).expect("save must work");
        save_to(&dir, "alpha", &[]).expect("save must work");
        // a non-playlist file must NOT appear in the list
        std::fs::write(dir.join("junk.txt"), "nope").expect("write must work");
        assert_eq!(list_in(&dir), vec!["alpha", "bravo"]);
        clean(&dir);
    }

    #[test]
    fn load_missing_returns_none() {
        let dir = temp_dir("missing");
        clean(&dir);
        assert!(load_from(&dir, "nonexistent").is_none());
    }

    #[test]
    fn parser_tolerates_utf8_bom() {
        let dir = temp_dir("bom");
        clean(&dir);
        let _ = std::fs::create_dir_all(&dir);
        // Windows editors often save UTF-8 WITH a BOM: the BOM must
        // not become a phantom track
        let raw = "\u{feff}#EXTM3U\nD:\\Music\\bom.mp3\n";
        std::fs::write(dir.join("bom.m3u8"), raw).expect("write must work");
        assert_eq!(
            load_from(&dir, "bom"),
            Some(vec![PathBuf::from("D:\\Music\\bom.mp3")])
        );
        clean(&dir);
    }

    #[test]
    fn parser_skips_comments_and_blanks() {
        let dir = temp_dir("raw");
        clean(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let raw = "#EXTM3U\n\n#EXTINF:214,Some title\nD:\\Music\\one.mp3\n";
        std::fs::write(dir.join("raw.m3u8"), raw).expect("write must work");
        let loaded = load_from(&dir, "raw").expect("file exists");
        assert_eq!(loaded, vec![PathBuf::from("D:\\Music\\one.mp3")]);
        clean(&dir);
    }

    #[test]
    fn add_track_appends_and_creates() {
        let dir = temp_dir("add");
        clean(&dir);
        // missing playlist: the first add CREATES it with one track
        add_track_to(&dir, "growing", &PathBuf::from("D:\\Music\\one.mp3"))
            .expect("add must work");
        assert_eq!(
            load_from(&dir, "growing"),
            Some(vec![PathBuf::from("D:\\Music\\one.mp3")])
        );
        // existing playlist: the track is APPENDED, nothing is lost
        add_track_to(&dir, "growing", &PathBuf::from("D:\\Music\\two.mp3"))
            .expect("add must work");
        assert_eq!(
            load_from(&dir, "growing"),
            Some(vec![
                PathBuf::from("D:\\Music\\one.mp3"),
                PathBuf::from("D:\\Music\\two.mp3"),
            ])
        );
        clean(&dir);
    }
}
