#[derive(Debug, PartialEq, Eq)]
pub struct LuaDesync<'a> {
    pub function: &'a [u8],
    pub params: Vec<&'a [u8]>,
}

pub fn parse(value: &[u8]) -> Option<LuaDesync<'_>> {
    let mut parts = value.split(|byte| *byte == b':');

    let function = parts.next()?;
    if function.is_empty() {
        return None;
    }

    let params: Vec<&[u8]> = parts.collect();
    if params.iter().any(|param| param.is_empty()) {
        return None;
    }

    Some(LuaDesync { function, params })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_desync_with_multiple_params() {
        assert_eq!(
            parse(b"wssize:wsize=1:scale=6"),
            Some(LuaDesync {
                function: b"wssize",
                params: vec![b"wsize=1", b"scale=6"],
            })
        );
    }

    #[test]
    fn preserves_commas_inside_param_value() {
        assert_eq!(
            parse(b"multidisorder:pos=1,midsld"),
            Some(LuaDesync {
                function: b"multidisorder",
                params: vec![b"pos=1,midsld"],
            })
        );
    }

    #[test]
    fn parses_desync_without_params() {
        assert_eq!(
            parse(b"syndata"),
            Some(LuaDesync {
                function: b"syndata",
                params: vec![],
            })
        );
    }

    #[test]
    fn preserves_bare_params() {
        assert_eq!(
            parse(b"syndata:multisplit:strategy=23"),
            Some(LuaDesync {
                function: b"syndata",
                params: vec![b"multisplit", b"strategy=23"],
            })
        );
    }

    #[test]
    fn rejects_empty_value() {
        assert_eq!(parse(b""), None);
    }

    #[test]
    fn rejects_empty_function() {
        assert_eq!(parse(b":wsize=1"), None);
    }

    #[test]
    fn rejects_empty_param() {
        assert_eq!(parse(b"wssize::scale=6"), None);
    }
}
