// ============================================================================
// FZF 风格的模糊搜索
// ============================================================================

use crate::model::PasswordEntry;

/// FZF 风格模糊匹配，返回分数，不匹配返回 None
pub fn fuzzy_match(query: &str, target: &str) -> Option<u32> {
    if query.is_empty() {
        return Some(0);
    }
    let q: Vec<char> = query.chars().flat_map(|c| c.to_lowercase()).collect();
    let t: Vec<char> = target.chars().flat_map(|c| c.to_lowercase()).collect();

    let mut qi = 0;
    let mut score = 0u32;
    let mut prev_match: Option<usize> = None;

    for (ti, &tc) in t.iter().enumerate() {
        if qi < q.len() && tc == q[qi] {
            qi += 1;
            score += if let Some(prev) = prev_match {
                if ti == prev + 1 { 20 } else { 10 }
            } else {
                15
            };
            if ti < t.len() / 3 {
                score += 5;
            }
            prev_match = Some(ti);
        }
    }
    if qi == q.len() { Some(score) } else { None }
}

/// 对所有条目运行搜索，返回 (索引, 分数) 降序列表
pub fn run_search(query: &str, entries: &[PasswordEntry]) -> Vec<(usize, u32)> {
    if query.is_empty() || query.starts_with("<cmd>") {
        return vec![];
    }
    let mut results: Vec<(usize, u32)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let name = fuzzy_match(query, &e.name);
            let pass = fuzzy_match(query, &e.password);
            match (name, pass) {
                (Some(a), Some(b)) => Some((i, a.max(b) + 2)),
                (Some(a), None) => Some((i, a)),
                (None, Some(b)) => Some((i, b.saturating_sub(1))),
                (None, None) => None,
            }
        })
        .collect();
    results.sort_by(|a, b| b.1.cmp(&a.1));
    results
}
