use crate::resp_types::{RespLoginInfo, RespMinimalAuthorInfo};

pub fn abbreviate_link(href: &str) -> &str {
    href.find("://").map_or(href, |idx1| {
        let after_scheme = &href[(idx1 + 3)..];
        after_scheme
            .find('/')
            .map_or(after_scheme, |idx2| &after_scheme[..idx2])
    })
}

pub fn safe_href(href: &str) -> Option<&str> {
    if href.trim() != href
        || href.is_empty()
        || href.chars().any(|ch| ch.is_ascii_control() || ch == '\\')
    {
        return None;
    }

    if href.starts_with('/') {
        if href.starts_with("//") {
            None
        } else {
            Some(href)
        }
    } else {
        match url::Url::parse(href) {
            Ok(url) if url.scheme() == "http" || url.scheme() == "https" => Some(href),
            Ok(_) | Err(_) => None,
        }
    }
}

pub fn author_is_me(
    author: &Option<RespMinimalAuthorInfo<'_>>,
    login: &Option<RespLoginInfo>,
) -> bool {
    if let Some(author) = author
        && let Some(login) = login
        && author.id == login.user.id
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn abbreviate_link_handles_host_only_urls() {
        assert_eq!(super::abbreviate_link("https://example.com"), "example.com");
        assert_eq!(
            super::abbreviate_link("https://example.com/path"),
            "example.com"
        );
        assert_eq!(super::abbreviate_link("not a url"), "not a url");
    }

    #[test]
    fn safe_href_accepts_only_browser_safe_link_targets() {
        assert_eq!(
            super::safe_href("https://example.com/a"),
            Some("https://example.com/a")
        );
        assert_eq!(
            super::safe_href("http://example.com/a"),
            Some("http://example.com/a")
        );
        assert_eq!(super::safe_href("/posts/1"), Some("/posts/1"));

        assert_eq!(super::safe_href("javascript:alert(1)"), None);
        assert_eq!(
            super::safe_href("data:text/html,<script>alert(1)</script>"),
            None
        );
        assert_eq!(super::safe_href("//evil.example/path"), None);
        assert_eq!(super::safe_href("/\\evil"), None);
        assert_eq!(super::safe_href("https://example.com/a\nb"), None);
        assert_eq!(super::safe_href(" https://example.com/a"), None);
        assert_eq!(super::safe_href("https://example.com/a "), None);
        assert_eq!(super::safe_href("not a url"), None);
    }
}
