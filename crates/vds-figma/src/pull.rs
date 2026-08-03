//! Reading the decided-target Figma file into a ledger.
//!
//! The transport is a trait, and that is not incidental. VDS S-7(2)(1) forbids a
//! network call inside a proof, so the network lives here, out of band, behind an
//! interface a test can substitute. Three consequences follow, all wanted:
//!
//!   - the parsing is testable without a token or a connection;
//!   - a saved API response is a first-class input, so a pull is reproducible
//!     and an air-gapped build can still derive a ledger from bytes someone
//!     fetched once;
//!   - nothing in the proof path can accidentally acquire a network dependency,
//!     because nothing in the proof path can see this module's transport.

use std::collections::BTreeMap;
use std::path::Path;

#[cfg(test)]
use vds_core::ComponentId;
use vds_core::{Digest, Result, Timestamp, VdsError};
use vds_store::Store;

use crate::ledger::{
    FigmaLedger, FigmaNodeRow, GENERATOR_COMMAND, LEDGER_SCHEMA_VERSION, UnclaimedNode,
    states_from_variants,
};

/// Where the file's JSON comes from.
pub trait FigmaSource {
    /// The raw `GET /v1/files/:key` response body.
    fn fetch_file(&self, file_key: &str) -> Result<String>;
    /// The raw `GET /v1/files/:key/variables/local` response body.
    ///
    /// A separate endpoint, and the one the PIN generator reads. Variables carry
    /// a name and a value per mode, which is one-to-one with a CSS custom
    /// property carrying a value per theme; the file tree carries rendered
    /// instances, which is a different question with a different answer.
    fn fetch_variables(&self, file_key: &str) -> Result<String>;
    /// A sentence naming the source, recorded on the ledger.
    fn describe(&self) -> String;
}

/// Read a saved response from disk.
///
/// The reproducible path: `vds figma pull --from response.json` derives the same
/// ledger from the same bytes, forever, with no token and no network.
pub struct SavedResponse {
    pub path: std::path::PathBuf,
}

impl FigmaSource for SavedResponse {
    fn fetch_file(&self, _file_key: &str) -> Result<String> {
        std::fs::read_to_string(&self.path).map_err(|e| VdsError::io(self.path.display(), e))
    }
    fn fetch_variables(&self, _file_key: &str) -> Result<String> {
        std::fs::read_to_string(&self.path).map_err(|e| VdsError::io(self.path.display(), e))
    }
    fn describe(&self) -> String {
        format!("a saved API response at {}", self.path.display())
    }
}

/// Fetch over HTTPS by shelling out to `curl`.
///
/// `curl` rather than an HTTP crate, for the same reason the rest of VDS takes
/// as few dependencies as it can: this is a governance tool, every dependency is
/// a supply-chain surface nobody reviewed, and the one network call it makes does
/// not justify pulling in a TLS stack. If `curl` is absent the error says so and
/// points at `--from`, which needs nothing.
pub struct FigmaApi {
    pub token: String,
}

/// How long `curl` may take before the pull is abandoned.
///
/// Sixty seconds cannot fetch a real design file. Measured 2026-08-03: the decided
/// target's 136 MB document took 312 s on a gigabit line, so a 60 s ceiling did not
/// slow that pull down, it made it impossible. Ten minutes is chosen to be longer
/// than any observed pull rather than to be tight, because the failure this bounds
/// is a hung connection, and a hung connection is not distinguishable from a slow
/// one in under a minute.
const MAX_TIME_SECONDS: &str = "600";

/// Whether a body stops before the JSON document it claims to be does.
///
/// `GET /v1/files/:key` on a large file answers **HTTP 200, sends no
/// `content-length`** because the response is chunked, and then stops mid-string.
/// `curl --fail` cannot see it: the status line said 200 and the stream ended, so
/// curl exits 0 and hands back a body that is simply short.
///
/// Measured 2026-08-03 on one 136,114,759 B file, fetched twice: 102,744,823 B and
/// 62,846,637 B arrived, both HTTP 200, neither valid JSON, truncated at two
/// DIFFERENT points. That last detail is the dangerous one. Comparing two such
/// bodies showed a 38 MB difference between two files whose documents are
/// byte-identical, and every count taken from either would have been short without
/// anything anywhere reporting an error.
///
/// The check is the cheapest one that is exact: a whole JSON document ends with its
/// closing brace, and a body cut mid-string cannot. It costs two trims on a string
/// already in memory, so it can run on every fetch rather than only when something
/// downstream already looks wrong.
///
/// It requires the OPENING brace as well, and that is not decoration. A rate-limit
/// page or a proxy's error HTML also fails to end in a brace, and calling that
/// "truncated" would send the reader to chunk a request that was never answered
/// with a document at all. This predicate says one thing: a JSON object began and
/// did not finish.
fn looks_truncated(body: &str) -> bool {
    let body = body.trim();
    body.starts_with('{') && !body.ends_with('}')
}

/// The error for a body that arrived short.
fn truncated_error(body: &str, what: &str) -> VdsError {
    VdsError::precondition(format!(
        "the Figma response for {what} is truncated: {} bytes that do not end in a closing \
         brace.\n  \
         The request SUCCEEDED. `GET /v1/files/:key` on a large file answers HTTP 200, sends no \
         content-length because the response is chunked, and then stops mid-string, so neither \
         the status code nor curl's exit code can see it.\n  \
         Do not retry the same call and do not compare this body with another one: two truncated \
         pulls of one file stop at different points, so they differ from each other while the \
         file does not.\n  \
         Fetch it in pieces instead: GET /v1/files/:key?depth=2 for the page shells, then GET \
         /v1/files/:key/nodes?ids=<id> once per top-level frame, parsing each response before \
         going on, then derive from the reassembled document with: vds figma pull --from <file>",
        body.len(),
        what = what
    ))
}

impl FigmaApi {
    /// Read the token from the environment.
    ///
    /// Never from `.vds/`. A token in a committed governance record is a
    /// credential in version control, and `no_stored_values` would not catch it
    /// because a token is not a design value.
    pub fn from_env() -> Result<FigmaApi> {
        let token = std::env::var("FIGMA_TOKEN")
            .or_else(|_| std::env::var("FIGMA_ACCESS_TOKEN"))
            .map_err(|_| {
                VdsError::precondition(
                    "no Figma token. Set FIGMA_TOKEN in the environment.\n  \
                     It is deliberately not read from .vds/: a token in a committed governance \
                     record is a credential in version control, and no_stored_values would not \
                     catch it, because a token is not a design value.\n  \
                     To derive a ledger without a token, save a response and use: \
                     vds figma pull --from response.json",
                )
            })?;
        if token.trim().is_empty() {
            return Err(VdsError::precondition("FIGMA_TOKEN is set and empty"));
        }
        Ok(FigmaApi { token })
    }
}

impl FigmaApi {
    fn get(&self, url: &str, what: &str) -> Result<String> {
        let output = std::process::Command::new("curl")
            .arg("--silent")
            .arg("--show-error")
            .arg("--fail")
            .arg("--max-time")
            .arg(MAX_TIME_SECONDS)
            .arg("--header")
            .arg(format!("X-Figma-Token: {}", self.token))
            .arg(url)
            .output()
            .map_err(|e| {
                VdsError::precondition(format!(
                    "could not run curl to reach the Figma API: {e}\n  \
                     Install curl, or work from a saved response, which needs neither a token \
                     nor a network."
                ))
            })?;
        if !output.status.success() {
            return Err(VdsError::precondition(format!(
                "the Figma API refused the request for {what}: {}\n  \
                 Check FIGMA_TOKEN and that the token can read this file. The variables \
                 endpoint additionally needs an Enterprise plan and the `file_variables:read` \
                 scope, and returns 403 rather than an empty result without them.",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let body = String::from_utf8(output.stdout).map_err(|e| {
            VdsError::precondition(format!("the Figma API returned invalid UTF-8: {e}"))
        })?;
        if looks_truncated(&body) {
            return Err(truncated_error(&body, what));
        }
        Ok(body)
    }
}

impl FigmaSource for FigmaApi {
    fn fetch_variables(&self, file_key: &str) -> Result<String> {
        self.get(
            &format!("https://api.figma.com/v1/files/{file_key}/variables/local"),
            &format!("the local variables of file {file_key}"),
        )
    }

    fn fetch_file(&self, file_key: &str) -> Result<String> {
        // Deliberately the same call path as the variables endpoint. This used to be
        // its own copy of the curl invocation, and the copy is how it came to carry
        // its own timeout: the two drifted, and the truncation check has to hold on
        // the file endpoint above all, because that is the response large enough to
        // be cut.
        self.get(
            &format!("https://api.figma.com/v1/files/{file_key}"),
            &format!("file {file_key}"),
        )
    }

    fn describe(&self) -> String {
        "the Figma REST API".to_owned()
    }
}

/// Build the ledger from a file response and the register.
pub fn build_ledger(
    store: &Store,
    file_key: &str,
    body: &str,
    source_description: &str,
) -> Result<FigmaLedger> {
    let document: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        // Truncation and corruption both land here and they are not the same problem.
        // A body that ends early is a TRANSPORT failure that a retry may fix and that
        // says nothing about the file; a body that is malformed part-way through is
        // not. serde_json classifies the first as Eof, so say which one happened
        // rather than making the reader guess from a byte offset.
        if e.classify() == serde_json::error::Category::Eof || looks_truncated(body) {
            return truncated_error(body, &format!("file {file_key}"));
        }
        VdsError::precondition(format!(
            "the Figma response is not JSON: {e}. A partial parse would produce a ledger \
             claiming fewer nodes than the file has, and every proof reading it would be \
             narrower than it looks."
        ))
    })?;

    let file_version = document
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned();
    let file_name = document
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned();
    if file_version.is_empty() {
        return Err(VdsError::precondition(
            "the Figma response carries no `version`. That field is the decided-target file's \
             own identifier for its current state, and without it the ledger cannot say WHICH \
             version of what is decided it recorded.",
        ));
    }

    // Index every component set in the document by node id, once.
    let mut sets: BTreeMap<String, ComponentSet> = BTreeMap::new();
    if let Some(root) = document.get("document") {
        walk(root, &mut sets);
    }
    // `componentSets` at the top level carries the human-facing names, which the
    // document tree does not always repeat.
    if let Some(map) = document.get("componentSets").and_then(|v| v.as_object()) {
        for (node_id, value) in map {
            let entry = sets.entry(normalise_node_id(node_id)).or_default();
            if entry.name.is_empty()
                && let Some(name) = value.get("name").and_then(|v| v.as_str())
            {
                entry.name = name.to_owned();
            }
            entry.is_set = true;
        }
    }

    let records = store.read_register()?;
    let mut nodes = Vec::new();
    let mut claimed: Vec<String> = Vec::new();
    let mut unmapped_values = 0usize;

    for located in &records {
        let record = &located.value;
        let Some(figma) = &record.figma else {
            continue;
        };
        if figma.file_key != file_key {
            nodes.push(FigmaNodeRow {
                component_id: record.id.clone(),
                node_id: figma.node_id.clone(),
                resolved: false,
                figma_name: None,
                is_component_set: false,
                variant_properties: BTreeMap::new(),
                states_drawn: Vec::new(),
                unresolved_because: Some(format!(
                    "the record names file {:?} and this pull read file {file_key:?}",
                    figma.file_key
                )),
            });
            continue;
        }

        let key = normalise_node_id(&figma.node_id);
        claimed.push(key.clone());
        match sets.get(&key) {
            None => nodes.push(FigmaNodeRow {
                component_id: record.id.clone(),
                node_id: figma.node_id.clone(),
                resolved: false,
                figma_name: None,
                is_component_set: false,
                variant_properties: BTreeMap::new(),
                states_drawn: Vec::new(),
                unresolved_because: Some(format!(
                    "no node {} exists in file {file_key}",
                    figma.node_id
                )),
            }),
            Some(found) => {
                let (states_drawn, unmapped) = states_from_variants(&found.variants);
                unmapped_values += unmapped;
                nodes.push(FigmaNodeRow {
                    component_id: record.id.clone(),
                    node_id: figma.node_id.clone(),
                    resolved: true,
                    figma_name: Some(found.name.clone()),
                    is_component_set: found.is_set,
                    variant_properties: found.variants.clone(),
                    states_drawn,
                    unresolved_because: None,
                });
            }
        }
    }
    nodes.sort_by(|a, b| a.component_id.cmp(&b.component_id));

    let mut unclaimed: Vec<UnclaimedNode> = sets
        .iter()
        .filter(|(node_id, set)| set.is_set && !claimed.contains(node_id))
        .map(|(node_id, set)| UnclaimedNode {
            node_id: node_id.clone(),
            figma_name: set.name.clone(),
            is_component_set: set.is_set,
        })
        .collect();
    unclaimed.sort_by(|a, b| a.node_id.cmp(&b.node_id));

    let mut notes = vec![
        format!("pulled from {source_description}"),
        "This ledger records names, node ids, variant property names and variant values. It \
         records no colour, length, font, duration or easing curve: those stay in the Figma \
         file, which [2026] VJS-CC-OPBOX 3 D1 makes the system of record for what is decided."
            .to_owned(),
    ];
    if unmapped_values > 0 {
        notes.push(format!(
            "{unmapped_values} variant values on state properties did not map onto one of the \
             nine states (VDS S-5(3)), so they contribute no drawn state. They are recorded in \
             `variant_properties` verbatim. Guessing that a synonym means a state would let \
             this ledger claim a state is drawn on a word VDS invented."
        ));
    }
    if !unclaimed.is_empty() {
        notes.push(format!(
            "{} component sets in this file are claimed by no register record. A component \
             drawn in the decided-target file and absent from the register is one design has \
             committed to and governance has never seen (VDS S-5(6)).",
            unclaimed.len()
        ));
    }

    let mut ledger = FigmaLedger {
        schema_version: LEDGER_SCHEMA_VERSION,
        generated_at: Timestamp::now(),
        generated_by: GENERATOR_COMMAND.to_owned(),
        file_key: file_key.to_owned(),
        file_version,
        file_name,
        content_digest: Digest::of_text("placeholder"),
        nodes,
        unclaimed,
        notes,
    };
    ledger.content_digest = ledger.compute_content_digest()?;
    Ok(ledger)
}

#[derive(Default)]
struct ComponentSet {
    name: String,
    is_set: bool,
    variants: BTreeMap<String, Vec<String>>,
}

/// Figma writes a node id as `12:34` in a file URL and `12-34` in a deep link.
/// They are the same node, and a ledger that treated them as different would
/// report a node unresolved because the designer copied the other spelling.
fn normalise_node_id(raw: &str) -> String {
    raw.replace('-', ":")
}

fn walk(node: &serde_json::Value, out: &mut BTreeMap<String, ComponentSet>) {
    if let Some(id) = node.get("id").and_then(|v| v.as_str()) {
        let node_type = node
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let is_set = node_type == "COMPONENT_SET";
        if is_set || node_type == "COMPONENT" {
            let entry = out.entry(normalise_node_id(id)).or_default();
            entry.is_set |= is_set;
            if entry.name.is_empty()
                && let Some(name) = node.get("name").and_then(|v| v.as_str())
            {
                entry.name = name.to_owned();
            }
            if is_set {
                collect_variants(node, entry);
            }
        }
    }
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child in children {
            walk(child, out);
        }
    }
}

/// Variant properties, from either shape Figma uses.
///
/// A component set may declare `componentPropertyDefinitions`, and its children
/// carry names like `State=Hover, Size=Large`. Both are read, because a file
/// authored in an older Figma has only the second.
fn collect_variants(node: &serde_json::Value, entry: &mut ComponentSet) {
    if let Some(definitions) = node
        .get("componentPropertyDefinitions")
        .and_then(|v| v.as_object())
    {
        for (property, definition) in definitions {
            if let Some(options) = definition.get("variantOptions").and_then(|v| v.as_array()) {
                let values: Vec<String> = options
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_owned())
                    .collect();
                if !values.is_empty() {
                    entry
                        .variants
                        .entry(strip_property_suffix(property))
                        .or_default()
                        .extend(values);
                }
            }
        }
    }
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child in children {
            let Some(name) = child.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            for pair in name.split(',') {
                if let Some((property, value)) = pair.split_once('=') {
                    let values = entry
                        .variants
                        .entry(strip_property_suffix(property.trim()))
                        .or_default();
                    let value = value.trim().to_owned();
                    if !values.contains(&value) {
                        values.push(value);
                    }
                }
            }
        }
    }
    for values in entry.variants.values_mut() {
        values.sort();
        values.dedup();
    }
}

/// Figma appends `#1:2` to a component property name internally.
fn strip_property_suffix(property: &str) -> String {
    property
        .split_once('#')
        .map(|(head, _)| head)
        .unwrap_or(property)
        .trim()
        .to_owned()
}

/// Where the ledger is written.
pub fn ledger_path(store: &Store) -> std::path::PathBuf {
    store
        .project
        .path(vds_core::PathRole::Ledgers)
        .join("figma.yaml")
}

pub fn write(store: &Store, ledger: &FigmaLedger) -> Result<std::path::PathBuf> {
    let path = ledger_path(store);
    let text = serde_yaml::to_string(ledger).map_err(|e| VdsError::Serialize {
        what: "the figma ledger".into(),
        message: e.to_string(),
    })?;
    vds_core::write_text_atomically(&path, &text)?;
    Ok(path)
}

pub fn read(store: &Store) -> Result<Option<FigmaLedger>> {
    let path = ledger_path(store);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| VdsError::io(path.display(), e))?;
    let ledger: FigmaLedger = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: store.project.rel(&path),
        message: format!("is not a figma ledger: {e}"),
    })?;
    Ok(Some(ledger))
}

/// The decided-target file every record agrees on, or an explanation.
pub fn declared_file_key(store: &Store) -> Result<Option<String>> {
    let records = store.read_register()?;
    let mut keys: Vec<String> = records
        .iter()
        .filter_map(|r| r.value.figma.as_ref().map(|f| f.file_key.clone()))
        .collect();
    keys.sort();
    keys.dedup();
    match keys.len() {
        0 => Ok(None),
        1 => Ok(Some(keys.remove(0))),
        _ => Err(VdsError::precondition(format!(
            "the register names {} different Figma files: {}.\n  \
             [2026] VJS-CC-OPBOX 3 D1 names ONE decided-target file as the system of record \
             for what is decided, and two of them is two opinions about what is decided. Pass \
             --file-key to say which, or amend the records that disagree.",
            keys.len(),
            keys.join(", ")
        ))),
    }
}

/// Read a saved response and build a ledger from it.
pub fn from_saved(store: &Store, file_key: &str, path: &Path) -> Result<FigmaLedger> {
    let source = SavedResponse {
        path: path.to_path_buf(),
    };
    let body = source.fetch_file(file_key)?;
    build_ledger(store, file_key, &body, &source.describe())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;
    use vds_core::{State, Status};

    /// A minimal file response: one component set with a State variant, one
    /// nobody claims.
    fn response() -> String {
        serde_json::json!({
            "name": "Decided target",
            "version": "998877",
            "document": {
                "id": "0:0",
                "type": "DOCUMENT",
                "children": [{
                    "id": "1:1",
                    "type": "CANVAS",
                    "children": [
                        {
                            "id": "12:34",
                            "type": "COMPONENT_SET",
                            "name": "Button",
                            "children": [
                                {"id": "12:35", "type": "COMPONENT", "name": "State=Default, Size=Medium"},
                                {"id": "12:36", "type": "COMPONENT", "name": "State=Hover, Size=Medium"}
                            ]
                        },
                        {
                            "id": "99:99",
                            "type": "COMPONENT_SET",
                            "name": "Undeclared Thing",
                            "children": []
                        }
                    ]
                }]
            }
        })
        .to_string()
    }

    fn with_node(f: &Fixture, name: &str, node_id: &str) -> ComponentId {
        let id = f.register(name, Status::Registered);
        f.amend(&id, |r| {
            r.figma = Some(vds_core::FigmaNode {
                file_key: "KEY".into(),
                node_id: node_id.into(),
                captured_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            });
        });
        id
    }

    #[test]
    fn a_resolved_node_records_its_variants_and_derived_states() {
        let f = Fixture::new();
        let id = with_node(&f, "Button", "12:34");
        let store = f.store();
        let ledger = build_ledger(&store, "KEY", &response(), "a test").unwrap();
        let row = ledger.row(&id).unwrap();
        assert!(row.resolved);
        assert_eq!(row.figma_name.as_deref(), Some("Button"));
        assert!(row.is_component_set);
        assert_eq!(row.states_drawn, vec![State::Default, State::Hover]);
        assert_eq!(
            row.variant_properties.get("Size").map(|v| v.as_slice()),
            Some(["Medium".to_string()].as_slice()),
            "a non-state property is recorded and contributes no state"
        );
    }

    #[test]
    fn a_node_that_is_not_there_is_unresolved_and_says_so() {
        let f = Fixture::new();
        let id = with_node(&f, "Ghost", "77:77");
        let store = f.store();
        let ledger = build_ledger(&store, "KEY", &response(), "a test").unwrap();
        let row = ledger.row(&id).unwrap();
        assert!(!row.resolved);
        assert!(row.unresolved_because.as_deref().unwrap().contains("77:77"));
    }

    #[test]
    fn the_two_node_id_spellings_are_the_same_node() {
        let f = Fixture::new();
        let id = with_node(&f, "Button", "12-34");
        let store = f.store();
        let ledger = build_ledger(&store, "KEY", &response(), "a test").unwrap();
        assert!(
            ledger.row(&id).unwrap().resolved,
            "12-34 from a deep link and 12:34 from a file URL are the same node, and treating \
             them as different reports a node unresolved because of how it was copied"
        );
    }

    #[test]
    fn a_component_set_no_record_claims_is_reported() {
        let f = Fixture::new();
        with_node(&f, "Button", "12:34");
        let store = f.store();
        let ledger = build_ledger(&store, "KEY", &response(), "a test").unwrap();
        assert_eq!(ledger.unclaimed.len(), 1);
        assert_eq!(ledger.unclaimed[0].figma_name, "Undeclared Thing");
        assert!(
            ledger
                .notes
                .iter()
                .any(|n| n.contains("governance has never seen"))
        );
    }

    #[test]
    fn a_record_naming_another_file_is_unresolved_rather_than_silently_skipped() {
        let f = Fixture::new();
        let id = f.register("Elsewhere", Status::Registered);
        f.amend(&id, |r| {
            r.figma = Some(vds_core::FigmaNode {
                file_key: "OTHER".into(),
                node_id: "12:34".into(),
                captured_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            });
        });
        let store = f.store();
        let ledger = build_ledger(&store, "KEY", &response(), "a test").unwrap();
        let row = ledger.row(&id).unwrap();
        assert!(!row.resolved);
        assert!(row.unresolved_because.as_deref().unwrap().contains("OTHER"));
    }

    #[test]
    fn a_response_with_no_version_is_refused() {
        let f = Fixture::new();
        let store = f.store();
        let body = serde_json::json!({"name": "x", "document": {"id": "0:0"}}).to_string();
        let error = build_ledger(&store, "KEY", &body, "a test").unwrap_err();
        assert!(error.to_string().contains("WHICH version"), "{error}");
    }

    #[test]
    fn a_non_json_response_is_refused_rather_than_partially_parsed() {
        let f = Fixture::new();
        let store = f.store();
        let error = build_ledger(&store, "KEY", "<html>rate limited</html>", "a test").unwrap_err();
        assert!(
            error.to_string().contains("narrower than it looks"),
            "{error}"
        );
    }

    /// The real shape of the 2026-08-03 failure: a whole response cut at a byte
    /// boundary inside a string, which is what `GET /v1/files/:key` hands back on a
    /// large file under an HTTP 200 with no content-length.
    #[test]
    fn a_response_cut_mid_string_is_named_as_truncated_not_as_malformed() {
        let f = Fixture::new();
        let store = f.store();
        let whole = response();
        let cut = &whole[..whole.len() - 40];
        let error = build_ledger(&store, "KEY", cut, "a test").unwrap_err();
        let text = error.to_string();
        assert!(text.contains("truncated"), "{error}");
        assert!(text.contains("The request SUCCEEDED"), "{error}");
        // The route out has to be in the message. A reader who is told only that the
        // body is short will retry the identical call, which is the one thing that
        // reproduces it.
        assert!(text.contains("/nodes?ids="), "{error}");
    }

    /// The negative control. Without this the truncation test passes just as well
    /// against a predicate that calls every response truncated.
    #[test]
    fn a_whole_response_is_not_called_truncated() {
        assert!(!looks_truncated(&response()));
        assert!(looks_truncated(&response()[..response().len() - 40]));
        // An error page is not a truncated document, and must not be sent down the
        // chunk-the-request route.
        assert!(!looks_truncated("<html>rate limited</html>"));
        assert!(!looks_truncated(""));
    }

    /// Two truncated pulls of ONE file stop at different points, so they disagree
    /// with each other while the file does not. This is the property that made the
    /// original failure dangerous rather than merely annoying, and it is why the
    /// refusal has to happen before anything compares two bodies.
    #[test]
    fn two_truncations_of_one_response_differ_from_each_other() {
        let f = Fixture::new();
        let store = f.store();
        let whole = response();
        let short = &whole[..whole.len() - 200];
        let shorter = &whole[..whole.len() - 400];
        assert_ne!(short.len(), shorter.len());
        for body in [short, shorter] {
            let error = build_ledger(&store, "KEY", body, "a test").unwrap_err();
            assert!(error.to_string().contains("truncated"), "{error}");
        }
    }

    #[test]
    fn two_pulls_of_one_response_produce_one_content_digest() {
        let f = Fixture::new();
        with_node(&f, "Button", "12:34");
        let store = f.store();
        let a = build_ledger(&store, "KEY", &response(), "a test").unwrap();
        let b = build_ledger(&store, "KEY", &response(), "a test").unwrap();
        assert_eq!(a.content_digest, b.content_digest);
    }

    #[test]
    fn a_register_naming_two_files_is_refused() {
        let f = Fixture::new();
        with_node(&f, "A", "1:1");
        let second = f.register("B", Status::Registered);
        f.amend(&second, |r| {
            r.figma = Some(vds_core::FigmaNode {
                file_key: "OTHER".into(),
                node_id: "2:2".into(),
                captured_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            });
        });
        let store = f.store();
        let error = declared_file_key(&store).unwrap_err();
        assert!(error.to_string().contains("two opinions"), "{error}");
    }

    #[test]
    fn a_saved_response_needs_no_token_and_no_network() {
        let f = Fixture::new();
        let id = with_node(&f, "Button", "12:34");
        let path = f.root().join("figma-response.json");
        std::fs::write(&path, response()).unwrap();
        let store = f.store();
        let ledger = from_saved(&store, "KEY", &path).unwrap();
        assert!(ledger.row(&id).unwrap().resolved);
        assert!(ledger.notes[0].contains("saved API response"));
    }
}
