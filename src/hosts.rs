//! Pure host-selection logic: alias resolution, preference chain, pick one link.

use std::collections::HashMap;

/// Canonical host keys as they appear in books.json `download_links`.
pub const RAPIDGATOR: &str = "rapidgator";
pub const NITROFLARE: &str = "nitroflare";
pub const DDOWNLOAD: &str = "ddownload";

/// Resolve a user-supplied --host value (alias or full name, case-insensitive)
/// to a canonical host key. Returns None if unrecognized.
pub fn resolve_alias(input: &str) -> Option<&'static str> {
    match input.trim().to_ascii_lowercase().as_str() {
        "rg" | "rapidgator" => Some(RAPIDGATOR),
        "nf" | "nitroflare" => Some(NITROFLARE),
        "dd" | "ddownload" => Some(DDOWNLOAD),
        _ => None,
    }
}

/// The default preference order. nitroflare is intentionally absent (never auto-selected).
const DEFAULT_CHAIN: [&str; 2] = [RAPIDGATOR, DDOWNLOAD];

/// Build the ordered host preference list. If `preferred` is Some, it goes first,
/// then the default chain follows, with duplicates removed (preserving first occurrence).
pub fn preference_chain(preferred: Option<&'static str>) -> Vec<&'static str> {
    let mut chain: Vec<&'static str> = Vec::new();
    if let Some(p) = preferred {
        chain.push(p);
    }
    for &h in DEFAULT_CHAIN.iter() {
        if !chain.contains(&h) {
            chain.push(h);
        }
    }
    chain
}

/// Pick exactly one (host, url) for a book given its download_links and an optional
/// preferred host. Walks the preference chain and returns the first host that has a
/// non-empty URL list, taking that list's first URL. Returns None if no chained host
/// has a usable link (the book is skipped by the caller).
pub fn pick_link(
    download_links: &HashMap<String, Vec<String>>,
    preferred: Option<&'static str>,
) -> Option<(&'static str, String)> {
    for host in preference_chain(preferred) {
        if let Some(urls) = download_links.get(host) {
            if let Some(first) = urls.first() {
                return Some((host, first.clone()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_aliases_case_insensitively() {
        assert_eq!(resolve_alias("rg"), Some(RAPIDGATOR));
        assert_eq!(resolve_alias("RG"), Some(RAPIDGATOR));
        assert_eq!(resolve_alias("rapidgator"), Some(RAPIDGATOR));
        assert_eq!(resolve_alias("nf"), Some(NITROFLARE));
        assert_eq!(resolve_alias("Nitroflare"), Some(NITROFLARE));
        assert_eq!(resolve_alias("dd"), Some(DDOWNLOAD));
        assert_eq!(resolve_alias(" ddownload "), Some(DDOWNLOAD));
        assert_eq!(resolve_alias("mega"), None);
    }

    #[test]
    fn default_chain_is_rapidgator_then_ddownload() {
        assert_eq!(preference_chain(None), vec![RAPIDGATOR, DDOWNLOAD]);
    }

    #[test]
    fn host_rg_matches_default() {
        assert_eq!(preference_chain(Some(RAPIDGATOR)), vec![RAPIDGATOR, DDOWNLOAD]);
    }

    #[test]
    fn host_nf_puts_nitroflare_first_then_default() {
        assert_eq!(preference_chain(Some(NITROFLARE)), vec![NITROFLARE, RAPIDGATOR, DDOWNLOAD]);
    }

    #[test]
    fn host_dd_puts_ddownload_first_deduped() {
        assert_eq!(preference_chain(Some(DDOWNLOAD)), vec![DDOWNLOAD, RAPIDGATOR]);
    }

    fn links(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(h, urls)| (h.to_string(), urls.iter().map(|u| u.to_string()).collect()))
            .collect()
    }

    #[test]
    fn picks_first_available_in_chain() {
        let dl = links(&[
            ("rapidgator", &["RG_URL"]),
            ("nitroflare", &["NF_URL"]),
        ]);
        let picked = pick_link(&dl, None);
        assert_eq!(picked, Some(("rapidgator", "RG_URL".to_string())));
    }

    #[test]
    fn falls_back_to_ddownload_when_no_rapidgator() {
        let dl = links(&[
            ("nitroflare", &["NF_URL"]),
            ("ddownload", &["DD_URL"]),
        ]);
        let picked = pick_link(&dl, None);
        assert_eq!(picked, Some(("ddownload", "DD_URL".to_string())));
    }

    #[test]
    fn skips_when_only_offchain_host() {
        let dl = links(&[("nitroflare", &["NF_URL"])]);
        assert_eq!(pick_link(&dl, None), None);
    }

    #[test]
    fn host_nf_picks_nitroflare_when_present() {
        let dl = links(&[
            ("rapidgator", &["RG_URL"]),
            ("nitroflare", &["NF_URL"]),
        ]);
        let picked = pick_link(&dl, Some(NITROFLARE));
        assert_eq!(picked, Some(("nitroflare", "NF_URL".to_string())));
    }

    #[test]
    fn empty_host_array_is_skipped_in_chain() {
        let dl = links(&[
            ("rapidgator", &[]),
            ("ddownload", &["DD_URL"]),
        ]);
        let picked = pick_link(&dl, None);
        assert_eq!(picked, Some(("ddownload", "DD_URL".to_string())));
    }

    #[test]
    fn multiple_urls_takes_first() {
        let dl = links(&[("rapidgator", &["FIRST", "SECOND"])]);
        let picked = pick_link(&dl, None);
        assert_eq!(picked, Some(("rapidgator", "FIRST".to_string())));
    }
}
