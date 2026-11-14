//! Scene diagnostics.
//!
//! A scene has three beats the writer fills in: what the viewpoint character
//! wants (goal), what stands in the way (conflict), and how it ends
//! (outcome). This crate holds those, measures a scene's text, and lists what
//! looks worth a second look.
//!
//! Everything here is counting and comparing on this computer. It does not
//! read meaning: it cannot tell whether a goal is a good one, only that none
//! was written down.

/// The three beats of a scene, in the writer's words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SceneBeats {
    pub goal: String,
    pub conflict: String,
    pub outcome: String,
}

impl SceneBeats {
    /// Nothing written in any of the three.
    pub fn is_empty(&self) -> bool {
        self.filled() == 0
    }

    /// How many of the three are written.
    pub fn filled(&self) -> usize {
        [&self.goal, &self.conflict, &self.outcome]
            .into_iter()
            .filter(|beat| !beat.trim().is_empty())
            .count()
    }

    pub fn is_complete(&self) -> bool {
        self.filled() == 3
    }
}

/// How often one character is named in a scene.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    pub name: String,
    pub count: u32,
}

/// What can be counted in a scene's text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SceneFacts {
    pub words: u32,
    pub sentences: u32,
    /// Words in an average sentence, rounded.
    pub average_sentence: u32,
    pub longest_sentence: u32,
    /// Share of the words that sit inside double quotation marks, 0 to 100.
    pub dialogue_percent: u32,
    /// Characters named in the scene, most named first.
    pub characters: Vec<Mention>,
}

/// What the rest of the book says about a scene.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BookContext {
    /// The middle scene length in this book. Zero when there are too few
    /// scenes to say.
    pub typical_words: u32,
    /// Whether the bible lists any characters to look for.
    pub has_cast: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finding {
    NoText,
    NoGoal,
    NoConflict,
    NoOutcome,
    MuchLonger { words: u32, typical: u32 },
    MuchShorter { words: u32, typical: u32 },
    NoCharacterNamed,
    LongSentences { average: u32 },
}

impl Finding {
    /// A missing beat, as opposed to something measured in the text.
    pub fn is_beat(&self) -> bool {
        matches!(self, Finding::NoGoal | Finding::NoConflict | Finding::NoOutcome)
    }

    pub fn message(&self) -> String {
        match self {
            Finding::NoText => "Nothing written yet".into(),
            Finding::NoGoal => "No goal".into(),
            Finding::NoConflict => "No conflict".into(),
            Finding::NoOutcome => "No outcome".into(),
            Finding::MuchLonger { words, typical } => {
                format!("Long: {words} words, where a scene in this book is usually about {typical}")
            }
            Finding::MuchShorter { words, typical } => {
                format!("Short: {words} words, where a scene in this book is usually about {typical}")
            }
            Finding::NoCharacterNamed => "No character from the bible is named".into(),
            Finding::LongSentences { average } => {
                format!("Long sentences: {average} words on average")
            }
        }
    }
}

/// Scenes this many times the typical length are called long.
const LONG_FACTOR: u32 = 2;
/// Scenes under one part in this many of the typical length are called short.
const SHORT_DIVISOR: u32 = 3;
/// An average sentence above this many words is called long.
const LONG_SENTENCE_WORDS: u32 = 30;
/// Fewer scenes with text than this and no length is called typical.
const MIN_SCENES_FOR_TYPICAL: usize = 4;

/// The middle length of the scenes that have text. Zero when there are fewer
/// than four of them.
pub fn typical_words(counts: impl IntoIterator<Item = u32>) -> u32 {
    let mut counts: Vec<u32> = counts.into_iter().filter(|count| *count > 0).collect();
    if counts.len() < MIN_SCENES_FOR_TYPICAL {
        return 0;
    }
    counts.sort_unstable();
    let middle = counts.len() / 2;
    if counts.len() % 2 == 0 {
        (counts[middle - 1] + counts[middle]) / 2
    } else {
        counts[middle]
    }
}

/// What to look at in one scene: missing beats first, then the text.
pub fn findings(beats: &SceneBeats, facts: &SceneFacts, context: &BookContext) -> Vec<Finding> {
    let mut found = Vec::new();
    if beats.goal.trim().is_empty() {
        found.push(Finding::NoGoal);
    }
    if beats.conflict.trim().is_empty() {
        found.push(Finding::NoConflict);
    }
    if beats.outcome.trim().is_empty() {
        found.push(Finding::NoOutcome);
    }
    if facts.words == 0 {
        found.push(Finding::NoText);
        return found;
    }
    let typical = context.typical_words;
    if typical > 0 && facts.words > typical * LONG_FACTOR {
        found.push(Finding::MuchLonger {
            words: facts.words,
            typical,
        });
    } else if typical > 0 && facts.words * SHORT_DIVISOR < typical {
        found.push(Finding::MuchShorter {
            words: facts.words,
            typical,
        });
    }
    if context.has_cast && facts.characters.is_empty() {
        found.push(Finding::NoCharacterNamed);
    }
    if facts.sentences >= 3 && facts.average_sentence > LONG_SENTENCE_WORDS {
        found.push(Finding::LongSentences {
            average: facts.average_sentence,
        });
    }
    found
}

/// Count what can be counted. `names` are the bible's character names.
pub fn scene_facts(text: &str, names: &[String]) -> SceneFacts {
    let (sentences, longest_sentence, words) = sentence_counts(text);
    let average_sentence = if sentences == 0 {
        0
    } else {
        (words + sentences / 2) / sentences
    };
    let spoken = dialogue_words(text);
    SceneFacts {
        words,
        sentences,
        average_sentence,
        longest_sentence,
        dialogue_percent: if words == 0 {
            0
        } else {
            (spoken * 100 + words / 2) / words
        },
        characters: mentions(text, names),
    }
}

fn word_count(text: &str) -> u32 {
    text.split_whitespace()
        .filter(|piece| piece.chars().any(char::is_alphanumeric))
        .count() as u32
}

/// Sentences end at `.`, `!`, `?`, or `…`, and at the end of a paragraph.
/// Returns how many, the longest in words, and the words in all of them.
fn sentence_counts(text: &str) -> (u32, u32, u32) {
    let (mut sentences, mut longest, mut total) = (0u32, 0u32, 0u32);
    for paragraph in text.lines() {
        let mut current = String::new();
        let mut close = |current: &mut String| {
            let words = word_count(current);
            if words > 0 {
                sentences += 1;
                longest = longest.max(words);
                total += words;
            }
            current.clear();
        };
        let mut chars = paragraph.chars().peekable();
        while let Some(ch) = chars.next() {
            current.push(ch);
            if matches!(ch, '.' | '!' | '?' | '\u{2026}') {
                // "3.5" and "e.g.x" do not end a sentence; a space or a closing mark does.
                let ends = chars.peek().is_none_or(|next| !next.is_alphanumeric());
                if ends {
                    close(&mut current);
                }
            }
        }
        close(&mut current);
    }
    (sentences, longest, total)
}

/// Words inside double quotation marks: “ ”, " ", and « ». A quotation left
/// open runs to the end of its paragraph, which is how a speech that carries
/// on into the next paragraph is written.
fn dialogue_words(text: &str) -> u32 {
    let mut spoken = 0;
    for paragraph in text.lines() {
        let mut inside = false;
        let mut current = String::new();
        for ch in paragraph.chars() {
            let (opens, closes) = match ch {
                '\u{201C}' | '\u{00AB}' => (true, false),
                '\u{201D}' | '\u{00BB}' => (false, true),
                '"' => (!inside, inside),
                _ => (false, false),
            };
            if opens && !inside {
                inside = true;
                current.clear();
            } else if closes && inside {
                inside = false;
                spoken += word_count(&current);
            } else if inside {
                current.push(ch);
            }
        }
        if inside {
            spoken += word_count(&current);
        }
    }
    spoken
}

/// A character is named when any capitalized word of their name appears as a
/// whole word, with or without a possessive. Matching keeps the capital, so
/// a character called Rose is not found in "the sun rose".
fn mentions(text: &str, names: &[String]) -> Vec<Mention> {
    let words: Vec<&str> = text
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '\'' || ch == '\u{2019}'))
        .map(|word| {
            word.strip_suffix("'s")
                .or_else(|| word.strip_suffix("\u{2019}s"))
                .unwrap_or(word)
                .trim_matches(|ch| ch == '\'' || ch == '\u{2019}')
        })
        .filter(|word| !word.is_empty())
        .collect();
    let mut found: Vec<Mention> = names
        .iter()
        .filter_map(|name| {
            let parts: Vec<&str> = name
                .split(|ch: char| !ch.is_alphanumeric())
                .filter(|part| part.chars().count() >= 2)
                .filter(|part| part.chars().next().is_some_and(char::is_uppercase))
                .collect();
            let count = words.iter().filter(|word| parts.contains(word)).count() as u32;
            (count > 0).then(|| Mention {
                name: name.trim().to_string(),
                count,
            })
        })
        .collect();
    found.sort_by(|a, b| b.count.cmp(&a.count));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn counts_words_sentences_and_dialogue() {
        let text = "Mara opened the door. “Who is there?” she said. Nobody answered!\n“It was 3.5 miles,” said Rose. “And then";
        let facts = scene_facts(text, &[]);
        assert_eq!(facts.words, 19);
        assert_eq!(facts.sentences, 6);
        // "It was 3.5 miles," said Rose.
        assert_eq!(facts.longest_sentence, 6);
        assert_eq!(facts.average_sentence, 3);
        // Who is there / It was 3.5 miles / And then: 9 of 19 words.
        assert_eq!(facts.dialogue_percent, 47);

        let straight = scene_facts("\"Go,\" he said. \"Now.\"", &[]);
        assert_eq!(straight.words, 4);
        assert_eq!(straight.dialogue_percent, 50);
        assert_eq!(scene_facts("", &[]), SceneFacts::default());
    }

    #[test]
    fn finds_named_characters_by_any_capitalized_part_of_the_name() {
        let text = "Mara crossed the square. The sun rose. Vance’s coat was Rose's. Mara ran.";
        let facts = scene_facts(text, &names(&["Mara Vance", "Rose of the Reach", "Tomas"]));
        assert_eq!(
            facts.characters,
            vec![
                Mention {
                    name: "Mara Vance".into(),
                    count: 3
                },
                Mention {
                    name: "Rose of the Reach".into(),
                    count: 1
                },
            ]
        );
    }

    #[test]
    fn typical_length_needs_four_scenes_with_text() {
        assert_eq!(typical_words([900, 0, 1100, 1000]), 0);
        assert_eq!(typical_words([900, 0, 1100, 1000, 5000]), 1050);
        assert_eq!(typical_words([10, 20, 30, 40, 50]), 30);
    }

    #[test]
    fn lists_missing_beats_first_and_then_what_the_text_shows() {
        let context = BookContext {
            typical_words: 1000,
            has_cast: true,
        };
        let empty = findings(&SceneBeats::default(), &SceneFacts::default(), &context);
        assert_eq!(
            empty,
            vec![
                Finding::NoGoal,
                Finding::NoConflict,
                Finding::NoOutcome,
                Finding::NoText
            ]
        );
        assert!(empty[0].is_beat() && !empty[3].is_beat());

        let beats = SceneBeats {
            goal: "Reach the harbor".into(),
            conflict: "  ".into(),
            outcome: "She is turned back".into(),
        };
        assert_eq!(beats.filled(), 2);
        let facts = SceneFacts {
            words: 2500,
            sentences: 50,
            average_sentence: 50,
            ..SceneFacts::default()
        };
        assert_eq!(
            findings(&beats, &facts, &context),
            vec![
                Finding::NoConflict,
                Finding::MuchLonger {
                    words: 2500,
                    typical: 1000
                },
                Finding::NoCharacterNamed,
                Finding::LongSentences { average: 50 },
            ]
        );

        let short = SceneFacts {
            words: 300,
            sentences: 20,
            average_sentence: 15,
            characters: vec![Mention {
                name: "Mara".into(),
                count: 2,
            }],
            ..SceneFacts::default()
        };
        let complete = SceneBeats {
            conflict: "The tide".into(),
            ..beats
        };
        assert!(complete.is_complete());
        assert_eq!(
            findings(&complete, &short, &context),
            vec![Finding::MuchShorter {
                words: 300,
                typical: 1000
            }]
        );
        // With too few scenes to compare, length is not judged.
        assert!(findings(&complete, &short, &BookContext::default()).is_empty());
    }
}
