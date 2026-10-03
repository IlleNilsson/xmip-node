//! What a System Process Xmip owns says of itself: its name, its location
//! and its purpose (ADR-0053 clause 3), and whatever else it was started
//! with that a reader would otherwise have to dig out of its command line.
//!
//! The name finds a process — every one is `xmip-<what>` — and the
//! declaration says what it is for. A process writes it where it starts, to
//! one file named for it and its pid, and takes it away where it ends; a
//! process that is killed leaves its file behind, and whoever lists the
//! declarations drops the ones whose process is gone. The directory is the
//! node's to say, in `XMIP_PROCESS_DIRECTORY`; unset, it is `xmip/process`
//! under the system's temporary directory, the same for every process on the
//! machine, so that a reader and a writer who were told nothing still meet.
//!
//! No other language writes or reads a declaration again: the runtime's
//! library forwards [`Declaration::declare`] and [`crate::standing`] to the
//! surfaces as `xmip_process_declare_v1` and `xmip_process_declarations_v1`
//! (`xmip_operate.h` section 13), and `Xmip.Surface` and the estate's
//! PowerShell module call those. Until 2026-09-27 .NET wrote the file again
//! and PowerShell read it with a TOML reader of its own.

use codec::toml::quote;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs, io, process};
use xcore::{Clock, SystemClock};

/// The environment variable that names the directory declarations are
/// written to.
pub const DIRECTORY_VARIABLE: &str = "XMIP_PROCESS_DIRECTORY";

/// The keys every declaration writes, which nothing else it says may take.
pub const KEYS: [&str; 6] = ["name", "location", "purpose", "pid", "started_unix", "path"];

/// What a System Process is for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Purpose {
    /// The Playground and everything it spawns, and whatever a test started.
    Test,
    /// Everything else: the product doing its work.
    Runtime,
}

impl Purpose {
    /// The words a process may declare, exact and lowercase.
    pub const WORDS: [&'static str; 2] = ["test", "runtime"];

    /// The word written in a declaration.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Test => "test",
            Self::Runtime => "runtime",
        }
    }

    /// The purpose a word declares, exactly and in lowercase, as a role's
    /// word is (`NodeRole::declared`).
    ///
    /// # Errors
    ///
    /// No purpose is called that: REFUSED, naming the word and the words
    /// there are (ADR-0055). A word is never taken for runtime by default.
    pub fn declared(word: &str) -> Result<Self, String> {
        match word {
            "test" => Ok(Self::Test),
            "runtime" => Ok(Self::Runtime),
            other => Err(format!(
                "REFUSED: no purpose is called {other:?}; a process declares {}.",
                Self::WORDS.join(" or ")
            )),
        }
    }
}

/// What a System Process says of itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    /// What it is: `xmip-<what>`, the name the operating system schedules.
    pub name: String,
    /// Where in Xmip it belongs: the scope it serves, or the surface it
    /// reads where it serves none.
    pub location: String,
    /// Test or runtime.
    pub purpose: Purpose,
    /// What else it says of itself, key to text, in the order it said it: a
    /// Playground node its flags, so no reader parses its command line.
    pub said: Vec<(String, String)>,
}

impl Declaration {
    #[must_use]
    pub fn new(name: impl Into<String>, location: impl Into<String>, purpose: Purpose) -> Self {
        Self {
            name: name.into(),
            location: location.into(),
            purpose,
            said: Vec::new(),
        }
    }

    /// The declaration saying one thing more.
    ///
    /// # Errors
    ///
    /// The key is not a bare TOML key — letters, digits, `_` and `-` — or
    /// is one of [`KEYS`], which every declaration writes itself.
    pub fn with(mut self, key: &str, value: impl Into<String>) -> Result<Self, String> {
        let bare = !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');

        if !bare || KEYS.contains(&key) {
            return Err(format!(
                "REFUSED: a declaration cannot say {key:?}; a key is a bare word and none of {}.",
                KEYS.join(", ")
            ));
        }

        self.said.push((key.to_string(), value.into()));
        Ok(self)
    }

    /// Declare this process in the directory the node names.
    ///
    /// # Errors
    ///
    /// The directory could not be made or the file could not be written. A
    /// process that cannot declare itself still runs; its caller says so.
    pub fn declare(&self) -> io::Result<Declared> {
        self.declare_in(&directory())
    }

    /// Declare this process in a given directory.
    ///
    /// # Errors
    ///
    /// The directory could not be made or the file could not be written.
    pub fn declare_in(&self, directory: &Path) -> io::Result<Declared> {
        fs::create_dir_all(directory)?;

        let pid = process::id();
        let file = directory.join(format!("{}-{pid}.toml", self.name));
        // The file counts whole seconds; the clock is the estate's one.
        let started = u64::try_from(SystemClock.unix_seconds()).unwrap_or(0);
        let path = env::current_exe()
            .map(|path| path.display().to_string())
            .unwrap_or_default();

        fs::write(&file, self.to_toml(pid, started, &path))?;
        Ok(Declared { file })
    }

    /// The declaration as the TOML a reader takes: the three things, which
    /// process said so, when, and from where on disk, then what else it said.
    #[must_use]
    pub fn to_toml(&self, pid: u32, started_unix: u64, path: &str) -> String {
        let mut text = format!(
            "name = {}\nlocation = {}\npurpose = {}\npid = {pid}\n\
             started_unix = {started_unix}\npath = {}\n",
            quote(&self.name),
            quote(&self.location),
            quote(self.purpose.word()),
            quote(path),
        );

        for (key, value) in &self.said {
            // Writing into a String cannot fail.
            let _ = writeln!(text, "{key} = {}", quote(value));
        }

        text
    }
}

/// A declaration that stands while this is held and is taken away when it
/// is dropped.
#[derive(Debug)]
pub struct Declared {
    file: PathBuf,
}

impl Declared {
    /// The file the declaration was written to.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Leave the declaration standing and hand its file to a holder across
    /// the C boundary, which takes it away where its process ends.
    #[must_use]
    pub fn handed_over(mut self) -> PathBuf {
        // What is dropped then names no file, and removing it removes nothing.
        std::mem::take(&mut self.file)
    }
}

impl Drop for Declared {
    fn drop(&mut self) {
        // Gone already is as good as removed.
        let _ = fs::remove_file(&self.file);
    }
}

/// Where declarations are written: what the node names, else `xmip/process`
/// under the system's temporary directory.
#[must_use]
pub fn directory() -> PathBuf {
    env::var_os(DIRECTORY_VARIABLE)
        .filter(|named| !named.is_empty())
        .map_or_else(
            || env::temp_dir().join("xmip").join("process"),
            PathBuf::from,
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let directory = env::temp_dir().join("xmip-declaration-test").join(name);
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    #[test]
    fn a_declaration_stands_while_held_and_is_gone_when_dropped() {
        let directory = scratch("held");
        // The name is whatever the operator called the node, and nothing here
        // reads anything out of it (ADR-0053).
        let cluster = configure::fixture::test_cluster();
        let name = format!(
            "xmip-playground-{}-node-{}",
            cluster.name,
            cluster.node(0).name
        );
        let location = cluster.node_scope(0);
        let declaration = Declaration::new(&name, &location, Purpose::Test);

        let declared = declaration.declare_in(&directory).expect("declared");
        let file = declared.file().to_path_buf();
        let text = fs::read_to_string(&file).expect("the file");

        assert!(
            file.ends_with(format!("{name}-{}.toml", process::id())),
            "{}",
            file.display()
        );
        assert!(text.contains(&format!("name = \"{name}\"")), "{text}");
        assert!(
            text.contains(&format!("location = \"{location}\"")),
            "{text}"
        );
        assert!(text.contains("purpose = \"test\""), "{text}");
        assert!(text.contains(&format!("pid = {}", process::id())), "{text}");

        drop(declared);
        assert!(!file.exists(), "taken away where the process ends");
    }

    #[test]
    fn a_declaration_handed_over_stands_until_its_holder_takes_it_away() {
        let directory = scratch("handed");
        let declared = Declaration::new("xmip-cli", "xmip:///", Purpose::Runtime)
            .declare_in(&directory)
            .expect("declared");

        let file = declared.handed_over();

        assert!(file.exists(), "a holder across the boundary removes it");
        fs::remove_file(file).expect("removed by its holder");
    }

    #[test]
    fn a_windows_path_and_a_quote_survive_as_toml() {
        let declaration = Declaration::new("xmip-cli", "a \"quoted\" place", Purpose::Runtime);
        let text = declaration.to_toml(7, 1_800_000_000, "C:\\Program Files\\Xmip\\xmip-cli.exe");

        assert!(
            text.contains("location = \"a \\\"quoted\\\" place\""),
            "{text}"
        );
        assert!(
            text.contains("path = \"C:\\\\Program Files\\\\Xmip\\\\xmip-cli.exe\""),
            "{text}"
        );
        assert!(text.contains("purpose = \"runtime\""), "{text}");
    }

    #[test]
    fn every_control_character_is_escaped_as_toml_requires() {
        let declaration = Declaration::new("xmip-cli", "a\rb\tc\u{0}d\u{7f}", Purpose::Runtime);
        let text = declaration.to_toml(7, 1_800_000_000, "C:\\x\ny");

        assert!(
            text.contains(r#"location = "a\rb\tc\u0000d\u007F""#),
            "{text}"
        );
        assert!(text.contains(r#"path = "C:\\x\ny""#), "{text}");
        assert_eq!(text.lines().count(), 6, "one line per key: {text}");
    }

    #[test]
    fn what_else_a_process_says_follows_the_six_and_takes_none_of_their_keys() {
        let scope = configure::fixture::test_cluster().scope();
        let declaration = Declaration::new("xmip-playground-node", &scope, Purpose::Test)
            .with("stress", "calm")
            .and_then(|said| said.with("rounds", "0"))
            .expect("two bare keys");
        let text = declaration.to_toml(7, 1, "node");

        assert!(
            text.ends_with("stress = \"calm\"\nrounds = \"0\"\n"),
            "{text}"
        );

        for refused in ["pid", "name", "a b", "", "x=y"] {
            let said = Declaration::new("xmip-cli", "", Purpose::Runtime).with(refused, "1");
            assert!(
                said.is_err_and(|why| why.starts_with("REFUSED")),
                "{refused}"
            );
        }
    }

    #[test]
    fn a_purpose_is_one_of_two_exact_words_and_anything_else_is_refused() {
        assert_eq!(Purpose::declared("test"), Ok(Purpose::Test));
        assert_eq!(Purpose::declared("runtime"), Ok(Purpose::Runtime));

        for stranger in ["TEST", " test", "", "production"] {
            let refused = Purpose::declared(stranger).expect_err(stranger);
            assert!(refused.starts_with("REFUSED"), "{refused}");
            assert!(refused.contains("test or runtime"), "{refused}");
        }
    }
}
