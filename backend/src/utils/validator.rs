use regex::Regex;
use once_cell::sync::Lazy;

static YOUTUBE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(https?://)?(www\.)?(youtube\.com/watch\?v=|youtu\.be/)[\w-]+").unwrap()
});

pub fn is_valid_youtube_url(url: &str) -> bool {
    YOUTUBE_REGEX.is_match(url)
}

pub fn validate_urls(urls: Vec<String>) -> Result<Vec<String>, String> {
    let mut valid_urls = Vec::new();
    let mut invalid_urls = Vec::new();

    for url in urls {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            continue;
        }

        if is_valid_youtube_url(trimmed) {
            valid_urls.push(trimmed.to_string());
        } else {
            invalid_urls.push(trimmed.to_string());
        }
    }

    if !invalid_urls.is_empty() {
        return Err(format!(
            "Invalid YouTube URLs:\n{}",
            invalid_urls.join("\n")
        ));
    }

    if valid_urls.is_empty() {
        return Err("No valid URLs provided".to_string());
    }

    Ok(valid_urls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_urls() {
        assert!(is_valid_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_valid_youtube_url("https://youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_valid_youtube_url("http://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_valid_youtube_url("https://youtu.be/dQw4w9WgXcQ"));
        assert!(is_valid_youtube_url("www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_valid_youtube_url("youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_valid_youtube_url("youtu.be/dQw4w9WgXcQ"));
    }

    #[test]
    fn test_invalid_urls() {
        assert!(!is_valid_youtube_url("https://www.google.com"));
        assert!(!is_valid_youtube_url("https://vimeo.com/123456"));
        assert!(!is_valid_youtube_url("not a url"));
        assert!(!is_valid_youtube_url(""));
    }
}
