//! Product version: `{version}.{year}.{month}.{patch}`.
//!
//! The line starts at `1.26.3.00`. An update changes the last two digits
//! until the year or month component is moved on purpose.

pub const VERSION: &str = "1.26.3.07";

pub fn parse(tag: &str) -> Option<[u32; 4]> {
    let tag = tag.trim().trim_start_matches('v');
    let mut parts = tag.split('.');
    let version = parts.next()?.parse().ok()?;
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some([version, year, month, patch])
}

pub fn cmp(a: [u32; 4], b: [u32; 4]) -> std::cmp::Ordering {
    a.cmp(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_version_parses() {
        assert_eq!(parse("1.26.3.00"), Some([1, 26, 3, 0]));
        assert_eq!(parse(VERSION), Some([1, 26, 3, 7]));
        assert_eq!(parse("v1.26.3.00"), Some([1, 26, 3, 0]));
    }

    #[test]
    fn old_tags_are_not_this_scheme() {
        assert!(parse("0.2.33").is_none());
        assert!(parse("v1.6.12").is_none());
    }

    #[test]
    fn patch_is_the_last_component() {
        let now = parse("1.26.3.00").unwrap();
        let next = parse("1.26.3.01").unwrap();
        let shipped = parse("1.26.3.04").unwrap();
        let art = parse("1.26.3.07").unwrap();
        assert_eq!(cmp(now, next), std::cmp::Ordering::Less);
        assert_eq!(cmp(next, shipped), std::cmp::Ordering::Less);
        assert_eq!(cmp(shipped, art), std::cmp::Ordering::Less);
    }
}
