// SPDX-License-Identifier: MIT AND Apache-2.0
//! Local **model** of the shape of LP-0021 adoption criteria A2/A3.
//!
//! A2 asks for 150 on-chain testimonials from distinct accounts. A3 asks for
//! sustained activity: at least 30 new testimonials in each of at least two
//! months. This crate can only count records in a local [`Store`] snapshot and
//! report whether their *shape* matches those thresholds.
//!
//! # HONESTY / LIMITATIONS
//!
//! * No chain, no testnet 0.4, no transactions, no verifier, no users.
//! * A passing [`ThresholdCheck`] is **NOT evidence** that A2 or A3 were met,
//!   and must never be presented as such. Real evidence must come from the
//!   official zone, real accounts, and a real two-month clock.
//! * [`LOCAL_MODEL_LABEL`] is carried in every structured result so a caller
//!   cannot accidentally drop the caveat.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use testimonial_store::{AuthorId, Testimonial, UtcMonth};

/// Local model of the A2 distinct-author threshold.
pub const MIN_DISTINCT_AUTHORS: usize = 150;
/// Local model of the A3 per-month threshold.
pub const MIN_RECORDS_PER_MONTH: usize = 30;
/// Local model of the A3 consecutive-month threshold.
pub const MIN_CONSECUTIVE_MONTHS: usize = 2;

/// Mandatory caveat attached to every result from this crate.
pub const LOCAL_MODEL_LABEL: &str =
    "LOCAL MODEL ONLY - not on-chain, not adoption evidence for LP-0021 A2/A3";

/// One UTC month's counts from the local snapshot.
#[derive(Clone, Debug)]
pub struct MonthCount {
    month: UtcMonth,
    records: usize,
    authors: BTreeSet<AuthorId>,
}

impl MonthCount {
    /// The UTC calendar month.
    pub const fn month(&self) -> UtcMonth {
        self.month
    }

    /// Number of records in that month.
    pub const fn records(&self) -> usize {
        self.records
    }

    /// Distinct local author handles seen in that month.
    pub fn distinct_authors(&self) -> usize {
        self.authors.len()
    }
}

/// Records grouped by UTC calendar month.
#[derive(Clone, Debug, Default)]
pub struct MonthlyCounts {
    months: BTreeMap<UtcMonth, MonthCount>,
}

impl MonthlyCounts {
    /// Build counts from any iterator of local testimonial references.
    pub fn from_records<'a, I>(records: I) -> Self
    where
        I: IntoIterator<Item = &'a Testimonial>,
    {
        let mut months: BTreeMap<UtcMonth, MonthCount> = BTreeMap::new();
        for record in records {
            let month = UtcMonth::from_timestamp_ms(record.timestamp_ms());
            let entry = months.entry(month).or_insert_with(|| MonthCount {
                month,
                records: 0,
                authors: BTreeSet::new(),
            });
            entry.records += 1;
            entry.authors.insert(record.author());
        }
        Self { months }
    }

    /// Number of records in `month`.
    pub fn records_in(&self, month: UtcMonth) -> usize {
        self.months.get(&month).map_or(0, MonthCount::records)
    }

    /// Distinct local authors in `month`.
    pub fn distinct_authors_in(&self, month: UtcMonth) -> usize {
        self.months
            .get(&month)
            .map_or(0, MonthCount::distinct_authors)
    }

    /// All months in ascending calendar order.
    pub fn months(&self) -> impl Iterator<Item = (&UtcMonth, &MonthCount)> {
        self.months.iter()
    }
}

/// Plain summary of one qualifying month.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonthSummary {
    /// UTC calendar month.
    pub month: UtcMonth,
    /// Records in that month.
    pub records: usize,
    /// Distinct local authors in that month.
    pub distinct_authors: usize,
}

/// Structured result of a local A2/A3 shape check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThresholdCheck {
    /// Distinct local authors across the whole snapshot.
    pub distinct_authors_total: usize,
    /// Longest run of consecutive months meeting the per-month count.
    pub qualifying_months: Vec<MonthSummary>,
    /// Length of that run.
    pub longest_qualifying_run: usize,
    /// Whether the local model's shape matches the stated thresholds.
    pub met: bool,
    /// Mandatory caveat. Never remove or ignore this field.
    pub label: &'static str,
}

/// Check the *shape* of A2/A3 against a local snapshot.
///
/// The rule modelled here is: at least 150 distinct authors overall, and at
/// least two consecutive UTC calendar months with at least 30 records each.
/// `qualifying_months` is the longest such consecutive run. A `true` result is
/// still only a local model result, never adoption evidence.
pub fn check_a2_a3_shape(counts: &MonthlyCounts) -> ThresholdCheck {
    let mut all_authors = BTreeSet::new();
    let mut summaries = Vec::new();
    for count in counts.months.values() {
        all_authors.extend(count.authors.iter().copied());
        summaries.push(MonthSummary {
            month: count.month(),
            records: count.records(),
            distinct_authors: count.distinct_authors(),
        });
    }

    let mut best: Vec<MonthSummary> = Vec::new();
    let mut run: Vec<MonthSummary> = Vec::new();
    let mut previous_month: Option<UtcMonth> = None;
    for summary in &summaries {
        let consecutive = previous_month.is_some_and(|month| month.next() == summary.month);
        if summary.records >= MIN_RECORDS_PER_MONTH {
            if consecutive && !run.is_empty() {
                run.push(summary.clone());
            } else {
                run = vec![summary.clone()];
            }
        } else {
            run.clear();
        }
        if run.len() > best.len() {
            best = run.clone();
        }
        previous_month = Some(summary.month);
    }

    let longest_qualifying_run = best.len();
    let met = all_authors.len() >= MIN_DISTINCT_AUTHORS
        && longest_qualifying_run >= MIN_CONSECUTIVE_MONTHS;
    ThresholdCheck {
        distinct_authors_total: all_authors.len(),
        qualifying_months: best,
        longest_qualifying_run,
        met,
        label: LOCAL_MODEL_LABEL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testimonial_store::{AuthorId, Error, Store, FIXED_SUBMISSION_ID};

    const JAN_2026: i64 = 1_767_225_600_000;
    const FEB_2026: i64 = 1_769_904_000_000;
    const MAR_2026: i64 = 1_772_323_200_000;
    const APR_2026: i64 = 1_775_001_600_000;
    const MAY_2026: i64 = 1_777_593_600_000;
    const JUN_2026: i64 = 1_780_272_000_000;

    fn store_with(specs: &[(u64, i64)]) -> Store {
        let mut store = Store::new();
        for (serial, ms) in specs {
            store
                .append(
                    FIXED_SUBMISSION_ID,
                    AuthorId::new(*serial),
                    "I use wallet XYZ",
                    None,
                    *ms,
                )
                .unwrap();
        }
        store
    }

    fn authors(start: u64, count: u64, ms: i64) -> Vec<(u64, i64)> {
        (start..start + count).map(|serial| (serial, ms)).collect()
    }

    fn check(store: &Store) -> ThresholdCheck {
        check_a2_a3_shape(&MonthlyCounts::from_records(store.list().iter()))
    }

    #[test]
    fn monthly_counts_are_empty_by_default() {
        let counts = MonthlyCounts::default();
        assert!(counts.months().next().is_none());
        assert_eq!(counts.records_in(UtcMonth::new(2026, 1).unwrap()), 0);
        assert_eq!(
            counts.distinct_authors_in(UtcMonth::new(2026, 1).unwrap()),
            0
        );
    }

    #[test]
    fn counts_records_per_utc_month() {
        let mut specs = authors(1, 3, JAN_2026);
        specs.extend(authors(10, 2, FEB_2026));
        let counts = MonthlyCounts::from_records(store_with(&specs).list().iter());
        assert_eq!(counts.records_in(UtcMonth::new(2026, 1).unwrap()), 3);
        assert_eq!(counts.records_in(UtcMonth::new(2026, 2).unwrap()), 2);
        assert_eq!(counts.months().count(), 2);
    }

    #[test]
    fn counts_distinct_authors_per_month() {
        let mut store = Store::new();
        store
            .append(FIXED_SUBMISSION_ID, AuthorId::new(1), "a", None, JAN_2026)
            .unwrap();
        let duplicate_result = store.append(
            FIXED_SUBMISSION_ID,
            AuthorId::new(1),
            "b",
            None,
            JAN_2026 + 1,
        );
        assert_eq!(duplicate_result.err(), Some(Error::DuplicateAuthor));
        store
            .append(
                FIXED_SUBMISSION_ID,
                AuthorId::new(2),
                "c",
                None,
                JAN_2026 + 2,
            )
            .unwrap();
        let counts = MonthlyCounts::from_records(store.list().iter());
        assert_eq!(counts.records_in(UtcMonth::new(2026, 1).unwrap()), 2);
        assert_eq!(
            counts.distinct_authors_in(UtcMonth::new(2026, 1).unwrap()),
            2
        );
    }

    #[test]
    fn month_boundary_counts_are_separate() {
        let mut specs = authors(1, 2, 1_767_225_599_999);
        specs.extend(authors(10, 1, JAN_2026));
        let counts = MonthlyCounts::from_records(store_with(&specs).list().iter());
        assert_eq!(counts.records_in(UtcMonth::new(2025, 12).unwrap()), 2);
        assert_eq!(counts.records_in(UtcMonth::new(2026, 1).unwrap()), 1);
    }

    #[test]
    fn threshold_with_zero_records_is_not_met() {
        let result = check(&Store::new());
        assert_eq!(result.distinct_authors_total, 0);
        assert!(result.qualifying_months.is_empty());
        assert_eq!(result.longest_qualifying_run, 0);
        assert!(!result.met);
    }

    #[test]
    fn threshold_fails_with_149_distinct_authors() {
        let mut specs = authors(1, 75, JAN_2026);
        specs.extend(authors(76, 74, FEB_2026));
        let result = check(&store_with(&specs));
        assert_eq!(result.distinct_authors_total, 149);
        assert_eq!(result.longest_qualifying_run, 2);
        assert!(!result.met);
    }

    #[test]
    fn threshold_fails_with_fewer_than_150_distinct_authors() {
        let mut specs = authors(1, 30, JAN_2026);
        specs.extend(authors(31, 30, FEB_2026));
        let result = check(&store_with(&specs));
        assert_eq!(result.distinct_authors_total, 60);
        assert_eq!(result.longest_qualifying_run, 2);
        assert!(!result.met);
    }

    #[test]
    fn threshold_fails_with_only_one_qualifying_month() {
        let result = check(&store_with(&authors(1, 150, JAN_2026)));
        assert_eq!(result.distinct_authors_total, 150);
        assert_eq!(result.longest_qualifying_run, 1);
        assert!(!result.met);
    }

    #[test]
    fn threshold_fails_with_29_records_in_a_month() {
        let mut specs = authors(1, 29, JAN_2026);
        specs.extend(authors(30, 121, FEB_2026));
        let result = check(&store_with(&specs));
        assert_eq!(result.distinct_authors_total, 150);
        assert_eq!(result.qualifying_months.len(), 1);
        assert_eq!(result.qualifying_months[0].month.to_string(), "2026-02");
        assert_eq!(result.qualifying_months[0].records, 121);
        assert_eq!(result.longest_qualifying_run, 1);
        assert!(!result.met);
    }

    #[test]
    fn threshold_fails_with_non_consecutive_months() {
        let mut specs = authors(1, 30, JAN_2026);
        specs.extend(authors(31, 120, MAR_2026));
        let result = check(&store_with(&specs));
        assert_eq!(result.distinct_authors_total, 150);
        assert_eq!(result.longest_qualifying_run, 1);
        assert!(!result.met);
    }

    #[test]
    fn threshold_passes_at_exact_edges() {
        let mut specs = authors(1, 75, JAN_2026);
        specs.extend(authors(76, 75, FEB_2026));
        let result = check(&store_with(&specs));
        assert_eq!(result.distinct_authors_total, 150);
        assert_eq!(result.longest_qualifying_run, 2);
        assert_eq!(result.qualifying_months.len(), 2);
        assert_eq!(result.qualifying_months[0].month.to_string(), "2026-01");
        assert_eq!(result.qualifying_months[1].month.to_string(), "2026-02");
        assert_eq!(result.qualifying_months[0].records, 75);
        assert!(result.met);
    }

    #[test]
    fn threshold_prefers_longest_consecutive_run() {
        let mut specs = authors(1, 30, JAN_2026);
        specs.extend(authors(31, 30, FEB_2026));
        specs.extend(authors(61, 30, MAR_2026));
        specs.extend(authors(91, 5, APR_2026));
        specs.extend(authors(96, 30, MAY_2026));
        specs.extend(authors(126, 30, JUN_2026));
        let result = check(&store_with(&specs));
        assert_eq!(result.distinct_authors_total, 155);
        assert_eq!(result.longest_qualifying_run, 3);
        assert_eq!(result.qualifying_months.len(), 3);
        assert_eq!(result.qualifying_months[2].month.to_string(), "2026-03");
        assert!(result.met);
    }

    #[test]
    fn threshold_result_is_labelled_as_a_local_model() {
        let result = check(&store_with(&authors(1, 30, JAN_2026)));
        assert_eq!(result.label, LOCAL_MODEL_LABEL);
        assert!(result.label.contains("LOCAL MODEL ONLY"));
        assert!(result.label.contains("not adoption evidence"));
    }
}
