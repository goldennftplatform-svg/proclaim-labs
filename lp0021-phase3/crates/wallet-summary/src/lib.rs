// SPDX-License-Identifier: MIT AND Apache-2.0
//! Human-readable summaries of proposed token effects for LP-0021 phase 3.
//!
//! HONESTY: summaries are produced **only from structured data** supplied by
//! the caller (labels, amounts, bounds and metadata). This crate never parses
//! calldata, never contacts a chain and never signs anything. It is not a
//! security review of a transaction; it is a legibility aid for a local model.
//! No private handles are accepted or printed here.

#![forbid(unsafe_code)]

use std::fmt::{self, Display, Formatter};

use wallet_assets::{AmountBound, TokenMeta};

const MAX_DECIMALS: u8 = 18;

/// Errors when building a structured summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SummaryError {
    /// A label was blank.
    EmptyLabel,
    /// Too many decimal places for safe local formatting.
    DecimalsTooLarge(u8),
    /// There is nothing to summarise.
    EmptyEffects,
}

impl Display for SummaryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLabel => write!(f, "summary labels must not be empty"),
            Self::DecimalsTooLarge(decimals) => {
                write!(
                    f,
                    "decimals must be at most {MAX_DECIMALS}, found {decimals}"
                )
            }
            Self::EmptyEffects => write!(f, "cannot summarise an empty effect list"),
        }
    }
}

impl std::error::Error for SummaryError {}

fn checked_label(raw: &str) -> Result<String, SummaryError> {
    let raw = raw.trim();
    if raw.is_empty() {
        Err(SummaryError::EmptyLabel)
    } else {
        Ok(raw.to_owned())
    }
}

/// A caller-supplied account label, e.g. `A`. Never a private handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountLabel(String);

impl AccountLabel {
    /// Build a non-empty account label.
    pub fn new(raw: &str) -> Result<Self, SummaryError> {
        Ok(Self(checked_label(raw)?))
    }

    /// Label text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for AccountLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A caller-supplied program label, e.g. `P`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramLabel(String);

impl ProgramLabel {
    /// Build a non-empty program label.
    pub fn new(raw: &str) -> Result<Self, SummaryError> {
        Ok(Self(checked_label(raw)?))
    }

    /// Label text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ProgramLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Token symbol plus local decimal count, used for unit formatting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetLabel {
    symbol: String,
    decimals: u8,
}

impl AssetLabel {
    /// Build an asset label with a decimal count.
    pub fn new(symbol: &str, decimals: u8) -> Result<Self, SummaryError> {
        if decimals > MAX_DECIMALS {
            return Err(SummaryError::DecimalsTooLarge(decimals));
        }
        Ok(Self {
            symbol: checked_label(symbol)?,
            decimals,
        })
    }

    /// Build a label from local token metadata.
    pub fn from_meta(meta: &TokenMeta) -> Self {
        Self {
            symbol: meta.symbol().to_owned(),
            decimals: meta.decimals(),
        }
    }

    /// Token symbol.
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Decimal count.
    pub const fn decimals(&self) -> u8 {
        self.decimals
    }
}

impl Display for AssetLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

/// One structured effect of a proposed transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// A token transfer between two labelled accounts.
    Transfer {
        from: AccountLabel,
        to: AccountLabel,
        asset: AssetLabel,
        amount: u128,
    },
    /// An approval of a labelled program.
    Approval {
        owner: AccountLabel,
        spender: ProgramLabel,
        asset: AssetLabel,
        bound: AmountBound,
    },
    /// A local mint credit.
    Mint {
        to: AccountLabel,
        asset: AssetLabel,
        amount: u128,
    },
    /// A local burn debit.
    Burn {
        from: AccountLabel,
        asset: AssetLabel,
        amount: u128,
    },
}

/// Rendered summary and any explicit risk warnings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    text: String,
    warnings: Vec<String>,
}

impl Summary {
    /// Main human-readable sentence.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Explicit warnings, empty when no risk was detected.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Full display text, including warnings.
    pub fn render(&self) -> String {
        let mut out = self.text.clone();
        for warning in &self.warnings {
            out.push(' ');
            out.push_str(warning);
        }
        out
    }
}

impl Display for Summary {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

/// Format base units using the token's decimal count, trimming trailing zeros.
fn format_units(amount: u128, decimals: u8) -> String {
    if decimals == 0 {
        return amount.to_string();
    }
    let scale = 10_u128.pow(u32::from(decimals));
    let whole = amount / scale;
    let fraction = amount % scale;
    if fraction == 0 {
        whole.to_string()
    } else {
        let padded = format!("{:0width$}", fraction, width = usize::from(decimals));
        let trimmed = padded.trim_end_matches('0');
        format!("{whole}.{trimmed}")
    }
}

/// Produce a summary from structured effects only.
pub fn summarise(effects: &[Effect]) -> Result<Summary, SummaryError> {
    if effects.is_empty() {
        return Err(SummaryError::EmptyEffects);
    }
    let mut parts = Vec::with_capacity(effects.len());
    let mut warnings = Vec::new();
    for effect in effects {
        match effect {
            Effect::Transfer {
                from,
                to,
                asset,
                amount,
            } => parts.push(format!(
                "Send {} {} from account {} to {}",
                format_units(*amount, asset.decimals()),
                asset.symbol(),
                from,
                to
            )),
            Effect::Approval {
                spender,
                asset,
                bound,
                ..
            } => {
                parts.push(format!(
                    "approve {} {} to program {}",
                    match bound {
                        AmountBound::Limited(amount) => format_units(*amount, asset.decimals()),
                        AmountBound::Unlimited => "unlimited".to_owned(),
                    },
                    asset.symbol(),
                    spender
                ));
                if bound.is_unlimited() {
                    warnings.push(format!(
                        "WARNING: unlimited approval of {} to program {} has no spending cap",
                        asset.symbol(),
                        spender
                    ));
                }
            }
            Effect::Mint { to, asset, amount } => parts.push(format!(
                "Mint {} {} to account {}",
                format_units(*amount, asset.decimals()),
                asset.symbol(),
                to
            )),
            Effect::Burn {
                from,
                asset,
                amount,
            } => parts.push(format!(
                "Burn {} {} from account {}",
                format_units(*amount, asset.decimals()),
                asset.symbol(),
                from
            )),
        }
    }
    Ok(Summary {
        text: format!("{}.", parts.join("; ")),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallet_assets::MintId;

    fn asset() -> AssetLabel {
        AssetLabel::new("TKN", 0).unwrap()
    }

    fn a() -> AccountLabel {
        AccountLabel::new("A").unwrap()
    }

    fn b() -> AccountLabel {
        AccountLabel::new("B").unwrap()
    }

    #[test]
    fn summarise_transfer_then_approval_example() {
        let summary = summarise(&[
            Effect::Transfer {
                from: a(),
                to: b(),
                asset: asset(),
                amount: 5,
            },
            Effect::Approval {
                owner: a(),
                spender: ProgramLabel::new("P").unwrap(),
                asset: asset(),
                bound: AmountBound::limited(100),
            },
        ])
        .unwrap();
        assert_eq!(
            summary.text(),
            "Send 5 TKN from account A to B; approve 100 TKN to program P."
        );
        assert!(summary.warnings().is_empty());
        assert_eq!(summary.render(), summary.text());
    }

    #[test]
    fn unlimited_approval_warning_is_explicit() {
        let summary = summarise(&[Effect::Approval {
            owner: a(),
            spender: ProgramLabel::new("P").unwrap(),
            asset: asset(),
            bound: AmountBound::unlimited(),
        }])
        .unwrap();
        assert_eq!(summary.text(), "approve unlimited TKN to program P.");
        assert_eq!(summary.warnings().len(), 1);
        assert!(summary.warnings()[0].starts_with("WARNING: unlimited approval"));
        assert!(summary.render().contains("WARNING: unlimited approval"));
    }

    #[test]
    fn limited_approval_has_no_warning() {
        let summary = summarise(&[Effect::Approval {
            owner: a(),
            spender: ProgramLabel::new("P").unwrap(),
            asset: asset(),
            bound: AmountBound::limited(10),
        }])
        .unwrap();
        assert!(summary.warnings().is_empty());
    }

    #[test]
    fn format_units_uses_decimals_and_trims_zeros() {
        assert_eq!(format_units(5, 0), "5");
        assert_eq!(format_units(5, 2), "0.05");
        assert_eq!(format_units(250, 2), "2.5");
        assert_eq!(format_units(100, 3), "0.1");
        assert_eq!(format_units(1_000_000, 6), "1");
    }

    #[test]
    fn empty_effects_is_an_error() {
        assert_eq!(summarise(&[]), Err(SummaryError::EmptyEffects));
    }

    #[test]
    fn blank_labels_are_rejected() {
        assert_eq!(AccountLabel::new("  "), Err(SummaryError::EmptyLabel));
        assert_eq!(ProgramLabel::new(""), Err(SummaryError::EmptyLabel));
        assert_eq!(
            AssetLabel::new("TKN", 19),
            Err(SummaryError::DecimalsTooLarge(19))
        );
        assert_eq!(AssetLabel::new("", 0), Err(SummaryError::EmptyLabel));
    }

    #[test]
    fn summary_contains_no_private_handle_text() {
        let summary = summarise(&[Effect::Transfer {
            from: a(),
            to: b(),
            asset: asset(),
            amount: 1,
        }])
        .unwrap();
        assert!(!summary.render().contains("pv-handle"));
        assert!(!summary.render().contains("acct-"));
    }

    #[test]
    fn multiple_transfers_join_with_semicolons() {
        let summary = summarise(&[
            Effect::Transfer {
                from: a(),
                to: b(),
                asset: asset(),
                amount: 1,
            },
            Effect::Transfer {
                from: b(),
                to: a(),
                asset: asset(),
                amount: 2,
            },
        ])
        .unwrap();
        assert_eq!(
            summary.text(),
            "Send 1 TKN from account A to B; Send 2 TKN from account B to A."
        );
    }

    #[test]
    fn asset_label_from_meta_uses_symbol_and_decimals() {
        let meta = TokenMeta::new(MintId::new(3), 2, "USD").unwrap();
        let label = AssetLabel::from_meta(&meta);
        assert_eq!(label.symbol(), "USD");
        assert_eq!(label.decimals(), 2);
        let summary = summarise(&[Effect::Mint {
            to: a(),
            asset: label,
            amount: 125,
        }])
        .unwrap();
        assert_eq!(summary.text(), "Mint 1.25 USD to account A.");
    }

    #[test]
    fn mint_and_burn_effects_render() {
        let summary = summarise(&[
            Effect::Mint {
                to: a(),
                asset: asset(),
                amount: 3,
            },
            Effect::Burn {
                from: b(),
                asset: asset(),
                amount: 2,
            },
        ])
        .unwrap();
        assert_eq!(
            summary.text(),
            "Mint 3 TKN to account A; Burn 2 TKN from account B."
        );
    }
}
