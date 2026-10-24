//! Text rendering for the CLI.

use awd_core::{Device, DeviceStatus, Lang, ProblemCode, ScanReport, ScanSummary};

/// Which devices to show.
#[derive(Debug, Default, Clone, Copy)]
pub struct Filter<'a> {
    /// Only devices with a problem or disabled.
    pub problems_only: bool,
    /// Only devices of this class (case-insensitive).
    pub class: Option<&'a str>,
}

impl Filter<'_> {
    fn keep(&self, d: &Device) -> bool {
        if self.problems_only && d.problem_code().is_none() {
            return false;
        }
        match self.class {
            Some(c) => d
                .class
                .as_deref()
                .is_some_and(|dc| dc.eq_ignore_ascii_case(c)),
            None => true,
        }
    }
}

/// A copy of the report containing only the devices that pass `filter`.
pub fn filtered(report: &ScanReport, filter: &Filter<'_>) -> ScanReport {
    ScanReport {
        scanned_at: report.scanned_at,
        host: report.host.clone(),
        devices: report
            .devices
            .iter()
            .filter(|d| filter.keep(d))
            .cloned()
            .collect(),
    }
}

/// Human-readable explanation of a problem code.
pub fn problem(code: ProblemCode, lang: Lang) -> String {
    format!(
        "Code {}: {}\n{}\nSeverity: {:?}. Suggested action: {:?}.",
        code.code(),
        code.name(),
        code.describe(lang),
        code.severity(),
        code.action()
    )
}

const COLUMNS: [(&str, usize); 5] = [
    ("STATUS", 18),
    ("CLASS", 14),
    ("NAME", 40),
    ("DRIVER", 48),
    ("HARDWARE ID", 46),
];

/// The device table plus a summary line.
pub fn table(report: &ScanReport, filter: &Filter<'_>) -> String {
    let mut rows: Vec<[String; 5]> = report
        .devices
        .iter()
        .filter(|d| filter.keep(d))
        .map(row)
        .collect();
    rows.sort_by(|a, b| a[1].cmp(&b[1]).then_with(|| a[2].cmp(&b[2])));

    let widths: Vec<usize> = COLUMNS
        .iter()
        .enumerate()
        .map(|(i, (title, cap))| {
            rows.iter()
                .map(|r| char_len(&r[i]))
                .chain(std::iter::once(title.chars().count()))
                .max()
                .unwrap_or(0)
                .min(*cap)
        })
        .collect();

    let mut out = String::new();
    push_row(&mut out, &COLUMNS.map(|(t, _)| t.to_owned()), &widths);
    push_row(
        &mut out,
        &widths
            .iter()
            .map(|w| "-".repeat(*w))
            .collect::<Vec<_>>()
            .try_into()
            .expect("five columns"),
        &widths,
    );
    for r in &rows {
        push_row(&mut out, r, &widths);
    }

    let shown = rows.len();
    let s = report.summary();
    out.push('\n');
    out.push_str(&summary_line(&s, shown));
    out.push('\n');
    out
}

fn row(d: &Device) -> [String; 5] {
    let status = match d.status {
        DeviceStatus::Ok => "OK".to_owned(),
        DeviceStatus::Problem { code } => format!("!{} {}", code.code(), code.name()),
        DeviceStatus::Disabled => "disabled".to_owned(),
        DeviceStatus::NotPresent => "not present".to_owned(),
        DeviceStatus::Unknown => "?".to_owned(),
    };
    let driver = match &d.driver {
        None => "-".to_owned(),
        Some(drv) => {
            let version = drv
                .version
                .map(|v| v.to_string())
                .or_else(|| drv.version_raw.clone())
                .unwrap_or_else(|| "?".to_owned());
            let mut s = version;
            if let Some(date) = drv.date {
                s.push_str(&format!(" ({date})"));
            }
            if let Some(p) = &drv.provider {
                s.push_str(&format!(" {p}"));
            }
            if drv.is_inbox {
                s.push_str(" [inbox]");
            }
            s
        }
    };
    let hwid = d
        .hardware_ids
        .first()
        .map(|h| h.as_str().to_owned())
        .unwrap_or_else(|| d.instance_id.clone());
    [
        status,
        d.class.clone().unwrap_or_else(|| "-".to_owned()),
        d.name.clone(),
        driver,
        hwid,
    ]
}

fn summary_line(s: &ScanSummary, shown: usize) -> String {
    let mut line = format!(
        "{} devices ({} shown): {} ok, {} with problems",
        s.total, shown, s.ok, s.problems
    );
    if s.missing_driver > 0 {
        line.push_str(&format!(" ({} without a driver)", s.missing_driver));
    }
    line.push_str(&format!(", {} disabled", s.disabled));
    if s.not_present > 0 {
        line.push_str(&format!(", {} not present", s.not_present));
    }
    if s.unknown > 0 {
        line.push_str(&format!(", {} unknown", s.unknown));
    }
    line
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

fn fit(s: &str, width: usize) -> String {
    let n = char_len(s);
    if n <= width {
        let mut out = s.to_owned();
        out.extend(std::iter::repeat_n(' ', width - n));
        out
    } else if width <= 1 {
        "…".repeat(width)
    } else {
        let mut out: String = s.chars().take(width - 1).collect();
        out.push('…');
        out
    }
}

fn push_row(out: &mut String, cells: &[String; 5], widths: &[usize]) {
    let line = cells
        .iter()
        .zip(widths)
        .map(|(c, w)| fit(c, *w))
        .collect::<Vec<_>>()
        .join("  ");
    out.push_str(line.trim_end());
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../../tests/fixtures/scan-sample.json");

    fn report() -> ScanReport {
        serde_json::from_str(SAMPLE).unwrap()
    }

    #[test]
    fn table_lists_every_device_and_summarizes() {
        let text = table(&report(), &Filter::default());
        assert!(text.contains("NVIDIA GeForce RTX 4090"));
        assert!(text.contains("!28 FAILED_INSTALL"));
        assert!(text.contains("32.0.15.6094 (2024-08-14) NVIDIA"));
        assert!(text.contains("[inbox]"));
        assert!(text.contains("6 devices (6 shown): 3 ok, 1 with problems (1 without a driver), 1 disabled, 1 not present"));
    }

    #[test]
    fn problems_filter_keeps_problem_and_disabled_devices() {
        let f = Filter {
            problems_only: true,
            class: None,
        };
        let r = filtered(&report(), &f);
        assert_eq!(r.devices.len(), 2);
        assert!(r.devices.iter().all(|d| d.problem_code().is_some()));
    }

    #[test]
    fn class_filter_is_case_insensitive() {
        let f = Filter {
            problems_only: false,
            class: Some("display"),
        };
        assert_eq!(filtered(&report(), &f).devices.len(), 1);
    }

    #[test]
    fn fit_counts_characters_not_bytes() {
        assert_eq!(fit("Привет", 8), "Привет  ");
        assert_eq!(fit("Привет, мир", 7), "Привет…");
        assert_eq!(fit("abc", 3), "abc");
    }

    #[test]
    fn problem_explanation_mentions_code_and_action() {
        let text = problem(ProblemCode(28), Lang::Ru);
        assert!(text.starts_with("Code 28: FAILED_INSTALL"));
        assert!(text.contains("Драйверы для этого устройства не установлены."));
        assert!(text.contains("FindDriver"));
    }
}
