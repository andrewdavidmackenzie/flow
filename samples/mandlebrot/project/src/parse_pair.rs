use std::str::FromStr;

/// Parse the string 's' as a coordinate pair, like "400x600" or "1.0,0.5"
/// Specifically, 's' should have the form <left><sep><right> where <sep> is the character given by
/// the 'separator' argument, and <left> and <right> are both strings that can be parsed
/// by 'T::from_str'.
/// If 's' has the proper form, return 'Some<(x,y)>'.
/// If 's' doesn't parse correctly, return None.
pub fn _parse_pair<T: FromStr>(s: &str, separator: &str) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
                (Ok(l), Ok(r)) => Some((l, r)),
                _ => None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::_parse_pair;

    #[test]
    fn test_parse_pair() {
        assert_eq!(_parse_pair::<i32>("", ","), None);
        assert_eq!(_parse_pair::<i32>("10,", ","), None);
        assert_eq!(_parse_pair::<i32>(",10", ","), None);
        assert_eq!(_parse_pair::<i32>("10,20", ","), Some((10, 20)));
        assert_eq!(_parse_pair::<i32>("10,20xy", ","), None);
        assert_eq!(_parse_pair::<f64>("0.5x", ","), None);
        assert_eq!(_parse_pair::<f64>("0.5x1.5", "x"), Some((0.5, 1.5)));
    }
}


