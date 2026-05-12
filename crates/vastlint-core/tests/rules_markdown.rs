use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use vastlint_core::{all_rules, Severity};

fn rules_markdown_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../RULES.md")
}

fn vscode_readme_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vscode/README.md")
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()))
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn parse_rule_id(cell: &str) -> Option<&str> {
    let (_, rest) = cell.split_once('[')?;
    let (rule_id, _) = rest.split_once(']')?;
    Some(rule_id)
}

fn parse_rule_rows(path: &Path) -> BTreeMap<String, (String, String)> {
    let text = read_text(path);
    let mut rows = BTreeMap::new();

    for line in text.lines() {
        if !line.starts_with("| [") {
            continue;
        }

        let parts: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
        assert!(
            parts.len() >= 3,
            "malformed rule row in {}: {}",
            path.display(),
            line
        );

        let rule_id = parse_rule_id(parts[0]).unwrap_or_else(|| {
            panic!("could not parse rule id from {} row: {}", path.display(), line)
        });

        rows.insert(
            rule_id.to_owned(),
            (parts[1].to_owned(), parts[2].to_owned()),
        );
    }

    rows
}

fn parse_vscode_readme_embedded_count() -> usize {
    let path = vscode_readme_path();
    let text = read_text(&path);

    text.lines()
        .find_map(|line| {
            let line = line.trim();
            let count = line.strip_prefix("<summary>All ")?;
            let count = count.strip_suffix(" rules</summary>")?;
            count.parse::<usize>().ok()
        })
        .unwrap_or_else(|| panic!("could not find embedded rule count in {}", path.display()))
}

fn catalog_severities() -> BTreeMap<String, String> {
    all_rules()
        .iter()
        .map(|rule| {
            (
                rule.id.to_owned(),
                severity_label(rule.default_severity).to_owned(),
            )
        })
        .collect()
}

fn assert_rule_rows_match_catalog(
    label: &str,
    declared_count: usize,
    rows: &BTreeMap<String, (String, String)>,
) {
    let catalog = catalog_severities();

    assert_eq!(
        declared_count,
        catalog.len(),
        "{label} declared count drifted from all_rules()"
    );
    assert_eq!(
        rows.len(),
        catalog.len(),
        "{label} row count drifted from all_rules()"
    );

    let missing: Vec<_> = catalog
        .keys()
        .filter(|rule_id| !rows.contains_key(*rule_id))
        .cloned()
        .collect();
    let extra: Vec<_> = rows
        .keys()
        .filter(|rule_id| !catalog.contains_key(*rule_id))
        .cloned()
        .collect();
    let severity_mismatches: Vec<_> = catalog
        .iter()
        .filter_map(|(rule_id, expected)| {
            rows.get(rule_id).and_then(|(actual, _)| {
                if actual == expected {
                    None
                } else {
                    Some((rule_id.clone(), expected.clone(), actual.clone()))
                }
            })
        })
        .collect();

    assert!(
        missing.is_empty() && extra.is_empty() && severity_mismatches.is_empty(),
        "{label} drifted from all_rules()\nmissing: {:?}\nextra: {:?}\nseverity mismatches: {:?}",
        missing,
        extra,
        severity_mismatches
    );
}

fn parse_rules_markdown() -> (usize, BTreeMap<String, (String, String)>) {
    let path = rules_markdown_path();
    let text = read_text(&path);

    let declared_count = text
        .lines()
        .find_map(|line| {
            if !line.contains(" rules across ") {
                return None;
            }

            line.split_whitespace().next()?.parse::<usize>().ok()
        })
        .unwrap_or_else(|| panic!("could not find declared rule count in {}", path.display()));

    (declared_count, parse_rule_rows(&path))
}

#[test]
fn rules_markdown_matches_catalog_ids_and_severities() {
    let (_, rows) = parse_rules_markdown();
    let (declared_count, _) = parse_rules_markdown();
    assert_rule_rows_match_catalog("RULES.md", declared_count, &rows);
}

#[test]
fn vscode_readme_rule_list_matches_catalog_ids_and_severities() {
    let path = vscode_readme_path();
    let rows = parse_rule_rows(&path);
    let declared_count = parse_vscode_readme_embedded_count();

    assert_rule_rows_match_catalog("vscode/README.md", declared_count, &rows);
}