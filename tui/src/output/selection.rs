use crate::search::types::SearchItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionOutput {
    Item {
        item: SearchItem,
        column: Option<usize>,
    },
    PreviewLocation {
        path: String,
        row: usize,
        column: usize,
    },
}
