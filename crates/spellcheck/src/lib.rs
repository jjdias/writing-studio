//! Local spellcheck.
//!
//! Words are checked against a Hunspell dictionary read from disk or from the
//! copy of English (US) built into the program. Nothing is sent anywhere.
//! A project adds its own words — names, places, invented terms — on top.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use spellbook::Dictionary;
use writing_studio_core::{Error, Result};
use writing_studio_document::Document;

/// The dictionary that ships inside the program.
pub const BUNDLED_LANGUAGE: &str = "en_US";

const BUNDLED_AFF: &str = include_str!("../dictionaries/en_US.aff");
const BUNDLED_DIC: &str = include_str!("../dictionaries/en_US.dic");

/// Folder inside a project where a writer can put `name.aff` and `name.dic`.
pub const PROJECT_DICTIONARY_DIR: &str = "dictionaries";

const MAX_SUGGESTIONS: usize = 6;

/// A dictionary the writer can choose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryInfo {
    /// The file name without its extension, such as `en_US` or `pt_PT`.
    pub id: String,
    pub label: String,
    /// `None` for the bundled dictionary.
    pub aff_path: Option<PathBuf>,
}

/// The bundled dictionary, then every `.aff` with a `.dic` beside it in the
/// project's `dictionaries/` folder and in the usual system folders. A
/// dictionary in the project wins over a system one with the same name.
pub fn available_dictionaries(project_root: Option<&Path>) -> Vec<DictionaryInfo> {
    let mut found = vec![DictionaryInfo {
        id: BUNDLED_LANGUAGE.into(),
        label: "English (US)".into(),
        aff_path: None,
    }];
    let mut folders: Vec<PathBuf> = Vec::new();
    if let Some(root) = project_root {
        folders.push(root.join(PROJECT_DICTIONARY_DIR));
    }
    folders.extend(system_folders());
    for folder in folders {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        let mut here: Vec<DictionaryInfo> = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "aff"))
            .filter(|path| path.with_extension("dic").is_file())
            .filter_map(|path| {
                let id = path.file_stem()?.to_str()?.to_string();
                Some(DictionaryInfo {
                    label: id.replace('_', "-"),
                    id,
                    aff_path: Some(path),
                })
            })
            .collect();
        here.sort_by(|a, b| a.id.cmp(&b.id));
        for item in here {
            if !found.iter().any(|known| known.id == item.id) {
                found.push(item);
            }
        }
    }
    found
}

fn system_folders() -> Vec<PathBuf> {
    let mut folders = vec![
        PathBuf::from("/usr/share/hunspell"),
        PathBuf::from("/usr/share/myspell/dicts"),
        PathBuf::from("/Library/Spelling"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        folders.push(PathBuf::from(home).join("Library/Spelling"));
    }
    folders
}

/// Hunspell files are UTF-8 or a one-byte encoding. Bytes that are not UTF-8
/// are read as Latin-1, which is right for the common Western dictionaries.
fn read_dictionary_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => err.into_bytes().into_iter().map(char::from).collect(),
    })
}

/// A word the dictionary and the project do not know, as written in the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownWord {
    pub word: String,
    pub count: usize,
}

pub struct Speller {
    language: String,
    dictionary: Dictionary,
    /// Answers already worked out, by the word as written.
    known: RefCell<HashMap<String, bool>>,
}

impl std::fmt::Debug for Speller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Speller")
            .field("language", &self.language)
            .finish()
    }
}

impl Speller {
    pub fn bundled() -> Result<Self> {
        Self::from_text(BUNDLED_LANGUAGE, BUNDLED_AFF, BUNDLED_DIC)
    }

    pub fn load(info: &DictionaryInfo) -> Result<Self> {
        match &info.aff_path {
            None => Self::bundled(),
            Some(aff) => {
                let aff_text = read_dictionary_file(aff)?;
                let dic_text = read_dictionary_file(&aff.with_extension("dic"))?;
                Self::from_text(&info.id, &aff_text, &dic_text)
            }
        }
    }

    pub fn from_text(language: &str, aff: &str, dic: &str) -> Result<Self> {
        let dictionary = Dictionary::new(aff, dic).map_err(|err| {
            Error::message(format!("could not read the {language} dictionary: {err}"))
        })?;
        Ok(Self {
            language: language.to_string(),
            dictionary,
            known: RefCell::new(HashMap::new()),
        })
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    /// Whether the dictionary accepts this word. Curly apostrophes count as
    /// straight ones.
    pub fn knows(&self, word: &str) -> bool {
        if let Some(answer) = self.known.borrow().get(word) {
            return *answer;
        }
        let plain = straighten(word);
        let answer = self.dictionary.check(&plain)
            || plain
                .strip_suffix("'s")
                .is_some_and(|stem| self.dictionary.check(stem));
        self.known.borrow_mut().insert(word.to_string(), answer);
        answer
    }

    /// A few replacements for a word, best first.
    pub fn suggest(&self, word: &str) -> Vec<String> {
        let mut out = Vec::new();
        self.dictionary.suggest(&straighten(word), &mut out);
        out.truncate(MAX_SUGGESTIONS);
        out
    }

    /// Words in `text` that neither the dictionary nor `extra` knows, in the
    /// order they first appear. `extra` holds lowercase words.
    pub fn unknown_words(&self, text: &str, extra: &HashSet<String>) -> Vec<UnknownWord> {
        let mut found: Vec<UnknownWord> = Vec::new();
        let mut at: HashMap<String, usize> = HashMap::new();
        for (_, word) in words(text) {
            if !should_check(word) || self.knows(word) || in_word_list(word, extra) {
                continue;
            }
            let key = straighten(word);
            match at.get(&key) {
                Some(index) => found[*index].count += 1,
                None => {
                    at.insert(key.clone(), found.len());
                    found.push(UnknownWord {
                        word: key,
                        count: 1,
                    });
                }
            }
        }
        found
    }
}

/// The form a project word is stored and compared in: trimmed, lowercase,
/// straight apostrophes. `None` when nothing is left to store.
pub fn normalize_word(word: &str) -> Option<String> {
    let word = straighten(word.trim()).to_lowercase();
    let word = word.trim_matches('\'');
    (!word.is_empty() && !word.chars().any(char::is_whitespace)).then(|| word.to_string())
}

/// Lowercase words from names, so "Mara Vance" makes "mara" and "vance" known.
pub fn name_words<'a>(names: impl IntoIterator<Item = &'a str>) -> HashSet<String> {
    names
        .into_iter()
        .flat_map(|name| words(name).map(|(_, word)| word))
        .filter_map(normalize_word)
        .collect()
}

fn in_word_list(word: &str, extra: &HashSet<String>) -> bool {
    let Some(word) = normalize_word(word) else {
        return true;
    };
    extra.contains(&word)
        || word
            .strip_suffix("'s")
            .is_some_and(|stem| extra.contains(stem))
}

fn straighten(word: &str) -> String {
    word.replace('\u{2019}', "'")
}

/// Numbers, single letters, and words in capitals (acronyms) are left alone.
fn should_check(word: &str) -> bool {
    let letters = word.chars().filter(|ch| ch.is_alphabetic()).count();
    if letters < 2 || word.chars().any(char::is_numeric) {
        return false;
    }
    !word
        .chars()
        .filter(|ch| ch.is_alphabetic())
        .all(char::is_uppercase)
}

fn is_apostrophe(ch: char) -> bool {
    ch == '\'' || ch == '\u{2019}'
}

/// Words with their byte offset. A word is letters and digits, and may hold
/// apostrophes between them ("don't", "o'clock"). The editor's script splits
/// words the same way.
pub fn words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut chars = text.char_indices().peekable();
    std::iter::from_fn(move || {
        while chars.next_if(|(_, ch)| !ch.is_alphanumeric()).is_some() {}
        let (start, _) = *chars.peek()?;
        let mut end = start;
        while let Some((index, ch)) = chars.peek().copied() {
            if ch.is_alphanumeric() {
                end = index + ch.len_utf8();
                chars.next();
            } else if is_apostrophe(ch) {
                let after = text[index + ch.len_utf8()..].chars().next();
                if !after.is_some_and(char::is_alphanumeric) {
                    break;
                }
                chars.next();
            } else {
                break;
            }
        }
        Some((start, &text[start..end]))
    })
}

/// Replace every whole-word `from` in a piece of text. Apostrophe style is
/// ignored when matching. Returns how many were replaced.
pub fn replace_word(text: &mut String, from: &str, to: &str) -> usize {
    let from = straighten(from);
    let hits: Vec<(usize, usize)> = words(text)
        .filter(|(_, word)| straighten(word) == from)
        .map(|(start, word)| (start, start + word.len()))
        .collect();
    for (start, end) in hits.iter().rev() {
        text.replace_range(*start..*end, to);
    }
    hits.len()
}

/// Replace every whole-word `from` in a document. A word split across
/// formatting (half of it bold) is not matched.
pub fn replace_in_document(document: &mut Document, from: &str, to: &str) -> usize {
    let mut count = 0;
    document.edit_text_runs(|text| {
        let changed = replace_word(text, from, to);
        count += changed;
        changed > 0
    });
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_core::Id;

    fn listed(found: &[UnknownWord]) -> Vec<(&str, usize)> {
        found
            .iter()
            .map(|item| (item.word.as_str(), item.count))
            .collect()
    }

    #[test]
    fn splits_words_and_keeps_inner_apostrophes() {
        let found: Vec<&str> = words("“Don’t,” she said—o'clock? 'quoted' mp3 naïve.")
            .map(|(_, word)| word)
            .collect();
        assert_eq!(
            found,
            ["Don’t", "she", "said", "o'clock", "quoted", "mp3", "naïve"]
        );
        let (start, word) = words("  harbor").next().unwrap();
        assert_eq!((start, word), (2, "harbor"));
    }

    #[test]
    fn bundled_english_flags_misspellings_and_counts_them() {
        let speller = Speller::bundled().unwrap();
        let text = "Teh harbor was quiet. She didn’t recieve teh letter in 1840, the NATO Xq one.";
        let found = speller.unknown_words(text, &HashSet::new());
        assert_eq!(listed(&found), [("Teh", 1), ("recieve", 1), ("teh", 1), ("Xq", 1)]);
        assert!(speller.knows("Harbor's"));
        assert!(speller.suggest("recieve").contains(&"receive".to_string()));
    }

    #[test]
    fn project_words_and_names_are_known_in_any_case_and_as_possessives() {
        let speller = Speller::bundled().unwrap();
        let text = "Maravance crossed Zhul. Maravance’s boat. ZHUL-born zhul.";
        assert_eq!(
            listed(&speller.unknown_words(text, &HashSet::new())),
            [("Maravance", 1), ("Zhul", 1), ("Maravance's", 1), ("zhul", 1)]
        );
        let mut extra = name_words(["Maravance of the Reach"]);
        extra.extend(normalize_word(" Zhul "));
        assert!(speller.unknown_words(text, &extra).is_empty());
        assert_eq!(normalize_word("  "), None);
        assert_eq!(normalize_word("two words"), None);
    }

    #[test]
    fn replaces_whole_words_only_and_through_a_document() {
        let mut text = String::from("teh tehran Teh teh's 'teh'");
        assert_eq!(replace_word(&mut text, "teh", "the"), 2);
        assert_eq!(text, "the tehran Teh teh's 'the'");

        let mut document = Document::from_plain_text(Id::new(), "I recieve.\nYou recieve it.");
        assert_eq!(replace_in_document(&mut document, "recieve", "receive"), 2);
        assert_eq!(document.to_plain_text(), "I receive.\nYou receive it.");
        assert_eq!(replace_in_document(&mut document, "recieve", "receive"), 0);
    }

    #[test]
    fn lists_the_bundled_dictionary_first_and_finds_project_dictionaries() {
        let dir = std::env::temp_dir().join(format!("ws-dict-{}", Id::new()));
        let folder = dir.join(PROJECT_DICTIONARY_DIR);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("xx_TEST.aff"), "SET UTF-8\n").unwrap();
        fs::write(folder.join("xx_TEST.dic"), "2\nzhul\nmaravance\n").unwrap();
        fs::write(folder.join("lonely.aff"), "SET UTF-8\n").unwrap();

        let found = available_dictionaries(Some(&dir));
        assert_eq!(found[0].id, BUNDLED_LANGUAGE);
        assert!(found[0].aff_path.is_none());
        let custom = found.iter().find(|item| item.id == "xx_TEST").unwrap();
        assert!(!found.iter().any(|item| item.id == "lonely"));

        let speller = Speller::load(custom).unwrap();
        assert_eq!(speller.language(), "xx_TEST");
        assert!(speller.knows("zhul"));
        assert!(!speller.knows("harbor"));
        let _ = fs::remove_dir_all(dir);
    }
}
