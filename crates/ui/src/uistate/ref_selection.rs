use crate::domain::NObject;

#[derive(Debug)]
pub enum RefSelection {
    None,
    Reference(usize),
    ReverseReverence(usize),
}

impl RefSelection {
    pub fn init_from_node(&mut self, node: &NObject) {
        if !node.references.is_empty() {
            *self = RefSelection::Reference(0);
        } else if !node.reverse_references.is_empty() {
            *self = RefSelection::ReverseReverence(0);
        } else {
            *self = RefSelection::None;
        }
    }
    pub fn ref_index(&self, is_reverse: bool) -> Option<usize> {
        match self {
            RefSelection::None => None,
            RefSelection::Reference(idx) => {
                if is_reverse {
                    None
                } else {
                    Some(*idx)
                }
            }
            RefSelection::ReverseReverence(idx) => {
                if is_reverse {
                    Some(*idx)
                } else {
                    None
                }
            }
        }
    }
    pub fn move_up(&mut self) {
        match self {
            RefSelection::None => {}
            RefSelection::Reference(idx) => {
                if *idx > 0 {
                    *idx -= 1;
                }
            }
            RefSelection::ReverseReverence(idx) => {
                if *idx > 0 {
                    *idx -= 1;
                }
            }
        }
    }
    pub fn move_down(&mut self, node: &NObject) {
        match self {
            RefSelection::None => {}
            RefSelection::Reference(idx) => {
                if *idx < node.references.len() - 1 {
                    *idx += 1;
                }
            }
            RefSelection::ReverseReverence(idx) => {
                if *idx < node.reverse_references.len() - 1 {
                    *idx += 1;
                }
            }
        }
    }
    pub fn move_right(&mut self, node: &NObject) {
        match self {
            RefSelection::None => {}
            RefSelection::Reference(idx) => {
                if !node.reverse_references.is_empty() {
                    if *idx > node.reverse_references.len() - 1 {
                        *idx = node.reverse_references.len() - 1;
                    }
                    *self = RefSelection::ReverseReverence(*idx);
                }
            }
            RefSelection::ReverseReverence(_) => {}
        }
    }
    pub fn move_left(&mut self, node: &NObject) {
        match self {
            RefSelection::None => {}
            RefSelection::Reference(_) => {}
            RefSelection::ReverseReverence(idx) => {
                if !node.references.is_empty() {
                    if *idx > node.references.len() - 1 {
                        *idx = node.references.len() - 1;
                    }
                    *self = RefSelection::Reference(*idx);
                }
            }
        }
    }
}
