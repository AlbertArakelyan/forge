/// Parse all `{{var}}` spans in `input`.
/// Returns a list of `(start_byte, end_byte, var_name)` where start/end are byte
/// offsets in the original string (inclusive of the `{{` and `}}` delimiters).
/// Empty names and unclosed braces are skipped.
pub fn parse_vars(input: &str) -> Vec<(usize, usize, String)> {
    let mut result = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 1 < len {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            let start = i;
            let inner_start = i + 2;
            // Search for closing '}}'
            let mut j = inner_start;
            let mut found = false;
            while j + 1 < len {
                if bytes[j] == b'}' && bytes[j + 1] == b'}' {
                    found = true;
                    break;
                }
                j += 1;
            }
            if found {
                let name = &input[inner_start..j];
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    result.push((start, j + 2, trimmed.to_string()));
                }
                i = j + 2;
            } else {
                // Unclosed — skip
                break;
            }
        } else {
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vars_basic() {
        let spans = parse_vars("{{host}}/api");
        assert_eq!(spans.len(), 1);
        let (start, end, name) = &spans[0];
        assert_eq!(*start, 0);
        assert_eq!(*end, 8); // "{{host}}" is 8 bytes
        assert_eq!(name, "host");
        // Verify the slice matches
        assert_eq!(&"{{host}}/api"[*start..*end], "{{host}}");
    }

    #[test]
    fn test_parse_vars_missing_close() {
        let spans = parse_vars("{{host");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_parse_vars_empty_name() {
        let spans = parse_vars("{{}}rest");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_parse_vars_multiple() {
        let spans = parse_vars("{{scheme}}://{{host}}/path");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].2, "scheme");
        assert_eq!(spans[1].2, "host");
    }

    #[test]
    fn test_parse_vars_no_vars() {
        let spans = parse_vars("https://example.com/api");
        assert!(spans.is_empty());
    }
}
