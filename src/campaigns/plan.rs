//! Pure planning: how many readers go in each wave, and the archive slug.
pub const FIRST_WAVE: usize = 100;
pub const MAX_WAVE: usize = 10_000;
pub const SINGLE_WAVE_BELOW: usize = 500;
pub const WAVE_GAP_MINUTES: i64 = 10;
pub const MAX_SLUG: usize = 60;

/// Under 500 readers: one wave. Else 100, then double each wave (at most 10,000); the last takes the rest.
pub fn wave_sizes(n: usize) -> Vec<usize> {
    if n == 0 {
        return vec![];
    }
    if n < SINGLE_WAVE_BELOW {
        return vec![n];
    }
    let (mut out, mut left, mut size) = (Vec::new(), n, FIRST_WAVE);
    while left > 0 {
        let take = size.min(left);
        out.push(take);
        left -= take;
        size = (size * 2).min(MAX_WAVE);
    }
    out
}

/// Cumulative ends: [100, 200, 400] → [100, 300, 700].
pub fn wave_ends(sizes: &[usize]) -> Vec<i32> {
    sizes
        .iter()
        .scan(0usize, |acc, s| {
            *acc += s;
            Some(*acc as i32)
        })
        .collect()
}

/// Lowercase a-z0-9, every other run becomes one `-`, at most 60 characters, never empty.
pub fn slug_from_subject(subject: &str) -> String {
    let mut s = String::new();
    for c in subject.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    s.truncate(MAX_SLUG); // ASCII only, so any byte index is a char boundary
    while s.ends_with('-') {
        s.pop();
    }
    if s.is_empty() { "issue".into() } else { s }
}

/// `base`, or `base-2`, `base-3`, … whichever is free.
pub fn pick_slug(base: &str, taken: &[String]) -> String {
    if !taken.iter().any(|t| t == base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|s| !taken.contains(s))
        .unwrap_or_else(|| base.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_plan_edges() {
        assert_eq!(wave_sizes(0), Vec::<usize>::new());
        assert_eq!(wave_sizes(1), vec![1]);
        assert_eq!(wave_sizes(499), vec![499]);
        assert_eq!(wave_sizes(500), vec![100, 200, 200]);
        assert_eq!(wave_sizes(1200), vec![100, 200, 400, 500]);
        assert_eq!(
            wave_sizes(30_000),
            vec![100, 200, 400, 800, 1600, 3200, 6400, 10_000, 7300]
        );
        assert_eq!(wave_sizes(50_000).iter().max(), Some(&10_000));
        for n in [0, 1, 499, 500, 1200, 30_000, 123_457] {
            assert_eq!(wave_sizes(n).iter().sum::<usize>(), n);
        }
        assert_eq!(wave_ends(&[100, 200, 400, 500]), vec![100, 300, 700, 1200]);
    }

    #[test]
    fn slugs_from_any_subject() {
        assert_eq!(slug_from_subject("Hello, World!"), "hello-world");
        assert_eq!(
            slug_from_subject("  Weekly #3 — October  "),
            "weekly-3-october"
        );
        assert_eq!(slug_from_subject("Café ☕ news #3"), "caf-news-3");
        assert_eq!(slug_from_subject("日本語のニュース"), "issue");
        assert_eq!(slug_from_subject(&"!".repeat(300)), "issue");
        assert_eq!(slug_from_subject(""), "issue");
        let long = slug_from_subject(&"word ".repeat(40));
        assert!(long.len() <= MAX_SLUG && !long.ends_with('-') && long.starts_with("word-word"));
        assert_eq!(pick_slug("hello", &[]), "hello");
        assert_eq!(pick_slug("hello", &["hello".into()]), "hello-2");
        assert_eq!(
            pick_slug(
                "hello",
                &["hello".into(), "hello-2".into(), "hello-world".into()]
            ),
            "hello-3"
        );
    }
}
