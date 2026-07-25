//! A small glob matcher for the declared surface.
//!
//! Only what a surface glob needs: `*` (within one path segment), `**` (across
//! segments), `?`, and literal text. There is no brace expansion and no
//! character class, and a pattern using one is REFUSED rather than
//! half-understood, for the same reason VDS S-11(2) forbids a loader that skips
//! what it cannot parse: a surface glob that silently matches less than the
//! author meant makes every proof narrower than the author believes, and nothing
//! says so.

use std::path::{Path, PathBuf};

use vds_core::{Result, VdsError};
use walkdir::WalkDir;

/// Every file under `root` matching any of `patterns`.
pub fn match_globs(root: &Path, patterns: &[String]) -> Result<Vec<PathBuf>> {
    for pattern in patterns {
        check_pattern(pattern)?;
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored_dir(e.path()))
    {
        let entry = match entry {
            Ok(entry) => entry,
            // An unreadable directory is not a reason to report a smaller
            // surface than exists.
            Err(e) => {
                return Err(VdsError::precondition(format!(
                    "could not walk the project tree while matching the declared surface: {e}. \
                     A partial walk would report a surface smaller than the one that exists."
                )));
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = match entry.path().strip_prefix(root) {
            Ok(rest) => rest.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if patterns.iter().any(|p| matches(p, &relative)) {
            out.push(entry.path().to_path_buf());
        }
    }
    out.sort();
    Ok(out)
}

/// Directories no declared surface should reach into. Walking them is slow and
/// matching inside them is always a mistake.
fn is_ignored_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some(".git") | Some("node_modules") | Some("target") | Some(".next") | Some("dist")
    )
}

fn check_pattern(pattern: &str) -> Result<()> {
    if pattern.trim().is_empty() {
        return Err(VdsError::precondition(
            "a screen glob is empty. An empty pattern matches nothing and reads like a \
             pattern that matches everything.",
        ));
    }
    if pattern.starts_with('/') {
        return Err(VdsError::precondition(format!(
            "screen glob {pattern:?} is absolute. Every glob is relative to the project root, \
             or the declared surface stops meaning the same thing on another machine."
        )));
    }
    for unsupported in ['{', '}', '[', ']'] {
        if pattern.contains(unsupported) {
            return Err(VdsError::precondition(format!(
                "screen glob {pattern:?} uses {unsupported:?}, which this matcher does not \
                 implement. Refusing rather than matching less than you meant: a surface glob \
                 that silently matches too little makes every proof narrower than it looks."
            )));
        }
    }
    Ok(())
}

/// Match one `/`-separated relative path against one pattern.
pub fn matches(pattern: &str, path: &str) -> bool {
    let pattern_segments: Vec<&str> = pattern.split('/').collect();
    let path_segments: Vec<&str> = path.split('/').collect();
    match_segments(&pattern_segments, &path_segments)
}

fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.first() {
        None => path.is_empty(),
        Some(&"**") => {
            // `**` matches zero or more segments. Try every split point.
            (0..=path.len()).any(|skip| match_segments(&pattern[1..], &path[skip..]))
        }
        Some(segment) => match path.first() {
            None => false,
            Some(candidate) => {
                match_segment(segment, candidate) && match_segments(&pattern[1..], &path[1..])
            }
        },
    }
}

/// Match one segment, where `*` matches any run of characters within the
/// segment and `?` matches exactly one.
fn match_segment(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    // Classic two-pointer wildcard match with backtracking on the last `*`.
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_literal_pattern_matches_only_itself() {
        assert!(matches("app/page.tsx", "app/page.tsx"));
        assert!(!matches("app/page.tsx", "app/other.tsx"));
    }

    #[test]
    fn a_star_stays_inside_one_segment() {
        assert!(matches("app/*.tsx", "app/page.tsx"));
        assert!(
            !matches("app/*.tsx", "app/dash/page.tsx"),
            "a single star must not cross a path separator"
        );
    }

    #[test]
    fn a_double_star_crosses_segments_including_none() {
        assert!(matches("app/**/page.tsx", "app/page.tsx"));
        assert!(matches("app/**/page.tsx", "app/dash/page.tsx"));
        assert!(matches("app/**/page.tsx", "app/a/b/c/page.tsx"));
        assert!(!matches("app/**/page.tsx", "src/dash/page.tsx"));
    }

    #[test]
    fn a_question_mark_matches_exactly_one_character() {
        assert!(matches("app/pag?.tsx", "app/page.tsx"));
        assert!(!matches("app/pag?.tsx", "app/pa.tsx"));
    }

    #[test]
    fn a_leading_double_star_matches_at_any_depth() {
        assert!(matches("**/page.tsx", "page.tsx"));
        assert!(matches("**/page.tsx", "a/b/page.tsx"));
    }

    #[test]
    fn a_pattern_this_matcher_cannot_read_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        for bad in [
            "app/{a,b}/page.tsx",
            "app/[ab]/page.tsx",
            "/abs/page.tsx",
            "  ",
        ] {
            assert!(
                match_globs(tmp.path(), &[bad.to_string()]).is_err(),
                "should refuse {bad:?}"
            );
        }
    }

    #[test]
    fn walking_finds_matching_files_and_skips_ignored_directories() {
        let tmp = tempfile::tempdir().unwrap();
        for rel in [
            "app/page.tsx",
            "app/dash/page.tsx",
            "app/dash/layout.tsx",
            "node_modules/pkg/app/page.tsx",
            "target/app/page.tsx",
            ".git/app/page.tsx",
        ] {
            let path = tmp.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "x").unwrap();
        }
        let found = match_globs(tmp.path(), &["app/**/page.tsx".to_string()]).unwrap();
        let names: Vec<String> = found
            .iter()
            .map(|p| {
                p.strip_prefix(tmp.path())
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, vec!["app/dash/page.tsx", "app/page.tsx"]);
    }

    #[test]
    fn a_directory_matching_a_glob_is_not_a_screen() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("app/page.tsx")).unwrap();
        assert!(
            match_globs(tmp.path(), &["app/**/page.tsx".to_string()])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn two_globs_matching_one_file_yield_it_once() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("app/page.tsx");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "x").unwrap();
        let found = match_globs(
            tmp.path(),
            &["app/**/page.tsx".to_string(), "app/*.tsx".to_string()],
        )
        .unwrap();
        assert_eq!(found.len(), 1);
    }
}
