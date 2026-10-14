//! Word error rate for repeatable accuracy measurements (P0 measurement requirements).

/// Lowercase and drop punctuation so the score reflects recognized words rather than formatting.
pub fn normalize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '\'' || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(|w| w.trim_matches('\'').to_string())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Word-level Levenshtein distance / reference length. Counts (errors, reference words).
pub fn word_errors(reference: &str, hypothesis: &str) -> (usize, usize) {
    let r = normalize(reference);
    let h = normalize(hypothesis);
    let mut prev: Vec<usize> = (0..=h.len()).collect();
    let mut cur = vec![0; h.len() + 1];
    for i in 1..=r.len() {
        cur[0] = i;
        for j in 1..=h.len() {
            let sub = prev[j - 1] + usize::from(r[i - 1] != h[j - 1]);
            cur[j] = sub.min(prev[j] + 1).min(cur[j - 1] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    (prev[h.len()], r.len())
}

pub fn wer(reference: &str, hypothesis: &str) -> f32 {
    let (e, n) = word_errors(reference, hypothesis);
    if n == 0 {
        return if normalize(hypothesis).is_empty() { 0.0 } else { 1.0 };
    }
    e as f32 / n as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_ignoring_case_and_punctuation() {
        assert_eq!(wer("Hello, world!", "hello world"), 0.0);
        assert_eq!(wer("Don't stop.", "don't stop"), 0.0);
    }

    #[test]
    fn counts_substitution_insertion_deletion() {
        assert_eq!(word_errors("a b c d", "a x c d"), (1, 4));
        assert_eq!(word_errors("a b c d", "a b c d e"), (1, 4));
        assert_eq!(word_errors("a b c d", "a c d"), (1, 4));
        assert_eq!(word_errors("a b", ""), (2, 2));
    }

    #[test]
    fn empty_reference() {
        assert_eq!(wer("", ""), 0.0);
        assert_eq!(wer("", "noise"), 1.0);
    }
}
