#[derive(Debug, PartialEq, Eq)]
pub struct Argument<'a> {
    pub name: &'a [u8],
    pub value: Option<&'a [u8]>,
}

pub fn parse(arg: &[u8]) -> Option<Argument<'_>> {
    let arg = arg.strip_prefix(b"--")?;
    let mut parts = arg.splitn(2, |byte| *byte == b'=');
    let name = parts.next()?;
    if name.is_empty() {
        return None;
    }
    let value = parts.next();
    Some(Argument { name, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_argument_with_value() {
        assert_eq!(
            parse(b"--qnum=200"),
            Some(Argument {
                name: b"qnum",
                value: Some(b"200"),
            })
        );
    }

    #[test]
    fn parses_argument_without_value() {
        assert_eq!(
            parse(b"--debug"),
            Some(Argument {
                name: b"debug",
                value: None,
            })
        );
    }

    #[test]
    fn preserves_empty_value() {
        assert_eq!(
            parse(b"--payload="),
            Some(Argument {
                name: b"payload",
                value: Some(b""),
            })
        );
    }

    #[test]
    fn preserves_additional_equals_in_value() {
        assert_eq!(
            parse(b"--example=a=b=c"),
            Some(Argument {
                name: b"example",
                value: Some(b"a=b=c"),
            })
        );
    }

    #[test]
    fn rejects_argument_without_double_dash() {
        assert_eq!(parse(b"qnum=200"), None);
    }

    #[test]
    fn rejects_empty_name() {
        assert_eq!(parse(b"--"), None);
    }

    #[test]
    fn rejects_empty_argument() {
        assert_eq!(parse(b""), None);
    }
}
