
#[derive(Clone, Default)]
pub struct TextChange {
    pub change_type: ChangeType,
    pub x: usize,
    pub y: usize,
    pub front: Option<Vec<char>>,
    pub back: Option<Vec<char>>,
    pub c: Option<char>,
    pub undotype: UndoType,
}

#[derive(Default, Clone, PartialEq)]
pub enum UndoType {
    #[default]Undo,
    Redo,
}

impl TextChange {
    pub fn new(change: ChangeType, x: usize, y: usize, c: Option<char>, front: Option<Vec<char>>, back: Option<Vec<char>>) -> Self {        
        Self {
            change_type: change,
            x: x,
            y: y,
            c: c,
            front: front,
            back: back,
            undotype: UndoType::Undo
        }
    }
    pub fn new_redo(change: ChangeType, x: usize, y: usize, c: Option<char>, front: Option<Vec<char>>, back: Option<Vec<char>>) -> Self {        
        Self {
            change_type: change,
            x: x,
            y: y,
            c: c,
            front: front,
            back: back,
            undotype: UndoType::Redo
        }
    }
}


#[derive(Clone, Default)]
pub enum ChangeType {
    Character,
    Removal,
    Enter,
    #[default] Nothing,
}

#[derive(Default)]
pub struct UndoRedo {
    changes: Vec<TextChange>,
    pub redos: Vec<TextChange>,
}

impl UndoRedo {
    pub fn add_change(&mut self, change: TextChange) {
        self.changes.push(change);
        if !self.redos.is_empty() {
            self.redos = Vec::new();
        }
        let mut index = 0;
        for i in self.changes.clone() {
            if i.undotype == UndoType::Redo {
                self.changes.remove(index);
                index += 1;
            }
        }
    }

    pub fn undo(&mut self) -> TextChange {
        if !self.changes.is_empty() {
           let change = self.changes.pop().unwrap();
            self.redos.push(change.clone());
            change
        }
        else {
            TextChange::default()
        }

    }
    pub fn redo(&mut self) -> TextChange {
        let redo = self.redos.pop().unwrap();
        self.changes.push(redo.clone());
        redo
    }
}

