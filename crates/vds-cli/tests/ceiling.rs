//! `docs/CEILING.md`, re-derived.
//!
//! That file records which `vds doctor` criteria cannot be closed from inside
//! this repository, so nobody spends a session grinding at one that is waiting
//! on a billing setting or a court grant.
//!
//! It is also the most dangerous kind of document here. A written account of
//! what cannot be done becomes, quickly, a written account of what nobody
//! tried. So every claim is checked below, and **each check fails when the
//! blocker CLEARS** - the message says the ceiling has lifted and the doc must
//! be rewritten. A ceiling that stays true after the ceiling lifts is an excuse
//! with a test next to it, which is worse than prose, because it looks verified.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(root().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

#[test]
fn the_ceiling_document_exists_and_names_its_own_expiry() {
    let doc = read("docs/CEILING.md");
    assert!(
        doc.contains("crates/vds-cli/tests/ceiling.rs"),
        "CEILING.md must name the test that re-derives it, or it is prose again"
    );
    assert!(
        doc.contains("FAILS when a blocker"),
        "CEILING.md must say that its checks expire, because that is the only thing \
         separating it from a list of excuses"
    );
}

/// D1/D2/D3 here: the declared surface matches nothing, and the config says so.
#[test]
fn this_repository_still_ships_no_screens() {
    let config = read(".vds/config.toml");
    assert!(
        config.contains("VDS ships no screens"),
        "the config no longer says this repository ships no screens. If a design \
         surface has been added, D1/D2/D3 are no longer vacuous by design and \
         docs/CEILING.md is wrong about the first three rows."
    );
    // And the surface really is empty, not merely described as empty. A ledger
    // is generated, so this reads the artefact rather than the prose beside it.
    let ledger = read(".vds/ledgers/screens.yaml");
    assert!(
        ledger.contains("screens: []") || ledger.contains("screens:\n"),
        "the screens ledger no longer looks empty; re-read CEILING.md's first section"
    );
}

/// D4: the workflow has never once succeeded, and the reason is billing.
#[test]
fn the_ci_workflow_has_still_never_succeeded() {
    let ledger = read(".vds/ledgers/ci.yaml");
    // `successes: 0` is the whole claim. The moment it is non-zero the ceiling
    // has lifted and D4 is reachable by work rather than by a payment.
    assert!(
        ledger.contains("successes: 0"),
        "THE CI WORKFLOW HAS SUCCEEDED. That is good news and it retires a section of \
         docs/CEILING.md: D4 is no longer blocked on GitHub billing. Re-read the D4 \
         section, re-run `vds doctor`, and rewrite the ceiling rather than relaxing \
         this assertion.\n{ledger}"
    );
    // The ledger must be a real reading and not an empty one, or the assertion
    // above passes over a file that measured nothing.
    assert!(
        ledger.contains("runs_concluded:") && !ledger.contains("runs_concluded: 0"),
        "the CI ledger records no CONCLUDED runs, so `successes: 0` proves nothing - \
         it is the shape of a window with nothing in it. Re-run `vds ledger ci`."
    );
}

/// D6: warrants are not VDS's to grant, and the tool says so itself.
#[test]
fn vds_still_grants_no_warrants() {
    // Read from the SOURCE of the message rather than running the binary, so
    // this test does not need a build. The sentence is load-bearing: it is the
    // whole basis for D6 being outside this repository's reach.
    let warrant = read("crates/vds-cli/src/warrant.rs");
    assert!(
        warrant.contains("VDS grants nothing"),
        "`vds warrant status` no longer opens by saying VDS grants nothing. If VDS has \
         acquired a granting power, D6 may be reachable from here and docs/CEILING.md \
         is wrong - and a granting power in VDS would itself want a submission."
    );
    assert!(
        warrant.contains("This does NOT grant") || warrant.contains("does NOT grant"),
        "`vds warrant record` no longer disclaims granting"
    );

    // No warrant record may appear without a real grant behind it. This is the
    // direction that matters: fabricating one would put a false record in the
    // governance chain, and it would make D6 go green for the worst reason.
    let dir = root().join(".vds/warrants");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let records: Vec<_> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "yaml"))
            .collect();
        assert!(
            records.is_empty(),
            "{} warrant record(s) exist. VDS grants nothing, so each must correspond to \
             a grant made by VJS (W1, W2, W4) or the Principal (W3), recorded with \
             `vds warrant record`. Check the grant is real, then update docs/CEILING.md \
             because D6 has started moving.",
            records.len()
        );
    }
}

/// D10 on the worked example: no designpack, because no assent event.
#[test]
fn no_designpack_is_vendored_because_no_assent_has_occurred() {
    let assent = root().join("designpack/v1/provenance/assent");
    let has_assent = std::fs::read_dir(&assent)
        .map(|d| d.flatten().count() > 0)
        .unwrap_or(false);
    assert!(
        !has_assent,
        "AN ASSENT EVENT EXISTS in designpack/v1/provenance/assent/. The specification \
         has commenced, so warrants can be granted under it and D10 on the worked \
         example is reachable. Rewrite docs/CEILING.md's last two sections rather than \
         deleting this test."
    );

    let doc = read("docs/CEILING.md");
    assert!(
        doc.contains("assent event"),
        "CEILING.md no longer explains that D10 waits on an assent event"
    );
}

/// The negative control for the whole file.
///
/// Every test above asserts that something is still absent or still zero, and
/// every one of them would pass if `docs/CEILING.md` were deleted and the
/// project abandoned. This is the assertion that fails in that case.
#[test]
fn the_ceiling_is_measured_against_a_project_that_still_works() {
    let doc = read("docs/CEILING.md");
    for criterion in ["D1", "D4", "D6", "D10"] {
        assert!(
            doc.contains(criterion),
            "CEILING.md no longer accounts for {criterion}"
        );
    }
    // The doc claims six criteria are MET here. If that ever drops, something
    // has REGRESSED and the ceiling is no longer the reason for the score.
    let lock = read(".vds/enforcement.lock");
    assert!(
        lock.contains("entries:") && lock.matches("\n- path:").count() >= 20,
        "the enforcement lock has shrunk below twenty entries, so D5 being MET is not \
         the fact CEILING.md records it as"
    );
}

/// No filed submission may carry a placeholder case-file digest.
///
/// SIX OF TEN DID, from filing until 2026-07-31. Sixty-four zeros reads exactly
/// like a computed digest, so six pending questions were awaiting a ruling on a
/// record that could move under them with nothing saying so - the defect the
/// geometry reading carries a digest to prevent, in the artefact whose entire
/// job is to fix a record for a court.
///
/// Lives here rather than beside the submission code because it is the same
/// class as the ceiling: a field that LOOKS measured and is not.
#[test]
fn no_filed_submission_carries_a_placeholder_digest() {
    let dir = root().join(".vds/submissions/filed");
    let mut checked = 0usize;
    let mut placeholders = Vec::new();
    let mut unpinned = Vec::new();

    for entry in std::fs::read_dir(&dir)
        .expect("the filed submissions directory")
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "yaml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable");
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_owned();
        let Some(line) = text.lines().find(|l| l.starts_with("case_file_digest:")) else {
            unpinned.push(name);
            continue;
        };
        checked += 1;
        let value = line.trim_start_matches("case_file_digest:").trim();
        // A placeholder is any digest that is all one character after the prefix.
        let hex = value.trim_start_matches("sha256:");
        if hex.len() < 64 || hex.chars().all(|c| c == '0') {
            placeholders.push(format!("{name}: {value}"));
        }
    }

    assert!(
        unpinned.is_empty(),
        "filed submission(s) with NO case_file_digest field at all: {unpinned:?}"
    );
    assert!(
        placeholders.is_empty(),
        "filed submission(s) carrying a placeholder digest, so the bench would rule on a \
         record that can move under it:\n  {}\nDerive one: scripts/case-file-digest.sh <the \
         evidence files>",
        placeholders.join("\n  ")
    );
    // The negative control: an empty directory would satisfy both assertions.
    assert!(
        checked >= 8,
        "only {checked} filed submissions were checked; the assertions above would pass \
         over nearly nothing"
    );
}
