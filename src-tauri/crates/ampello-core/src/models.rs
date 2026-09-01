// SPDX-License-Identifier: GPL-3.0-or-later
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub trigger: String,
    pub content: String,
    pub enabled: bool,
    pub favorite: bool,
    pub category_id: Option<String>,
    pub usage_count: i64,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,

    #[serde(default)]
    pub attachments: Vec<Attachment>,

    pub attachments_first: bool,

    pub strict_order: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub snippet_id: String,
    pub position: i64,
    pub name: String,

    pub mime: String,
    pub digest: String,
    pub size_bytes: i64,
    pub created_at: i64,

    #[serde(default)]
    pub present: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetSummary {
    pub id: String,
    pub trigger: String,
    pub preview: String,
    pub content_length: i64,
    pub enabled: bool,
    pub favorite: bool,
    pub category_id: Option<String>,
    pub usage_count: i64,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,

    pub attachment_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub position: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSnippet {
    pub trigger: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub category_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetPatch {
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub favorite: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    pub category_id: Option<Option<String>>,
    #[serde(default)]
    pub attachments_first: Option<bool>,
    #[serde(default)]
    pub strict_order: Option<bool>,
}

// Distinguishes an absent field from an explicit null, which for `category_id`
// is the difference between "leave alone" and "clear it".
fn double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    pub path: String,

    pub recovered_from: Option<String>,
    pub snippet_count: i64,
    pub category_count: i64,
    pub schema_version: i64,
    pub size_bytes: i64,
}

pub fn preview_of(content: &str) -> String {
    const LIMIT: usize = 160;
    let mut out = String::with_capacity(LIMIT);
    let mut pending_space = false;
    let mut count = 0usize;

    for ch in content.chars() {
        if ch.is_whitespace() {
            if count > 0 {
                pending_space = true;
            }
            continue;
        }
        if pending_space {
            out.push(' ');
            count += 1;
            pending_space = false;
            if count >= LIMIT {
                break;
            }
        }
        out.push(ch);
        count += 1;
        if count >= LIMIT {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::preview_of;

    #[test]
    fn collapses_whitespace_without_touching_the_original() {
        let content = "Hello,\n\n  Thank you   for reaching out.\n";
        assert_eq!(preview_of(content), "Hello, Thank you for reaching out.");
    }

    #[test]
    fn is_char_safe_for_unicode() {
        let content = "日本語のテキスト ".repeat(100);
        let preview = preview_of(&content);
        assert!(preview.chars().count() <= 160);

        assert!(preview.starts_with("日本語"));
    }

    #[test]
    fn handles_empty_and_whitespace_only() {
        assert_eq!(preview_of(""), "");
        assert_eq!(preview_of("   \n\t  "), "");
    }

    #[test]
    fn keeps_emoji_intact() {
        assert_eq!(preview_of("ship it 🚀"), "ship it 🚀");
    }
}
