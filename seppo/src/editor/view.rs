use super::terminal::{Size, Terminal};
mod buffer;
use buffer::Buffer;
mod undoredo;
use undoredo::{UndoRedo, TextChange, ChangeType};
const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct View {
    buffer: Buffer,
    filename: Option<String>,
    needs_redraw: bool,
    size: Size,
    line: Vec<char>,
    undoredo: UndoRedo,
    undoing: bool,
    redoing: bool,
}

impl View {
    pub fn resize(&mut self, to: Size) {
        self.size = to;
        self.needs_redraw = true;
    }
    fn render_line(at: usize, line_text: &str) {
        let result = Terminal::print_row(at, line_text);
        debug_assert!(result.is_ok(), "Failed to render line");
    }

    pub fn render(&mut self) {
        if !self.needs_redraw {
            return;
        }
        let Size { height, width } = self.size;
        if height == 0 || width == 0 {
            return;
        }
        #[allow(clippy::integer_division)]
        let vertical_center = height / 3;

        for current_row in 0..height {
            if let Some(line) = self.buffer.lines.get(current_row) {
                let truncated_line = if line.len() >= width {
                    &line[0..width]
                } else {
                    line
                };
                Self::render_line(current_row, truncated_line);
            } else if current_row == vertical_center && self.buffer.is_empty() {
                Self::render_line(current_row, &Self::build_welcome_message(width));
            } else {
                Self::render_line(current_row, "~");
            }
        }
        self.needs_redraw = false;
    }

    fn build_welcome_message(width: usize) -> String {
        if width == 0 {
            return " ".to_string();
        }
        let welcome_message = format!("{NAME} o(〃＾▽＾〃)o tekstieditowi -- vewsio {VERSION}");
        let len = welcome_message.len();
        if width <= len {
            return "~".to_string();
        }
        #[allow(clippy::integer_division)]
        let padding = (width.saturating_sub(len).saturating_sub(1)) / 2;

        let mut full_message = format!("~{}{}", " ".repeat(padding), welcome_message);
        full_message.truncate(width);
        full_message
    }

    pub fn load(&mut self, file_name: &str) -> Result<(), String> {
        if let Ok(buffer) = Buffer::load(file_name) {
            self.buffer = buffer;
            self.filename = Some(file_name.to_string());
            self.needs_redraw = true;
            Ok(())
        } else {
            Err(format!("Failed to load file: {}", file_name))
        }
    }
    
    pub fn save(&self) {
        if self.buffer.save().is_ok() {}
        else {print!("ei onnistu!");}
    }

    pub fn insert_char(&mut self, c: char, posx: usize, posy: usize ) {
        if self.line.is_empty() {
            self.line.push(c);
        }
        else {
            self.line.insert(posx - 1, c);
            self.needs_redraw = true;
        }
        self.buffer.refresh_buffer(self.line.clone().into_iter().collect(), posy);
        self.needs_redraw = true;
        
        if !self.undoing {
            self.undoredo.add_change(TextChange::new(ChangeType::Character, posx, posy, Some(c), None, None));
        }
        if self.redoing {
            self.undoredo.add_change(TextChange::new_redo(ChangeType::Character, posx, posy, Some(c), None, None))
        }

    }
    pub fn insert_row(&mut self, x: usize, y: usize) {

        let new_row = String::new();

        if self.line_len() >= x  && (self.line_len() != 0) {
            let back = self.line.split_off(x);
            let front = self.line.clone();
            self.line = front.to_vec();
            self.buffer.refresh_buffer(self.line.clone().into_iter().collect(), y);

            self.line = back.clone();
            self.buffer.insert(back.clone().into_iter().collect(), y + 1);
            self.buffer.refresh_buffer(self.line.clone().into_iter().collect(), y + 1);

            if !self.undoing {
                self.undoredo.add_change(TextChange::new(ChangeType::Enter, x, y, None, Some(front.clone()), Some(back.clone())));
            }
            if self.redoing {
                self.undoredo.add_change(TextChange::new_redo(ChangeType::Enter, x, y, None, Some(front), Some(back)))
            }
        }
        else if self.line_len() == 0 {
            self.buffer.refresh_buffer("".to_string(), y);
            
            self.buffer.insert(new_row, y + 1);
            self.line = Vec::new();
            self.buffer.refresh_buffer("".to_string(), y + 1);

            if !self.undoing {
                self.undoredo.add_change(TextChange::new(ChangeType::Enter, x, y, None, None, None));
            }
            if self.redoing {
                self.undoredo.add_change(TextChange::new_redo(ChangeType::Enter, x, y, None, None, None));
            }
        }
        self.needs_redraw = true;
    }

    pub fn remove_char(&mut self, posx: usize, posy: usize) {
        let c = self.line.remove(posx);
        self.needs_redraw = true;
        self.buffer.refresh_buffer(self.line.clone().into_iter().collect(), posy);

        if !self.undoing {
            self.undoredo.add_change(TextChange::new(ChangeType::Removal, posx, posy, Some(c), None, None));
        }
        if self.redoing {
            self.undoredo.add_change(TextChange::new_redo(ChangeType::Removal, posx, posy, Some(c), None, None));
        }
        
    }
    pub fn remove_line(&mut self, pos: usize) {
        self.line = self.buffer.get_line(pos - 1);
        self.buffer.lines.remove(pos);
        self.buffer.refresh_buffer(self.line.clone().into_iter().collect(), pos - 1);
        
        if !self.undoing {
            self.undoredo.add_change(TextChange::new(ChangeType::Removal, 0, pos, None, Some(self.line.clone()), None));
        }
        if self.redoing {
            self.undoredo.add_change(TextChange::new(ChangeType::Removal, 0, pos, None, Some(self.line.clone()), None));
        }
        self.needs_redraw = true;
    }
    pub fn join_lines(&mut self, y: usize) -> usize {
        let back = self.line.clone();
        let front = self.buffer.get_line(y - 1).clone();

        self.remove_line(y);
        let length = self.line_len();
        for i in 0..back.len() {
            self.insert_char(back[i], self.line_len() + 1, y - 1);
        }

        self.buffer.refresh_buffer(self.line.clone().into_iter().collect(), y - 1);
        self.needs_redraw = true;

        if !self.undoing {
            self.undoredo.add_change(TextChange::new(ChangeType::Removal, 0, y, None, Some(front.clone()), Some(back.clone())));
        }
        if self.redoing {
            self.undoredo.add_change(TextChange::new_redo(ChangeType::Removal, 0, y, None, Some(front), Some(back)))
        }
        length
    }
    pub fn del_line(&mut self, y: usize) {
        let back = self.get_line(y + 1);
        for i in 0..back.len() {
            self.insert_char(back[i], self.line_len() + 1, y);
        }
        self.buffer.lines.remove(y + 1);
        self.needs_redraw = true;
    }

    pub fn move_line(&mut self, new_index: usize) {
        self.line = self.buffer.get_line(new_index);
    }
    pub fn buffer_len(&mut self) -> usize {
        self.buffer.lines.len()
    }
    pub fn line_len(&self) -> usize {
        self.line.len()
    }
    pub fn get_line(&mut self, pos: usize) -> Vec<char> {
        self.buffer.get_line(pos)
    }

    pub fn undo(&mut self,) -> Option<(usize, usize)>{
        let TextChange {c, x, y, change_type, front, back, ..} = self.undoredo.undo();
        self.undoing = true;

        match change_type {
            ChangeType::Character => {
                self.remove_char(x - 1, y);
                self.undoing = false;
                Some((x - 1, y))
            }
            ChangeType::Removal => {
                if c.is_some() {
                    self.insert_char(c?, x + 1, y);
                    self.undoing = false;
                    Some((x + 1, y))
                } 
                else if front.is_some() && back.is_none() {
                    //tyhjän rivin poisto
                    let loc = self.get_line(y - 1).len();
                    self.insert_row(loc, y - 1);
                    self.undoing = false;
                    Some((x, y))
                }
                else if front.is_some() && back.is_some() {
                    //kirjoitetun rivin jakautuminen
                    let loc = self.get_line(y - 1).len();
                    self.insert_row(loc, y - 1);
                    self.undoing = false;
                    Some((x, y))
                }
                else {self.undoing = false;None}
                
                
            }
            ChangeType::Enter => {

                if front.is_some() && back.is_some() {
                    self.join_lines(y + 1);
                }
                else if front.is_none() && back.is_none() {
                    self.remove_line(y + 1);
                }
                self.undoing = false;
                Some((x, y))
            }
            ChangeType::Nothing => {
                self.undoing = false;
                None
            }
        }
        
    }

    pub fn redo (&mut self,) -> Option<(usize, usize)> {
        if !self.undoredo.redos.is_empty() {
            let TextChange {c, x, y, change_type, front, back, ..} = self.undoredo.redo();
            self.undoing = true;
            match change_type {
                ChangeType::Character => {
                    self.insert_char(c?, x, y);
                    self.undoing = false;
                    Some((x, y))
                }
                ChangeType::Removal => {
                    if c.is_some() {
                        self.remove_char( x, y);
                        Some((x, y))
                    } 
                    else if front.is_some() && back.is_none() {
                        self.remove_line(y);
                        Some((x, y - 1))
                    }
                    else if front.is_some() && back.is_some() {
                        self.join_lines(y + 1);
                        Some((x, y))
                    }
                    else {None}
                    
                }
                ChangeType::Enter => {
    
                    if front.is_some() && back.is_some() {
                        self.insert_row(x, y);
                        
                    }
                    else if front.is_none() && back.is_none() {
                        self.insert_row(x, y);
                    }
                    Some((x, y))
                }
                ChangeType::Nothing => {
                    None
                }
                
            }
        }
        else {
            None
        }

    }

    
}

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            filename : None,
            needs_redraw: true,
            size: Terminal::size().unwrap_or_default(),
            line: Vec::new(),
            undoredo: UndoRedo::default(),
            undoing: false,
            redoing: false,
        }
    }
}
