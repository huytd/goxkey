use std::collections::HashMap;

use zbus::zvariant::{Array, Structure, Value};

use super::ibus_serde::make_ibus_text;

#[derive(Debug, Copy, Clone)]
pub enum IBusOrientation {
    Horizontal,
    Vertical,
    /// Use ibus global orientation setup.
    System,
}

impl From<IBusOrientation> for i32 {
    fn from(value: IBusOrientation) -> Self {
        match value {
            IBusOrientation::Horizontal => 0,
            IBusOrientation::Vertical => 1,
            IBusOrientation::System => 2,
        }
    }
}

#[derive(Debug, Clone)]
/// A table of strings ("candidates") that IBus displays to the user
///
/// The user can then scroll through the candidates and select one.
/// IBus automatically paginates candidates by groups of 1 to 16.
pub struct LookupTable {
    candidates: Vec<String>,
    /// Labels for items in a page
    ///
    /// If this vector is empty (the default), items are labelled 1 through f.
    /// Populating this vector allows to customize labels.
    /// Note that labels are the same on every page, they do not label candidates.
    pub labels: Vec<String>,
    page_size: u32,
    cursor_pos: u32,
    pub cursor_visible: bool,
    /// If true, scrolling beyond the end of the lookup table wraps to the beginning
    pub round: bool,
    pub orientation: IBusOrientation,
}

impl LookupTable {
    /// Creates a lookup table with cursor at position 0.
    ///
    /// Returns an error if page size is not in `range 1..=16`
    pub fn new(
        candidates: Vec<String>,
        page_size: u32,
        cursor_visible: bool,
        round: bool,
    ) -> Result<Self, ()> {
        if page_size == 0 || page_size > 16 {
            return Err(());
        }
        Ok(LookupTable {
            candidates,
            labels: vec![],
            page_size,
            cursor_pos: 0,
            cursor_visible,
            round,
            orientation: IBusOrientation::System,
        })
    }

    pub(crate) fn serialize(&self) -> Value<'static> {
        let special = HashMap::<String, Value<'static>>::new();
        let candidates: Array = self
            .candidates
            .iter()
            .cloned()
            .map(make_ibus_text)
            .collect::<Vec<Value<'static>>>()
            .into();
        let labels: Array = self
            .labels
            .iter()
            .cloned()
            .map(make_ibus_text)
            .collect::<Vec<Value<'static>>>()
            .into();
        let structure = Structure::from((
            "IBusLookupTable",
            // a{sv}
            special,
            self.page_size,
            self.cursor_pos,
            self.cursor_visible,
            self.round,
            i32::from(self.orientation),
            candidates,
            labels,
        ));
        Value::new(structure)
    }

    /// Sets the cursor position, making sure it remains in bound
    #[inline]
    pub fn set_cursor_pos(&mut self, desired_pos: i64) {
        if self.round {
            if self.candidates.is_empty() {
                self.cursor_pos = 0
            } else {
                self.cursor_pos = desired_pos.rem_euclid(self.candidates.len() as i64) as u32
            }
        } else if desired_pos < 0 {
            self.cursor_pos = 0
        } else if desired_pos >= self.candidates.len() as i64 {
            self.cursor_pos = (self.candidates.len() as u32).saturating_sub(1)
        } else {
            self.cursor_pos = desired_pos as u32
        }
    }

    /// Sets the cursor position to the next element
    ///
    /// Wraps around if round is true
    pub fn cursor_down(&mut self) {
        self.set_cursor_pos(self.cursor_pos as i64 + 1)
    }

    /// Sets the cursor position to the previous element
    ///
    /// Wraps around if round is true
    pub fn cursor_up(&mut self) {
        self.set_cursor_pos(self.cursor_pos as i64 - 1)
    }

    /// Sets the cursor position on an element of the next page
    pub fn page_down(&mut self) {
        self.set_cursor_pos(
            self.cursor_pos as i64 - (self.cursor_pos % self.page_size) as i64
                + self.page_size as i64,
        )
    }

    /// Sets the cursor position on an element of the previous page
    pub fn page_up(&mut self) {
        self.set_cursor_pos(self.cursor_pos as i64 - (self.cursor_pos % self.page_size) as i64 - 1)
    }

    /// The index of the currently selected candidate
    pub fn cursor_pos(&self) -> u32 {
        self.cursor_pos
    }

    /// The number of elements shown per page.
    pub fn page_size(&self) -> u32 {
        self.page_size
    }

    /// The 0-based index of the currently selected element in its page.
    ///
    /// If default labels are displayed, this corresponds to the label minus one
    pub fn cursor_pos_in_page(&self) -> u32 {
        self.cursor_pos % self.page_size
    }

    /// Returns the candidate at `index_in_page` in the current page
    pub fn get_candidate_by_index_in_page(&self, index_in_page: u32) -> Option<&String> {
        if index_in_page >= self.page_size {
            None
        } else {
            let page = self.cursor_pos / self.page_size;
            let index = self.page_size * page + index_in_page;
            self.candidates.get(index as usize)
        }
    }

    /// Sets the page size. Fails if the page size is 0 or more than 16.
    #[inline]
    pub fn set_page_size(&mut self, page_size: u32) -> Result<(), ()> {
        if page_size == 0 || page_size > 16 {
            return Err(());
        }
        self.page_size = page_size;
        Ok(())
    }

    /// Modifies the list of candidates
    ///
    /// Moves the cursor if the list of candidates is made shorter.
    #[inline]
    pub fn modify_candidates(&mut self, f: impl FnOnce(&mut Vec<String>)) {
        f(&mut self.candidates);
        // if the cursor position is out of bound, make it in bound. Otherwise ibus makes the
        // lookup table disappear
        self.cursor_pos = self
            .cursor_pos
            .min(self.candidates.len().saturating_sub(1) as u32);
    }

    /// Returns the current list of candidates
    pub fn candidates(&self) -> &[String] {
        &self.candidates[..]
    }

    /// removes all candidates
    pub fn clear(&mut self) {
        self.modify_candidates(|c| c.clear());
    }

    /// adds one candidate to the end of the list
    pub fn push_candidate(&mut self, candidate: String) {
        self.modify_candidates(|c| c.push(candidate))
    }

    /// Replaces the list of candidates by this one, and set the cursor to the beginning
    pub fn reset_candidates(&mut self, candidates: Vec<String>) {
        self.candidates = candidates;
        self.cursor_pos = 0;
    }
}

impl Extend<String> for LookupTable {
    fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
        self.modify_candidates(|c| c.extend(iter))
    }
}

#[test]
fn lookup_table_zero_page_size() {
    assert!(LookupTable::new(vec![], 0, false, false).is_err());
}
#[test]
fn lookup_table_large_page_size() {
    assert!(LookupTable::new(vec![], 17, false, false).is_err());
}
#[test]
fn cursor_down_saturate() {
    let mut table =
        LookupTable::new(vec!["one".to_string(), "two".to_string()], 1, false, false).unwrap();
    table.set_cursor_pos(1);
    assert_eq!(table.cursor_pos(), 1);
    table.cursor_down();
    assert_eq!(table.cursor_pos(), 1);
}
#[test]
fn cursor_down_wrap() {
    let mut table =
        LookupTable::new(vec!["one".to_string(), "two".to_string()], 1, false, true).unwrap();
    table.set_cursor_pos(1);
    assert_eq!(table.cursor_pos(), 1);
    table.cursor_down();
    assert_eq!(table.cursor_pos(), 0);
}
#[test]
fn cursor_down_nominal() {
    let mut table =
        LookupTable::new(vec!["one".to_string(), "two".to_string()], 1, false, true).unwrap();
    table.cursor_down();
    assert_eq!(table.cursor_pos(), 1);
}
#[test]
fn cursor_up_saturate() {
    let mut table =
        LookupTable::new(vec!["one".to_string(), "two".to_string()], 1, false, false).unwrap();
    table.cursor_up();
    assert_eq!(table.cursor_pos(), 0);
}
#[test]
fn cursor_up_wrap() {
    let mut table =
        LookupTable::new(vec!["one".to_string(), "two".to_string()], 1, false, true).unwrap();
    table.cursor_up();
    assert_eq!(table.cursor_pos(), 1);
}
#[test]
fn cursor_up_nominal() {
    let mut table =
        LookupTable::new(vec!["one".to_string(), "two".to_string()], 1, false, true).unwrap();
    table.set_cursor_pos(1);
    assert_eq!(table.cursor_pos(), 1);
    table.cursor_up();
    assert_eq!(table.cursor_pos(), 0);
}

#[test]
fn page_up_nominal() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(2);
    assert_eq!(table.cursor_pos(), 2);
    table.page_up();
    assert_eq!(table.cursor_pos(), 1);
}
#[test]
fn page_up_saturate() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        false,
    )
    .unwrap();
    table.set_cursor_pos(1);
    assert_eq!(table.cursor_pos(), 1);
    table.page_up();
    assert_eq!(table.cursor_pos(), 0);
}
#[test]
fn page_up_wrap() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(1);
    assert_eq!(table.cursor_pos(), 1);
    table.page_up();
    assert_eq!(table.cursor_pos(), 3);
}
#[test]
fn page_up_wrap_empty() {
    let mut table = LookupTable::new(vec![], 2, false, true).unwrap();
    table.page_up();
    assert_eq!(table.cursor_pos(), 0);
}
#[test]
fn page_down_nominal() {
    let mut table = LookupTable::new(
        vec!["one".to_string(), "two".to_string(), "three".to_string()],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(1);
    assert_eq!(table.cursor_pos(), 1);
    table.page_down();
    assert_eq!(table.cursor_pos(), 2);
}
#[test]
fn page_down_saturate() {
    let mut table = LookupTable::new(
        vec!["one".to_string(), "two".to_string(), "three".to_string()],
        2,
        false,
        false,
    )
    .unwrap();
    table.set_cursor_pos(2);
    assert_eq!(table.cursor_pos(), 2);
    table.page_down();
    assert_eq!(table.cursor_pos(), 2);
}
#[test]
fn page_down_wrap() {
    let mut table = LookupTable::new(
        vec!["one".to_string(), "two".to_string(), "three".to_string()],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(2);
    assert_eq!(table.cursor_pos(), 2);
    table.page_down();
    assert_eq!(table.cursor_pos(), 1);
}
#[test]
fn cursor_pos_in_page() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(2);
    assert_eq!(table.cursor_pos(), 2);
    assert_eq!(table.cursor_pos_in_page(), 0);
    table.set_cursor_pos(3);
    assert_eq!(table.cursor_pos(), 3);
    assert_eq!(table.cursor_pos_in_page(), 1);
}

#[test]
fn get_candidate_by_index_in_page() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    assert_eq!(
        table.get_candidate_by_index_in_page(0),
        Some(&"one".to_string())
    );
    assert_eq!(
        table.get_candidate_by_index_in_page(1),
        Some(&"two".to_string())
    );
    assert_eq!(table.get_candidate_by_index_in_page(2), None);
    table.set_cursor_pos(3);
    assert_eq!(
        table.get_candidate_by_index_in_page(0),
        Some(&"three".to_string())
    );
    assert_eq!(
        table.get_candidate_by_index_in_page(1),
        Some(&"four".to_string())
    );
    assert_eq!(table.get_candidate_by_index_in_page(2), None);
}

#[test]
fn set_page_size() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    assert_eq!(table.set_page_size(0), Err(()));
    assert_eq!(table.set_page_size(17), Err(()));
    assert_eq!(table.set_page_size(3), Ok(()));
    assert_eq!(table.page_size(), 3)
}

#[test]
fn modify_candidates_nominal() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(3);
    table.modify_candidates(|c| {
        for candidate in c.iter_mut() {
            candidate.push('!');
        }
        c.push("five!!!".to_string());
    });
    assert_eq!(table.cursor_pos(), 3);
    assert_eq!(
        table.candidates(),
        &["one!", "two!", "three!", "four!", "five!!!"]
    );
}

#[test]
fn modify_candidates_smaller() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(3);
    table.modify_candidates(|c| {
        c.pop();
        c.pop();
    });
    assert_eq!(table.candidates(), &["one", "two"]);
    assert_eq!(table.cursor_pos(), 1);
}

#[test]
fn clear() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(3);
    table.clear();
    assert_eq!(table.cursor_pos(), 0);
    assert_eq!(table.candidates(), &Vec::<String>::new());
}

#[test]
fn push_candidate() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(3);
    table.push_candidate("five".to_string());
    assert_eq!(table.cursor_pos(), 3);
    assert_eq!(table.candidates(), &["one", "two", "three", "four", "five"]);
}

#[test]
fn reset_candidates() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(3);
    table.reset_candidates(vec!["new1".to_string(), "new2".to_string()]);
    assert_eq!(table.cursor_pos(), 0);
    assert_eq!(table.candidates(), &["new1", "new2"]);
}

#[test]
fn extend() {
    let mut table = LookupTable::new(
        vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ],
        2,
        false,
        true,
    )
    .unwrap();
    table.set_cursor_pos(3);
    table.extend(vec!["new1".to_string(), "new2".to_string()]);
    assert_eq!(table.cursor_pos(), 3);
    assert_eq!(
        table.candidates(),
        &["one", "two", "three", "four", "new1", "new2"]
    );
}
