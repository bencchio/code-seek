pub(crate) struct LocMap {
    prefix: Vec<usize>,
}

impl LocMap {
    pub(crate) fn build(source: &str) -> Self {
        let lines: Vec<&str> = source.lines().collect();
        let n = lines.len();
        let mut prefix = vec![0usize; n + 1];
        let mut in_block = false;
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            let is_loc = if in_block {
                if t.contains("*/") {
                    in_block = false;
                }
                false
            } else if t.is_empty() || t.starts_with("//") || t.starts_with('*') {
                false
            } else if t.starts_with("/*") {
                if !t.contains("*/") {
                    in_block = true;
                }
                false
            } else {
                if let Some(pos) = t.find("/*")
                    && !t[pos..].contains("*/")
                {
                    in_block = true;
                }
                true
            };
            prefix[i + 1] = prefix[i] + is_loc as usize;
        }
        Self { prefix }
    }

    pub(crate) fn count(&self, start_line: usize, end_line: usize) -> usize {
        let n = self.prefix.len().saturating_sub(1);
        let e = end_line.min(n);
        let s = start_line.saturating_sub(1);
        self.prefix[e].saturating_sub(self.prefix[s])
    }

    pub(crate) fn total(&self) -> usize {
        *self.prefix.last().unwrap_or(&0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loc_map_counts_code_lines() {
        let src = "int x = 1;\n\n// comment\n/* block */\n* continuation\nint y = 2;\n";
        let map = LocMap::build(src);
        assert_eq!(map.total(), 2);
    }

    #[test]
    fn loc_map_skips_multiline_block_comment() {
        let src = "int x = 1;\n/*\n   inside comment\n   no star prefix\n*/\nint y = 2;\n";
        let map = LocMap::build(src);
        assert_eq!(map.total(), 2);
    }

    #[test]
    fn loc_map_range_is_inclusive() {
        let src = "line1\nline2\nline3\nline4\n";
        let map = LocMap::build(src);
        assert_eq!(map.count(2, 3), 2);
    }

    #[test]
    fn loc_map_single_line() {
        let src = "int x = 1;\n\nint y = 2;\n";
        let map = LocMap::build(src);
        assert_eq!(map.count(1, 1), 1);
        assert_eq!(map.count(2, 2), 0);
        assert_eq!(map.count(3, 3), 1);
    }

    #[test]
    fn loc_map_total_matches_full_range() {
        let src = "a\nb\n\nc\n";
        let map = LocMap::build(src);
        assert_eq!(map.count(1, 4), map.total());
    }
}
