//! A basic filter for chat messages (App Store guideline 1.2 asks apps with user content to
//! filter objectionable material): listed words are masked, keeping their first letter
//! ("f***"). Matches whole words, ignoring case; common letter swaps (0→o, 1→i, @→a, $→s) are
//! undone before matching.

const WORDS: &[&str] = &[
    "fuck",
    "fucking",
    "fucker",
    "fucked",
    "motherfucker",
    "shit",
    "shitty",
    "bullshit",
    "bitch",
    "bitches",
    "bastard",
    "asshole",
    "dick",
    "dickhead",
    "cunt",
    "pussy",
    "slut",
    "whore",
    "wanker",
    "twat",
    "prick",
    "cock",
    "nigger",
    "nigga",
    "faggot",
    "fag",
    "retard",
    "kys",
    "كس",
    "كسمك",
    "زب",
    "شرموط",
    "شرموطة",
    "قحبة",
    "منيوك",
    "خول",
    "عاهرة",
    "حقير",
    "كلب",
];

/// The body with listed words masked.
pub fn clean(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut word = String::new();
    for c in body.chars() {
        if c.is_alphanumeric() || matches!(c, '@' | '$') {
            word.push(c);
        } else {
            flush(&mut out, &mut word);
            out.push(c);
        }
    }
    flush(&mut out, &mut word);
    out
}

fn flush(out: &mut String, word: &mut String) {
    if word.is_empty() {
        return;
    }
    if is_listed(word) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.push(first);
        }
        out.extend(chars.map(|_| '*'));
    } else {
        out.push_str(word);
    }
    word.clear();
}

fn is_listed(word: &str) -> bool {
    let normalized: String = word
        .to_lowercase()
        .chars()
        .map(|c| match c {
            '0' => 'o',
            '1' | '!' => 'i',
            '3' => 'e',
            '4' | '@' => 'a',
            '5' | '$' => 's',
            '7' => 't',
            other => other,
        })
        .collect();
    WORDS.contains(&normalized.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_listed_words_only() {
        assert_eq!(clean("what the fuck, man"), "what the f***, man");
        assert_eq!(clean("SHIT happens"), "S*** happens");
        assert_eq!(clean("f@ck? no: sh1t"), "f@ck? no: s***");
        assert_eq!(
            clean("Scunthorpe is a town"),
            "Scunthorpe is a town",
            "whole words only"
        );
        assert_eq!(clean("see you at 8pm"), "see you at 8pm");
        assert_eq!(clean("hello"), "hello");
    }
}
