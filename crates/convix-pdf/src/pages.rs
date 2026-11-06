//! Page lists typed by the user: "1-3, 5, 8-" (1-based, empty = every page).

use convix_core::{Error, Result};

/// Pages selected among `count`, in the order typed, without duplicates.
pub fn parse(spec: &str, count: u32) -> Result<Vec<u32>> {
    let invalid = |reason: String| Error::InvalidOption { name: "pages".into(), reason };
    if spec.trim().is_empty() {
        return Ok((1..=count).collect());
    }
    let mut pages = Vec::new();
    for part in spec.split([',', ';']).map(str::trim).filter(|p| !p.is_empty()) {
        let number = |s: &str| -> Result<u32> {
            s.trim().parse::<u32>().map_err(|_| invalid(format!("« {part} » n'est pas une page ou une plage (ex. 1-3, 5)")))
        };
        let (from, to) = match part.split_once('-') {
            Some((a, b)) => (
                if a.trim().is_empty() { 1 } else { number(a)? },
                if b.trim().is_empty() { count } else { number(b)? },
            ),
            None => {
                let n = number(part)?;
                (n, n)
            }
        };
        if from == 0 || from > count || to > count || from > to {
            return Err(invalid(format!("« {part} » : le document a {count} page(s)")));
        }
        for p in from..=to {
            if !pages.contains(&p) {
                pages.push(p);
            }
        }
    }
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(parse("", 3).unwrap(), vec![1, 2, 3]);
        assert_eq!(parse("1-3, 5", 6).unwrap(), vec![1, 2, 3, 5]);
        assert_eq!(parse("4-, 1", 6).unwrap(), vec![4, 5, 6, 1]);
        assert_eq!(parse("-2;2", 6).unwrap(), vec![1, 2]);
        assert!(parse("7", 6).is_err());
        assert!(parse("3-1", 6).is_err());
        assert!(parse("abc", 6).is_err());
    }
}
