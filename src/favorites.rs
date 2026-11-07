//! The favorites: songs kept to play again, a song's id a line in
//! ~/.local/share/musicbox/favorites.txt (the genre's name after it, to read
//! by). The first word of a line is its id; whatever else the file holds
//! (notes after an id, lines without one) is kept as it is.

use std::path::PathBuf;

use crate::music::{self, SongId};

pub struct Favorites {
    path: PathBuf,
    lines: Vec<String>,
}

/// The id a line keeps, if it keeps one.
fn id_of(line: &str) -> Option<SongId> {
    line.split_whitespace().next().and_then(SongId::parse)
}

impl Favorites {
    /// The favorites file (`MUSICBOX_FAVORITES`, else in musicbox's own
    /// directory), read; none yet if it is not there.
    pub fn load() -> Favorites {
        let path = std::env::var_os("MUSICBOX_FAVORITES").map_or_else(
            || {
                let home = std::env::var_os("HOME").map_or_else(|| ".".into(), PathBuf::from);
                home.join(".local/share/musicbox/favorites.txt")
            },
            PathBuf::from,
        );
        let lines = std::fs::read_to_string(&path).map(|s| s.lines().map(str::to_string).collect()).unwrap_or_default();
        Favorites { path, lines }
    }

    /// The songs kept, oldest first.
    pub fn ids(&self) -> Vec<SongId> {
        self.lines.iter().filter_map(|l| id_of(l)).collect()
    }

    /// Whether `id` is kept (the same id, as written).
    pub fn has(&self, id: &SongId) -> bool {
        let s = id.to_string();
        self.ids().iter().any(|k| k.to_string() == s)
    }

    /// `id` kept, or no longer if it was; whether it is kept now. The file
    /// is written at once (an error: where, and why).
    pub fn toggle(&mut self, id: &SongId) -> Result<bool, String> {
        let s = id.to_string();
        let was = self.lines.len();
        self.lines.retain(|l| id_of(l).is_none_or(|k| k.to_string() != s));
        let kept = self.lines.len() == was;
        if kept {
            self.lines.push(format!("{s}  {}", music::name(id.music).to_lowercase()));
        }
        self.save().map(|_| kept)
    }

    fn save(&self) -> Result<(), String> {
        let err = |e: std::io::Error| format!("{}: {e}", self.path.display());
        if let Some(d) = self.path.parent() {
            std::fs::create_dir_all(d).map_err(err)?;
        }
        let mut text = self.lines.join("\n");
        text.push('\n');
        let tmp = self.path.with_extension("txt.new");
        std::fs::write(&tmp, text).map_err(err)?;
        std::fs::rename(&tmp, &self.path).map_err(err)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

/// The first song's id in `text`: a bare id, a link to the page playing one
/// (…/musicbox/#ska.2817.0.3), a command line with one (musicbox ska.2817.0.3).
pub fn song_in(text: &str) -> Option<SongId> {
    text.split(|c: char| c.is_whitespace() || "#/?&=\"'<>,;".contains(c)).find_map(SongId::parse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_song_id_is_found_however_it_was_copied() {
        let id = SongId::parse("celtic+ska.2817.0.2.180s.t").expect("an id");
        for text in [
            "celtic+ska.2817.0.2.180s.t",
            "  celtic+ska.2817.0.2.180s.t\n",
            "https://bhoult.github.io/musicbox/#celtic+ska.2817.0.2.180s.t",
            "musicbox celtic+ska.2817.0.2.180s.t --reverb 200",
            "\"celtic+ska.2817.0.2.180s.t\"",
        ] {
            assert_eq!(song_in(text), Some(id), "{text}");
        }
        assert_eq!(song_in("https://bhoult.github.io/musicbox/"), None);
        assert_eq!(song_in("ska"), None);
    }

    #[test]
    fn favorites_are_kept_and_let_go_and_the_rest_of_the_file_stays() {
        let dir = std::env::temp_dir().join(format!("musicbox-fav-{}", std::process::id()));
        let path = dir.join("favorites.txt");
        std::fs::create_dir_all(&dir).expect("a directory");
        std::fs::write(&path, "# mine\nska.2817.0.3  the one with the trumpets\n").expect("written");
        let mut f = Favorites { path: path.clone(), lines: Vec::new() };
        f.lines = std::fs::read_to_string(&path).expect("read").lines().map(str::to_string).collect();
        let (ska, celtic) = (SongId::parse("ska.2817.0.3").expect("id"), SongId::parse("celtic.7.0.1").expect("id"));
        assert!(f.has(&ska) && !f.has(&celtic));
        assert_eq!(f.toggle(&celtic), Ok(true));
        assert_eq!(f.toggle(&ska), Ok(false));
        assert_eq!(f.ids(), [celtic]);
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "# mine\nceltic.7.0.1  celtic\n");
        std::fs::remove_file(&path).expect("removed");
        std::fs::remove_dir(&dir).expect("removed");
    }
}
