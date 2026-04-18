use std::collections::HashMap;
use std::io::Read;
use std::process::ExitCode;

use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use anstream::eprintln;
use anstyle::{AnsiColor, Color, Style};
use clap::{Parser, Subcommand, ValueEnum};
use vastlint_core::{
    fix_with_context, validate_with_context, FixResult, Issue, RuleLevel, Severity,
    ValidationContext, ValidationResult,
};

mod telemetry;

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "vastlint",
    about = "VAST XML validator — checks tags against the IAB spec",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate one or more VAST XML files (use - for stdin)
    Check {
        /// Files to validate. Pass - to read from stdin.
        #[arg(required = true, num_args = 1..)]
        files: Vec<String>,

        /// Output format
        #[arg(long, default_value = "plain")]
        format: Format,

        /// Disable colour output
        #[arg(long)]
        no_color: bool,

        /// Exit 0 even when errors are found (useful in some CI setups)
        #[arg(long)]
        no_fail: bool,

        /// Path to a vastlint.toml config file. Defaults to searching up from CWD.
        #[arg(long, value_name = "PATH")]
        config: Option<String>,

        /// Skip config file loading entirely
        #[arg(long)]
        no_config: bool,

        /// Send an anonymous usage ping (version, OS, install ID, file count).
        /// Off by default. See README for what is sent.
        #[arg(long)]
        telemetry: bool,
    },
    /// List all known rule IDs with their default severity
    Rules,
    /// Automatically fix common VAST issues and write repaired XML
    Fix {
        /// File to fix. Pass - to read from stdin and write repaired XML to stdout.
        #[arg(required = true)]
        file: String,

        /// Write repaired XML to this path instead of overwriting the input file.
        /// When reading from stdin, output always goes to stdout regardless of this flag.
        #[arg(long, value_name = "PATH")]
        out: Option<String>,

        /// Show what would be changed without writing any files.
        #[arg(long)]
        dry_run: bool,

        /// Output format for the fix report
        #[arg(long, default_value = "plain")]
        format: Format,

        /// Disable colour output
        #[arg(long)]
        no_color: bool,

        /// Path to a vastlint.toml config file. Defaults to searching up from CWD.
        #[arg(long, value_name = "PATH")]
        config: Option<String>,

        /// Skip config file loading entirely
        #[arg(long)]
        no_config: bool,
    },
}

#[derive(ValueEnum, Clone)]
enum Format {
    Plain,
    Json,
}

// ── exit codes ────────────────────────────────────────────────────────────────

/// Exit 0: all files valid (no errors).
/// Exit 1: at least one file had validation errors.
/// Exit 2: CLI usage error (unreadable file, bad config).
const EXIT_VALIDATION_ERROR: u8 = 1;
const EXIT_USAGE_ERROR: u8 = 2;

// ── entry point ───────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Check {
            files,
            format,
            no_color,
            no_fail,
            config,
            no_config,
            telemetry,
        } => {
            if no_color {
                std::env::set_var("NO_COLOR", "1");
            }
            run_check(files, format, no_fail, config, no_config, telemetry)
        }
        Command::Rules => {
            run_rules();
            ExitCode::SUCCESS
        }
        Command::Fix {
            file,
            out,
            dry_run,
            format,
            no_color,
            config,
            no_config,
        } => {
            if no_color {
                std::env::set_var("NO_COLOR", "1");
            }
            run_fix(file, out, dry_run, format, config, no_config)
        }
    }
}

// ── config loading ────────────────────────────────────────────────────────────

/// Parsed configuration from a vastlint.toml file.
struct Config {
    rule_overrides: Option<HashMap<&'static str, RuleLevel>>,
    /// Whether the user has opted in to telemetry via the config file.
    telemetry: bool,
}

/// Load rule overrides from a config file.
///
/// Returns None when no config file is found or loading is disabled.
/// Returns an error string when a config file was found but could not be parsed.
fn load_config(config_path: Option<String>, no_config: bool) -> Result<Option<Config>, String> {
    if no_config {
        return Ok(None);
    }

    let path = match config_path {
        Some(p) => std::path::PathBuf::from(p),
        None => match find_config_file() {
            Some(p) => p,
            None => return Ok(None),
        },
    };

    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;

    parse_config(&content, &path.display().to_string())
}
/// Walk up from CWD looking for `vastlint.toml` or `.vastlint.toml`.
fn find_config_file() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        for name in &["vastlint.toml", ".vastlint.toml"] {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Parse a TOML config string into a Config.
///
/// Expected format:
/// ```toml
/// telemetry = true   # opt-in to anonymous usage ping
///
/// [rules]
/// "VAST-2.0-mediafile-https" = "off"
/// "VAST-2.0-root-version" = "error"
/// ```
fn parse_config(content: &str, source: &str) -> Result<Option<Config>, String> {
    let table: toml::Table = content.parse().map_err(|e| format!("{source}: {e}"))?;

    let telemetry_cfg = match table.get("telemetry") {
        Some(toml::Value::Boolean(b)) => *b,
        Some(_) => return Err(format!("{source}: telemetry must be a boolean")),
        None => false,
    };

    let rules_table = match table.get("rules") {
        Some(toml::Value::Table(t)) => t,
        Some(_) => return Err(format!("{source}: [rules] must be a table")),
        None => {
            return Ok(Some(Config {
                rule_overrides: None,
                telemetry: telemetry_cfg,
            }));
        }
    };

    let catalog = vastlint_core::all_rules();
    let mut map: HashMap<&'static str, RuleLevel> = HashMap::new();

    for (key, val) in rules_table {
        let level_str = match val {
            toml::Value::String(s) => s.as_str(),
            _ => {
                return Err(format!(
                    "{source}: rule value for \"{key}\" must be a string"
                ))
            }
        };

        let level = match level_str {
            "error" => RuleLevel::Error,
            "warning" => RuleLevel::Warning,
            "info" => RuleLevel::Info,
            "off" => RuleLevel::Off,
            other => {
                return Err(format!(
                    "{source}: unknown level \"{other}\" for rule \"{key}\" — must be error, warning, info, or off"
                ))
            }
        };

        // Validate the rule ID against the catalog. Warn but continue.
        let known = catalog.iter().find(|r| r.id == key.as_str());
        if known.is_none() {
            eprintln!("{source}: warning: unknown rule ID \"{key}\" — ignored");
            continue;
        }

        // Safe: we just confirmed the ID exists in the static catalog, so the
        // static str lifetime is satisfied by the catalog entry.
        let static_id: &'static str = known.unwrap().id;
        map.insert(static_id, level);
    }

    Ok(Some(Config {
        rule_overrides: if map.is_empty() { None } else { Some(map) },
        telemetry: telemetry_cfg,
    }))
}

// ── check subcommand ──────────────────────────────────────────────────────────

fn run_check(
    files: Vec<String>,
    format: Format,
    no_fail: bool,
    config_path: Option<String>,
    no_config: bool,
    telemetry_flag: bool,
) -> ExitCode {
    let cfg = match load_config(config_path, no_config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(EXIT_USAGE_ERROR);
        }
    };

    let (rule_overrides, telemetry_enabled) = match cfg {
        Some(c) => (c.rule_overrides, c.telemetry || telemetry_flag),
        None => (None, telemetry_flag),
    };

    let ctx = ValidationContext {
        rule_overrides,
        ..Default::default()
    };

    let mut any_errors = false;

    for file in &files {
        let input = match read_input(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}: {}", file, e);
                return ExitCode::from(EXIT_USAGE_ERROR);
            }
        };

        let result = validate_with_context(&input, ctx.clone());
        let has_errors = !result.summary.is_valid();
        if has_errors {
            any_errors = true;
        }

        match format {
            Format::Plain => print_plain(file, &result),
            Format::Json => print_json(file, &result),
        }
    }

    if telemetry_enabled {
        telemetry::ping(files.len());
    } else {
        telemetry::maybe_show_notice();
    }

    if any_errors && !no_fail {
        ExitCode::from(EXIT_VALIDATION_ERROR)
    } else {
        ExitCode::SUCCESS
    }
}

fn read_input(file: &str) -> Result<String, String> {
    if file == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        Ok(buf)
    } else {
        std::fs::read_to_string(file).map_err(|e| format!("{e}"))
    }
}

// ── plain output ──────────────────────────────────────────────────────────────

const ERROR_STYLE: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Red)))
    .bold();
const WARN_STYLE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
const INFO_STYLE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
const DIM_STYLE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)));
const BOLD_STYLE: Style = Style::new().bold();
const OK_STYLE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));

const UNDERLINE_STYLE: Style = Style::new().underline();

fn print_plain(file: &str, result: &ValidationResult) {
    let version_str = result
        .version
        .best()
        .map(|v| v.as_str())
        .unwrap_or("unknown");

    println!(
        "\n{}{}{}  {}VAST {}{}",
        UNDERLINE_STYLE.render(),
        file,
        UNDERLINE_STYLE.render_reset(),
        DIM_STYLE.render(),
        version_str,
        DIM_STYLE.render_reset(),
    );

    if result.issues.is_empty() {
        println!(
            "  {}✓ no issues{}\n",
            OK_STYLE.render(),
            OK_STYLE.render_reset()
        );
        return;
    }

    for issue in &result.issues {
        print_issue(issue);
    }

    let s = &result.summary;
    let marker = if result.summary.is_valid() {
        format!("{}✓{}", OK_STYLE.render(), OK_STYLE.render_reset())
    } else {
        format!("{}✖{}", ERROR_STYLE.render(), ERROR_STYLE.render_reset())
    };

    println!(
        "\n{} {} error{}, {} warning{}, {} info\n",
        marker,
        s.errors,
        if s.errors == 1 { "" } else { "s" },
        s.warnings,
        if s.warnings == 1 { "" } else { "s" },
        s.infos,
    );
}

fn print_issue(issue: &Issue) {
    let (label, style) = match issue.severity {
        Severity::Error => ("error  ", ERROR_STYLE),
        Severity::Warning => ("warning", WARN_STYLE),
        Severity::Info => ("info   ", INFO_STYLE),
    };

    let path = issue.path.as_deref().unwrap_or("(document)");

    // Line 1: severity + message + rule ID
    println!(
        "  {}{}{}  {}  {}{}{}",
        style.render(),
        label,
        style.render_reset(),
        issue.message,
        DIM_STYLE.render(),
        issue.id,
        DIM_STYLE.render_reset(),
    );
    // Line 2: XPath location (dimmed, indented under severity)
    println!(
        "  {}         {}{}",
        DIM_STYLE.render(),
        path,
        DIM_STYLE.render_reset(),
    );
}

// ── JSON output ───────────────────────────────────────────────────────────────

fn print_json(file: &str, result: &ValidationResult) {
    let version_str = result
        .version
        .best()
        .map(|v| v.as_str())
        .unwrap_or("unknown");

    let issues_json: Vec<String> = result
        .issues
        .iter()
        .map(|i| {
            let path = match &i.path {
                Some(p) => format!("\"{}\"", json_escape(p)),
                None => "null".to_owned(),
            };
            let line = match i.line {
                Some(l) => l.to_string(),
                None => "null".to_owned(),
            };
            let col = match i.col {
                Some(c) => c.to_string(),
                None => "null".to_owned(),
            };
            format!(
                "{{\"id\":\"{}\",\"severity\":\"{}\",\"message\":\"{}\",\"path\":{},\"spec_ref\":\"{}\",\"line\":{},\"col\":{}}}",
                i.id,
                i.severity.as_str(),
                json_escape(i.message),
                path,
                i.spec_ref,
                line,
                col,
            )
        })
        .collect();

    println!(
        "{{\"file\":\"{}\",\"version\":\"{}\",\"valid\":{},\"summary\":{{\"errors\":{},\"warnings\":{},\"infos\":{}}},\"issues\":[{}]}}",
        json_escape(file),
        version_str,
        result.summary.is_valid(),
        result.summary.errors,
        result.summary.warnings,
        result.summary.infos,
        issues_json.join(","),
    );
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ── fix subcommand ────────────────────────────────────────────────────────────

fn run_fix(
    file: String,
    out: Option<String>,
    dry_run: bool,
    format: Format,
    config_path: Option<String>,
    no_config: bool,
) -> ExitCode {
    let cfg = match load_config(config_path, no_config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(EXIT_USAGE_ERROR);
        }
    };

    let rule_overrides = cfg.and_then(|c| c.rule_overrides);
    let ctx = ValidationContext {
        rule_overrides,
        ..Default::default()
    };

    let input = match read_input(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {}", file, e);
            return ExitCode::from(EXIT_USAGE_ERROR);
        }
    };

    let result = fix_with_context(&input, ctx);
    let is_stdin = file == "-";

    match format {
        Format::Plain => print_fix_plain(&file, &result, dry_run),
        Format::Json => print_fix_json(&file, &result),
    }

    if !dry_run {
        if is_stdin {
            // stdin → stdout: write repaired XML directly.
            print!("{}", result.xml);
        } else {
            let dest = out.as_deref().unwrap_or(file.as_str());

            // Write a .bak backup only when writing in-place (no --out flag).
            if out.is_none() {
                let bak = format!("{}.bak", file);
                if let Err(e) = std::fs::copy(&file, &bak) {
                    eprintln!("error: failed to write backup {}: {}", bak, e);
                    return ExitCode::from(EXIT_USAGE_ERROR);
                }
            }

            if let Err(e) = std::fs::write(dest, &result.xml) {
                eprintln!("error: failed to write {}: {}", dest, e);
                return ExitCode::from(EXIT_USAGE_ERROR);
            }
        }
    }

    // Exit 0 when no remaining errors; exit 1 when issues remain.
    if result
        .remaining
        .iter()
        .any(|i| i.severity == Severity::Error)
    {
        ExitCode::from(EXIT_VALIDATION_ERROR)
    } else {
        ExitCode::SUCCESS
    }
}

fn print_fix_plain(file: &str, result: &FixResult, dry_run: bool) {
    let dry_label = if dry_run { " (dry run)" } else { "" };
    println!(
        "\n{}{}{}{}",
        UNDERLINE_STYLE.render(),
        file,
        UNDERLINE_STYLE.render_reset(),
        dry_label,
    );

    if result.applied.is_empty() {
        println!(
            "  {}✓ nothing to fix{}",
            OK_STYLE.render(),
            OK_STYLE.render_reset()
        );
    } else {
        for fix in &result.applied {
            println!(
                "  {}fixed{}  {}  {}{}{}",
                OK_STYLE.render(),
                OK_STYLE.render_reset(),
                fix.description,
                DIM_STYLE.render(),
                fix.rule_id,
                DIM_STYLE.render_reset(),
            );
            println!(
                "  {}         {}{}",
                DIM_STYLE.render(),
                fix.path,
                DIM_STYLE.render_reset(),
            );
        }
    }

    if !result.remaining.is_empty() {
        println!(
            "\n  {}Remaining issues (not auto-fixable):{}",
            BOLD_STYLE.render(),
            BOLD_STYLE.render_reset()
        );
        for issue in &result.remaining {
            print_issue(issue);
        }
    }

    let n = result.applied.len();
    println!(
        "\n  {} fix{} applied, {} remaining\n",
        n,
        if n == 1 { "" } else { "es" },
        result.remaining.len(),
    );
}

fn print_fix_json(file: &str, result: &FixResult) {
    let applied_json: Vec<String> = result
        .applied
        .iter()
        .map(|f| {
            format!(
                "{{\"rule_id\":\"{}\",\"description\":\"{}\",\"path\":\"{}\"}}",
                f.rule_id,
                json_escape(&f.description),
                json_escape(&f.path),
            )
        })
        .collect();

    let remaining_json: Vec<String> = result
        .remaining
        .iter()
        .map(|i| {
            let path = match &i.path {
                Some(p) => format!("\"{}\"", json_escape(p)),
                None => "null".to_owned(),
            };
            format!(
                "{{\"id\":\"{}\",\"severity\":\"{}\",\"message\":\"{}\",\"path\":{}}}",
                i.id,
                i.severity.as_str(),
                json_escape(i.message),
                path,
            )
        })
        .collect();

    println!(
        "{{\"file\":\"{}\",\"applied\":[{}],\"remaining\":[{}],\"valid\":{}}}",
        json_escape(file),
        applied_json.join(","),
        remaining_json.join(","),
        result
            .remaining
            .iter()
            .all(|i| i.severity != Severity::Error),
    );
}

// ── rules subcommand ──────────────────────────────────────────────────────────

fn run_rules() {
    let rules = vastlint_core::all_rules();

    println!(
        "{}{:<45} {:<8} DESCRIPTION{}",
        BOLD_STYLE.render(),
        "RULE ID",
        "DEFAULT",
        BOLD_STYLE.render_reset()
    );
    println!(
        "{}{}{}",
        DIM_STYLE.render(),
        "─".repeat(100),
        DIM_STYLE.render_reset()
    );

    for rule in rules {
        let style = match rule.default_severity {
            Severity::Error => ERROR_STYLE,
            Severity::Warning => WARN_STYLE,
            Severity::Info => INFO_STYLE,
        };
        println!(
            "{:<45} {}{:<8}{}  {}",
            rule.id,
            style.render(),
            rule.default_severity.as_str(),
            style.render_reset(),
            rule.description,
        );
    }
}
