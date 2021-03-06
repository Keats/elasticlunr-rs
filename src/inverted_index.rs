use std::collections::HashMap;
use serde::ser::{Serialize, SerializeMap, Serializer};

#[derive(Debug, Copy, Clone, Serialize, PartialEq)]
struct TermFrequency {
    #[serde(rename = "tf")]
    term_freq: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct IndexItem {
    docs: HashMap<String, TermFrequency>,
    doc_freq: i64,
    children: HashMap<String, IndexItem>,
}

impl IndexItem {
    fn new() -> Self {
        IndexItem {
            docs: HashMap::new(),
            doc_freq: 0,
            children: HashMap::new(),
        }
    }

    fn add_token(&mut self, doc_ref: &str, token: &str, term_freq: f64) {
        let mut iter = token.char_indices();
        if let Some((_, char)) = iter.next() {
            let item = self.children
                .entry(char.to_string())
                .or_insert(IndexItem::new());
            if let Some((index, _)) = iter.next() {
                item.add_token(doc_ref, &token[index..], term_freq);
            } else {
                // We're at the end of the token, now update info
                if !item.docs.contains_key(doc_ref.into()) {
                    item.doc_freq += 1;
                }
                item.docs.insert(
                    doc_ref.into(),
                    TermFrequency { term_freq },
                );
            }
        }
    }

    #[cfg(test)]
    fn get_node(&self, token: &str) -> Option<&IndexItem> {
        let mut root = self;
        for char in token.chars() {
            if let Some(item) = root.children.get(&char.to_string()) {
                root = item;
            } else {
                return None;
            }
        }

        Some(root)
    }

    #[cfg(test)]
    fn remove_token(&mut self, doc_ref: &str, token: &str) {
        let mut iter = token.char_indices();
        if let Some((_, char)) = iter.next() {
            if let Some(item) = self.children.get_mut(&char.to_string()) {
                if let Some((idx, _)) = iter.next() {
                    item.remove_token(doc_ref, &token[idx..]);
                } else {
                    if item.docs.contains_key(doc_ref) {
                        item.docs.remove(doc_ref);
                        item.doc_freq -= 1;
                    }
                }
            } else {
                return;
            }
        }
    }
}

// Manually implement serialize so `children` are inline
impl Serialize for IndexItem {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_map(Some(2 + self.children.len()))?;
        state.serialize_entry("df", &self.doc_freq)?;
        state.serialize_entry("docs", &self.docs)?;

        for (key, value) in &self.children {
            state.serialize_entry(key, &value)?;
        }

        state.end()
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub struct InvertedIndex {
    root: IndexItem,
}

impl InvertedIndex {
    pub fn new() -> Self {
        InvertedIndex { root: IndexItem::new() }
    }

    pub fn add_token(&mut self, doc_ref: &str, token: &str, term_freq: f64) {
        self.root.add_token(doc_ref, token, term_freq)
    }

    #[cfg(test)]
    pub fn has_token(&self, token: &str) -> bool {
        self.root.get_node(token).map_or(false, |_| true)
    }

    #[cfg(test)]
    pub fn remove_token(&mut self, doc_ref: &str, token: &str) {
        self.root.remove_token(doc_ref, token)
    }

    #[cfg(test)]
    pub fn get_docs(&self, token: &str) -> Option<HashMap<String, f64>> {
        self.root.get_node(token).map(|node| {
            node.docs
                .iter()
                .map(|(k, &v)| (k.clone(), v.term_freq))
                .collect()
        })
    }

    #[cfg(test)]
    pub fn get_term_frequency(&self, doc_ref: &str, token: &str) -> f64 {
        self.root
            .get_node(token)
            .and_then(|node| node.docs.get(doc_ref.into()))
            .map_or(0., |docs| docs.term_freq)
    }

    #[cfg(test)]
    pub fn get_doc_frequency(&self, token: &str) -> i64 {
        self.root.get_node(token).map_or(0, |node| node.doc_freq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_token() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 1.);
        assert_eq!(inverted_index.get_doc_frequency("foo"), 1);
        assert_eq!(inverted_index.get_term_frequency("123", "foo"), 1.);
    }

    #[test]
    fn has_token() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 1.);
        assert!(inverted_index.has_token(token));
        assert!(inverted_index.has_token("fo"));
        assert!(inverted_index.has_token("f"));

        assert!(!inverted_index.has_token("bar"));
        assert!(!inverted_index.has_token("foo "));
        assert!(!inverted_index.has_token("foo  "))
    }

    #[test]
    fn adding_another_document_to_the_token() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 1.);
        inverted_index.add_token("456", token, 1.);

        assert_eq!(inverted_index.get_term_frequency("123", "foo"), 1.);
        assert_eq!(inverted_index.get_term_frequency("456", "foo"), 1.);
        assert_eq!(inverted_index.get_doc_frequency("foo"), 2);
    }

    #[test]
    fn df_of_nonexistant_token() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 1.);
        inverted_index.add_token("456", token, 1.);

        assert_eq!(inverted_index.get_doc_frequency("foo"), 2);
        assert_eq!(inverted_index.get_doc_frequency("fox"), 0);
    }

    #[test]
    fn adding_existing_doc() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 1.);
        inverted_index.add_token("456", token, 1.);
        inverted_index.add_token("456", token, 100.);

        assert_eq!(inverted_index.get_term_frequency("456", "foo"), 100.);
        assert_eq!(inverted_index.get_doc_frequency("foo"), 2);
    }

    #[test]
    fn checking_token_exists_in() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 1.);

        assert!(inverted_index.has_token(token));
    }

    #[test]
    fn checking_if_a_token_does_not_exist() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 1.);
        assert!(!inverted_index.has_token("fooo"));
        assert!(!inverted_index.has_token("bar"));
        assert!(!inverted_index.has_token("fof"));
    }

    #[test]
    fn retrieving_items() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 1.);
        assert_eq!(
            inverted_index.get_docs(token).unwrap(),
            hashmap!{
                "123".into() => 1.
            }
        );

        assert_eq!(inverted_index.get_docs(""), Some(HashMap::new()));

        inverted_index.add_token("234", "boo", 100.);
        inverted_index.add_token("345", "too", 101.);

        assert_eq!(
            inverted_index.get_docs(token).unwrap(),
            hashmap!{
                "123".into() => 1.
            }
        );

        inverted_index.add_token("234", token, 100.);
        inverted_index.add_token("345", token, 101.);

        assert_eq!(
            inverted_index.get_docs(token).unwrap(),
            hashmap!{
                "123".into() => 1.,
                "234".into() => 100.,
                "345".into() => 101.,
            }
        );
    }

    #[test]
    fn retrieving_nonexistant_items() {
        let inverted_index = InvertedIndex::new();

        assert_eq!(inverted_index.get_docs("foo"), None);
        assert_eq!(inverted_index.get_docs("fox"), None);
    }

    #[test]
    fn df_of_items() {
        let mut inverted_index = InvertedIndex::new();

        inverted_index.add_token("123", "foo", 1.);
        inverted_index.add_token("456", "foo", 1.);
        inverted_index.add_token("789", "bar", 1.);

        assert_eq!(inverted_index.get_doc_frequency("foo"), 2);
        assert_eq!(inverted_index.get_doc_frequency("bar"), 1);
        assert_eq!(inverted_index.get_doc_frequency("baz"), 0);
        assert_eq!(inverted_index.get_doc_frequency("ba"), 0);
        assert_eq!(inverted_index.get_doc_frequency("b"), 0);
        assert_eq!(inverted_index.get_doc_frequency("fo"), 0);
        assert_eq!(inverted_index.get_doc_frequency("f"), 0);
    }

    #[test]
    fn removing_document_from_token() {
        let mut inverted_index = InvertedIndex::new();
        assert_eq!(inverted_index.get_docs("foo"), None);

        inverted_index.add_token("123", "foo", 1.);
        assert_eq!(
            inverted_index.get_docs("foo").unwrap(),
            hashmap!{
                "123".into() => 1.,
            }
        );

        inverted_index.remove_token("123", "foo");
        assert_eq!(inverted_index.get_docs("foo"), Some(HashMap::new()));
        assert_eq!(inverted_index.get_doc_frequency("foo"), 0);
        assert_eq!(inverted_index.has_token("foo"), true);
    }

    #[test]
    fn removing_nonexistant_document() {
        let mut inverted_index = InvertedIndex::new();

        inverted_index.add_token("123", "foo", 1.);
        inverted_index.add_token("567", "bar", 1.);
        inverted_index.remove_token("foo", "456");

        assert_eq!(
            inverted_index.get_docs("foo").unwrap(),
            hashmap!{
                "123".into() => 1.
            }
        );
        assert_eq!(inverted_index.get_doc_frequency("foo"), 1);
    }

    #[test]
    fn removing_documet_nonexistant_key() {
        let mut inverted_index = InvertedIndex::new();

        inverted_index.remove_token("123", "foo");
        assert!(!inverted_index.has_token("foo"));
        assert_eq!(inverted_index.get_doc_frequency("foo"), 0);
    }


    #[test]
    fn get_term_frequency() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 2.);
        inverted_index.add_token("456", token, 3.);

        assert_eq!(inverted_index.get_term_frequency("123", token), 2.);
        assert_eq!(inverted_index.get_term_frequency("456", token), 3.);
        assert_eq!(inverted_index.get_term_frequency("789", token), 0.);
    }

    #[test]
    fn get_term_frequency_nonexistant_token() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 2.);
        inverted_index.add_token("456", token, 3.);

        assert_eq!(inverted_index.get_term_frequency("123", "ken"), 0.);
        assert_eq!(inverted_index.get_term_frequency("456", "ken"), 0.);
    }

    #[test]
    fn get_term_frequency_nonexistant_docref() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 2.);
        inverted_index.add_token("456", token, 3.);

        assert_eq!(inverted_index.get_term_frequency(token, "12"), 0.);
        assert_eq!(inverted_index.get_term_frequency(token, "23"), 0.);
        assert_eq!(inverted_index.get_term_frequency(token, "45"), 0.);
    }

    #[test]
    fn get_term_frequency_nonexistant_token_and_docref() {
        let mut inverted_index = InvertedIndex::new();
        let token = "foo";

        inverted_index.add_token("123", token, 2.);
        inverted_index.add_token("456", token, 3.);

        assert_eq!(inverted_index.get_term_frequency("token", "1"), 0.);
        assert_eq!(inverted_index.get_term_frequency("abc", "2"), 0.);
        assert_eq!(inverted_index.get_term_frequency("fo", "123"), 0.);
    }
}
