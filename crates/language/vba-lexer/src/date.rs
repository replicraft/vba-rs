//! Shape check for `DATE` token contents (MS-VBAL §3.3.3).
//!
//! Only the lexical grammar is checked (`date-or-time`). The numeric range
//! rules (legal month and day, hour 0-23, ...) are static semantics and are
//! not applied here.

const MONTHS: [&[u8]; 12] = [
    b"january",
    b"february",
    b"march",
    b"april",
    b"may",
    b"june",
    b"july",
    b"august",
    b"september",
    b"october",
    b"november",
    b"december",
];

const MONTH_ABBREVIATIONS: [&[u8]; 11] = [
    b"jan", b"feb", b"mar", b"apr", b"jun", b"jul", b"aug", b"sep", b"oct", b"nov", b"dec",
];

pub(crate) fn is_wsc(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | 0x19)
}

/// Bytes that may appear between the `#` of a date literal.
pub(crate) fn is_date_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || is_wsc(b) || matches!(b, b'/' | b'-' | b',' | b':' | b'.')
}

fn skip_ws(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && is_wsc(s[i]) {
        i += 1;
    }
    i
}

fn digits(s: &[u8], i: usize) -> Option<usize> {
    let n = s[i..].iter().take_while(|b| b.is_ascii_digit()).count();
    (n > 0).then_some(i + n)
}

/// A `decimal-literal` or `month-name`; returns the end and whether it is a month.
fn date_item(s: &[u8], i: usize) -> Option<(usize, bool)> {
    if let Some(e) = digits(s, i) {
        return Some((e, false));
    }
    let n = s[i..]
        .iter()
        .take_while(|b| b.is_ascii_alphabetic())
        .count();
    if n == 0 {
        return None;
    }
    let word = s[i..i + n].to_ascii_lowercase();
    let known = MONTHS.contains(&&word[..]) || MONTH_ABBREVIATIONS.contains(&&word[..]);
    known.then_some((i + n, true))
}

/// `date-separator` alternatives starting at `i`: `1*WSC` and
/// `*WSC ("/" / "-" / ",") *WSC`.
fn date_separators(s: &[u8], i: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let j = skip_ws(s, i);
    if j > i {
        out.push(j);
    }
    if j < s.len() && matches!(s[j], b'/' | b'-' | b',') {
        out.push(skip_ws(s, j + 1));
    }
    out
}

/// All possible ends of a `date-value` starting at `i`.
fn date_ends(s: &[u8], i: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    let Some((l_end, l_month)) = date_item(s, i) else {
        return ends;
    };
    for s1 in date_separators(s, l_end) {
        let Some((m_end, m_month)) = date_item(s, s1) else {
            continue;
        };
        if l_month && m_month {
            continue;
        }
        ends.push(m_end);
        for s2 in date_separators(s, m_end) {
            if let Some((r_end, r_month)) = date_item(s, s2)
                && u8::from(l_month) + u8::from(m_month) + u8::from(r_month) <= 1
            {
                ends.push(r_end);
            }
        }
    }
    ends
}

/// `ampm = *WSC ("am" / "pm" / "a" / "p")`, returns the end if present.
fn ampm(s: &[u8], i: usize) -> Option<usize> {
    let j = skip_ws(s, i);
    let n = s[j..]
        .iter()
        .take_while(|b| b.is_ascii_alphabetic())
        .count();
    let w = s[j..j + n].to_ascii_lowercase();
    matches!(&w[..], b"am" | b"pm" | b"a" | b"p").then_some(j + n)
}

/// `time-separator = *WSC (":" / ".") *WSC`, returns the end if present.
fn time_sep(s: &[u8], i: usize) -> Option<usize> {
    let j = skip_ws(s, i);
    (j < s.len() && matches!(s[j], b':' | b'.')).then(|| skip_ws(s, j + 1))
}

/// All possible ends of a `time-value` starting at `i`.
fn time_ends(s: &[u8], i: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    let Some(h_end) = digits(s, i) else {
        return ends;
    };
    if let Some(e) = ampm(s, h_end) {
        ends.push(e);
    }
    let Some(m_start) = time_sep(s, h_end) else {
        return ends;
    };
    let Some(m_end) = digits(s, m_start) else {
        return ends;
    };
    let mut bases = vec![m_end];
    if let Some(s_start) = time_sep(s, m_end)
        && let Some(s_end) = digits(s, s_start)
    {
        bases.push(s_end);
    }
    for b in bases {
        ends.push(b);
        if let Some(e) = ampm(s, b) {
            ends.push(e);
        }
    }
    ends
}

/// Whether `content` (the bytes between the two `#`) matches `date-or-time`,
/// allowing surrounding whitespace.
pub(crate) fn is_date_or_time(content: &[u8]) -> bool {
    let mut start = 0;
    let mut end = content.len();
    while start < end && is_wsc(content[start]) {
        start += 1;
    }
    while end > start && is_wsc(content[end - 1]) {
        end -= 1;
    }
    let s = &content[start..end];
    if s.is_empty() {
        return false;
    }
    if time_ends(s, 0).contains(&s.len()) {
        return true;
    }
    for e in date_ends(s, 0) {
        if e == s.len() {
            return true;
        }
        let j = skip_ws(s, e);
        if j > e && time_ends(s, j).contains(&s.len()) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::is_date_or_time as ok;

    #[test]
    fn valid() {
        for s in [
            "1/2/2000",
            "1-2-2000",
            "1,2,2000",
            "1 2 2000",
            "12/31/99",
            "1/2",
            "Jan 1 2000",
            "January 1, 2000",
            "1 jan 2000",
            "jan/2000",
            "2000/jan/1",
            "10:30",
            "10:30:15",
            "10.30",
            "10 pm",
            "10pm",
            "10:30 AM",
            "10:30:15 p",
            "1/2/2000 10:30",
            "1/2/2000 10:30 PM",
            "Mar 5 8am",
            " 1/2/2000 ",
            "10 : 30",
        ] {
            assert!(ok(s.as_bytes()), "{s:?} should be a date-or-time");
        }
    }

    #[test]
    fn invalid() {
        for s in [
            "",
            " ",
            "10",
            "abc",
            "1/",
            "/1/2",
            "1//2",
            "jan/feb/2000",
            "1/2/3/4",
            "10:",
            "10:30:",
            "10 xm",
            "1, a: Print",
            "1",
            "jan",
            "1/2/2000 foo",
            "5 + 3",
        ] {
            assert!(!ok(s.as_bytes()), "{s:?} should not be a date-or-time");
        }
    }
}
