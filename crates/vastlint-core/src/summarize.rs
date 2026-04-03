//! Summarise a list of Issues into a Summary.

use crate::{Issue, Severity, Summary};

pub fn summarize(issues: &[Issue]) -> Summary {
    let mut errors: usize = 0;
    let mut warnings: usize = 0;
    let mut infos: usize = 0;

    for issue in issues {
        match issue.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
            Severity::Info => infos += 1,
        }
    }

    Summary {
        errors,
        warnings,
        infos,
    }
}
