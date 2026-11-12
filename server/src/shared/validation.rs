//! Input normalization and validation shared by feature DTOs.

use crate::error::{AppError, AppResult};

pub const HANDLE_MIN: usize = 3;
pub const HANDLE_MAX: usize = 32;
pub const PASSWORD_MIN: usize = 8;

/// Handles (usernames, playernames) are case-insensitive: stored and looked up in lowercase.
pub fn normalize_handle(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Validates an already-normalized handle. `label` is used in error messages, e.g. "Username".
pub fn validate_handle(label: &str, value: &str) -> AppResult<()> {
    let len = value.chars().count();
    if !(HANDLE_MIN..=HANDLE_MAX).contains(&len) {
        return Err(AppError::BadRequest(format!(
            "{label} must be {HANDLE_MIN}-{HANDLE_MAX} characters"
        )));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return Err(AppError::BadRequest(format!(
            "{label} may only contain letters, numbers, '_', '.' and '-'"
        )));
    }
    Ok(())
}

/// Password rules for every new password (players and staff): at least 8 characters with an
/// uppercase letter, a number and a special character, and not a common password. Existing
/// passwords keep working for sign-in; this applies only when a password is set or changed.
pub fn validate_password(password: &str) -> AppResult<()> {
    let mut missing = Vec::new();
    if password.chars().count() < PASSWORD_MIN {
        missing.push(format!("at least {PASSWORD_MIN} characters"));
    }
    if !password.chars().any(char::is_uppercase) {
        missing.push("an uppercase letter".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        missing.push("a number".to_string());
    }
    if !password.chars().any(is_special) {
        missing.push("a special character (like ! @ # ?)".to_string());
    }
    if !missing.is_empty() {
        return Err(AppError::BadRequest(format!(
            "Password needs {}",
            join_and(&missing)
        )));
    }
    if is_common_password(password) {
        return Err(AppError::BadRequest(
            "This password is too common and easy to guess. Please choose a less predictable one"
                .into(),
        ));
    }
    Ok(())
}

/// Anything that isn't a letter, a number or a space.
fn is_special(c: char) -> bool {
    !c.is_alphanumeric() && !c.is_whitespace()
}

/// "a", "a and b", "a, b and c".
fn join_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Words people build weak passwords from. `Password1!`, `P@ssw0rd123` and `Havoc2026!` meet the
/// character rules but are among the first guesses attackers try.
const COMMON_PASSWORDS: &[&str] = &[
    "password",
    "passw",
    "pass",
    "qwerty",
    "qwertyuiop",
    "qwertz",
    "azerty",
    "asdf",
    "asdfgh",
    "asdfghjkl",
    "zxcvbn",
    "zxcvbnm",
    "qazwsx",
    "qweasd",
    "abc",
    "abcd",
    "abcde",
    "abcdef",
    "abcdefg",
    "abcdefgh",
    "letmein",
    "welcome",
    "admin",
    "administrator",
    "root",
    "login",
    "user",
    "guest",
    "test",
    "testing",
    "default",
    "changeme",
    "secret",
    "iloveyou",
    "love",
    "lovely",
    "hello",
    "hi",
    "monkey",
    "dragon",
    "master",
    "shadow",
    "sunshine",
    "princess",
    "superman",
    "batman",
    "trustno",
    "freedom",
    "whatever",
    "starwars",
    "football",
    "baseball",
    "soccer",
    "hockey",
    "basketball",
    "liverpool",
    "chelsea",
    "arsenal",
    "barcelona",
    "realmadrid",
    "manchester",
    "ronaldo",
    "cristiano",
    "messi",
    "michael",
    "jordan",
    "summer",
    "winter",
    "spring",
    "autumn",
    "family",
    "computer",
    "internet",
    "google",
    "samsung",
    "apple",
    "iphone",
    "android",
    "mobile",
    "money",
    "flower",
    "angel",
    "baby",
    "friend",
    "friends",
    "killer",
    "ninja",
    "pokemon",
    "minecraft",
    "fortnite",
    "gaming",
    "gamer",
    "player",
    "havoc",
    "havoctlt",
    "lasertag",
    "laser",
    "tactical",
    "qatar",
    "doha",
    "allah",
    "mohammed",
    "muhammad",
    "mohamed",
    "ahmed",
    "ahmad",
    "ali",
    "omar",
    "fatima",
    "aaa",
];

/// Whether the password is a common word dressed up with capitals, digits and symbols:
/// `Password1!`, `!Qwerty123`, `P@ssw0rd`, `Laser@Tag1`, `MyPassword9#`.
fn is_common_password(password: &str) -> bool {
    // The word in the middle: leading and trailing digits/symbols are just decoration.
    let core = password.trim_matches(|c: char| !c.is_alphabetic());
    let leet = |c: char| match c {
        '0' => 'o',
        '1' => 'i',
        '3' => 'e',
        '4' => 'a',
        '5' => 's',
        '7' => 't',
        '@' => 'a',
        '$' => 's',
        c => c,
    };
    let letters = |s: &str| -> String { s.chars().filter(|c| c.is_alphabetic()).collect() };
    let plain = letters(&core.to_lowercase());
    let decoded = letters(&core.to_lowercase().chars().map(leet).collect::<String>());
    [plain, decoded].iter().any(|word| {
        word.is_empty()
            // "Aaaaaaa1!": one letter repeated.
            || word.chars().all(|c| word.starts_with(c))
            || COMMON_PASSWORDS
                .iter()
                .any(|common| word == common || (common.len() >= 6 && word.contains(common)))
    })
}

/// Trims a required text field and rejects it if empty.
pub fn required_text(label: &str, value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{label} is required")));
    }
    Ok(value.to_string())
}

/// Trims an optional text field; blank strings become `None`.
pub fn optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub fn validate_email(email: &str) -> AppResult<()> {
    let valid = email.len() <= 254
        && !email.contains(char::is_whitespace)
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if !valid {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }
    Ok(())
}

pub fn validate_phone(phone: &str) -> AppResult<()> {
    let digits = phone.chars().filter(char::is_ascii_digit).count();
    let allowed = phone
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '-' | '(' | ')'));
    if !allowed || !(7..=15).contains(&digits) {
        return Err(AppError::BadRequest("Invalid phone number".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles() {
        assert_eq!(normalize_handle("  Admin_1 "), "admin_1");
        assert!(validate_handle("Username", "admin_1").is_ok());
        for bad in ["ab", "has space", "bad@char", &"x".repeat(33)] {
            assert!(validate_handle("Username", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn emails() {
        assert!(validate_email("a@b.co").is_ok());
        for bad in ["nope", "@b.co", "a@b", "a b@c.de"] {
            assert!(validate_email(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn phones() {
        assert!(validate_phone("+1 (555) 123-4567").is_ok());
        for bad in ["123", "phone", "+1 555 abc 4567"] {
            assert!(validate_phone(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn passwords_need_length_uppercase_number_and_special() {
        for good in [
            "Blue-Falcon42",
            "Tr0mbone#Sky",
            "9xK!mQ2v",
            "Havoc is #1 in Doha",
        ] {
            assert!(validate_password(good).is_ok(), "{good}");
        }
        let err = |p: &str| validate_password(p).unwrap_err().to_string();
        assert!(err("Ab1!").contains("at least 8 characters"));
        assert!(err("blue-falcon42").contains("an uppercase letter"));
        assert!(err("Blue-Falcon").contains("a number"));
        assert!(err("BlueFalcon42").contains("a special character"));
        assert!(
            err("bluefalcon").contains("an uppercase letter, a number and a special character")
        );
    }

    #[test]
    fn common_passwords_are_refused() {
        for weak in [
            "Password1!",
            "P@ssw0rd123",
            "!Qwerty123",
            "Welcome1#",
            "Havoc2026!",
            "Laser@Tag1",
            "MyPassword9#",
            "Aaaaaaa1!",
            "Abcdef12$",
            "Qatar2026!",
            "Iloveyou1!",
        ] {
            let e = validate_password(weak).unwrap_err().to_string();
            assert!(e.contains("too common"), "{weak}: {e}");
        }
    }

    #[test]
    fn optional_text_blank_is_none() {
        assert_eq!(optional_text(Some("  ".into())), None);
        assert_eq!(optional_text(Some(" x ".into())), Some("x".into()));
        assert_eq!(optional_text(None), None);
    }
}
