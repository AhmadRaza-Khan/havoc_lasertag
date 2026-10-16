//! Plain text from the small bits of HTML that come from third-party rich-text editors
//! (e.g. Bookeo option descriptions), for display in the apps.

/// Line breaks for `<br>`, `</p>`, `</li>` and `</div>`; other tags dropped; common entities
/// decoded; blank lines collapsed. Not a sanitizer for rendering HTML — the result is text.
pub fn to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            rest = "";
            break;
        };
        let tag = rest[start + 1..start + end].trim().to_ascii_lowercase();
        let name = tag
            .trim_start_matches('/')
            .split([' ', '/'])
            .next()
            .unwrap_or("");
        let breaks = name == "br" || (tag.starts_with('/') && matches!(name, "p" | "li" | "div"));
        if breaks {
            out.push('\n');
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);

    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#039;", "'")
        .replace("&amp;", "&");
    decoded
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tags_keeps_lines() {
        assert_eq!(
            to_text(
                r#"<p class="x"><strong>Add the Search &amp; Destroy mission.</strong><br />Second line</p><p></p>"#
            ),
            "Add the Search & Destroy mission.\nSecond line"
        );
        assert_eq!(
            to_text("Extra ammo<br data-start=\"1\" />Up to 3"),
            "Extra ammo\nUp to 3"
        );
        assert_eq!(to_text("no tags"), "no tags");
        assert_eq!(to_text("<ul><li>a</li><li>b</li></ul>"), "a\nb");
        assert_eq!(to_text("broken <b"), "broken");
        assert_eq!(to_text(""), "");
    }
}
