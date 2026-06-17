use crate::lang;

pub struct SafeTimeAgo<'a> {
    pub since: &'a str,
    pub lang: &'a crate::Translator,
}

impl render::Render for SafeTimeAgo<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, writer: &mut W) -> std::fmt::Result {
        match chrono::DateTime::parse_from_rfc3339(self.since) {
            Ok(since) => TimeAgo {
                since,
                lang: self.lang,
            }
            .render_into(writer),
            Err(_) => render::rsx! {
                <span title={self.since}>{self.since}</span>
            }
            .render_into(writer),
        }
    }
}

#[render::component]
pub fn TimeAgo<'a>(
    since: chrono::DateTime<chrono::offset::FixedOffset>,
    lang: &'a crate::Translator,
) {
    let since_str = since.to_rfc3339();

    let duration = chrono::offset::Utc::now().signed_duration_since(since);

    let arg = {
        let weeks = duration.num_weeks();
        if weeks > 52 {
            let years = u32::try_from(weeks / 52).unwrap_or(u32::MAX);
            lang::timeago_years(years)
        } else if weeks > 5 {
            let months = u8::try_from((weeks * 100) / 435).unwrap_or(u8::MAX);
            lang::timeago_months(months)
        } else if weeks > 0 {
            lang::timeago_weeks(weeks)
        } else {
            let days = duration.num_days();
            if days > 0 {
                lang::timeago_days(days)
            } else {
                let hours = duration.num_hours();
                if hours > 0 {
                    lang::timeago_hours(hours)
                } else {
                    let minutes = duration.num_minutes();
                    if minutes > 0 {
                        lang::timeago_minutes(minutes)
                    } else {
                        let seconds = duration.num_seconds();

                        match seconds.cmp(&0) {
                            std::cmp::Ordering::Greater => lang::timeago_seconds(seconds),
                            std::cmp::Ordering::Less => lang::timeago_future(),
                            std::cmp::Ordering::Equal => lang::timeago_now(),
                        }
                    }
                }
            }
        }
    };
    let text = lang.tr(&arg).into_owned();

    render::rsx! {
        <span title={since_str}>{text}</span>
    }
}

#[cfg(test)]
mod tests {
    use render::Render;

    #[test]
    fn safe_time_ago_renders_bad_timestamps_without_panicking() {
        let lang = crate::get_lang_for_headers(&crate::hyper::HeaderMap::default());
        let mut html = String::new();

        super::SafeTimeAgo {
            since: "<bad timestamp>",
            lang: &lang,
        }
        .render_into(&mut html)
        .unwrap();

        assert!(html.contains("&lt;bad timestamp&gt;"));
        assert!(!html.contains("<bad timestamp>"));
    }

    #[test]
    fn safe_time_ago_renders_valid_timestamps_as_timeago() {
        let lang = crate::get_lang_for_headers(&crate::hyper::HeaderMap::default());
        let mut html = String::new();

        super::SafeTimeAgo {
            since: "2026-01-01T00:00:00+00:00",
            lang: &lang,
        }
        .render_into(&mut html)
        .unwrap();

        assert!(html.contains("2026-01-01T00:00:00+00:00"));
        assert!(!html.contains("&lt;"));
    }
}
