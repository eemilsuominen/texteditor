use core::cmp::min;
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::{
    env,
    fs::File,
    path::Path,
    io::{Error, Write},
    panic::{set_hook, take_hook}
};

mod terminal;
mod view;
use terminal::{Position, Size, Terminal};
use view::View;

#[derive(Copy, Clone, Default)]
struct Location {
    x: usize,
    y: usize,
}

pub struct Editor {
    should_quit: bool,
    location: Location,
    view: View,
}

impl Editor {
    pub fn new() -> Result<Self, Error> {
        let current_hook = take_hook();
        set_hook(Box::new(move |panic_info| {
            let _ = Terminal::terminate();
            current_hook(panic_info);
        }));
        Terminal::initialize()?;
        let mut view = View::default();
        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            let _ = view.load(file_name);
        }
        Ok(Self {
            should_quit: false,
            location: Location::default(),
            view,
        })
    }

    pub fn run(&mut self) {
        loop {
            self.refresh_screen();
            if self.should_quit {
                break;
            }
            match read() {
                Ok(event) => self.evaluate_event(event),
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not read event: {err:?}");
                    }
                }
            }
        }
    }

    fn move_point(&mut self, key_code: KeyCode) {
        let Location { mut x, mut y } = self.location;
        let Size { height, width } = Terminal::size().unwrap_or_default();
        match key_code {
            KeyCode::Up => {
                y = y.saturating_sub(1);
            }
            KeyCode::Down => {
                y = min(height.saturating_sub(1), y.saturating_add(1));
            }
            KeyCode::Left => {
                x = x.saturating_sub(1);
            }
            KeyCode::Right => {
                if x < self.view.line_len() {
                    x = min(width.saturating_sub(1), x.saturating_add(1));
                }
            }
            KeyCode::PageUp => {
                y = 0;
                self.view.move_line(0);
                if x > self.view.line_len() {
                    x = self.view.line_len();
                }
            }
            KeyCode::PageDown => {
                y = self.view.buffer_len() - 1;
                self.view.move_line(y);
                if x > self.view.line_len() {
                    x = self.view.line_len();
                }

            }
            KeyCode::Home => {
                x = 0;
            }
            KeyCode::End => {
                x = self.view.line_len();
            }
            _ => (),
        }
        self.location = Location { x, y };
    }

    #[allow(clippy::needless_pass_by_value)]
    fn evaluate_event(&mut self, event: Event) {
        match event {
            Event::Key(KeyEvent {
                code,
                modifiers,
                ..
            }) => match (code, modifiers) {
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                    self.view.save();
                }
                (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                    self.open_file_prompt();
                }
                (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                    if let Some((x, y)) = self.view.undo() {
                        self.location = Location{x, y};                   
                    }
                }
                (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                    if let Some((x, y)) = self.view.redo() {
                        self.location = Location{x, y};
                    }
                }
                (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                    self.view.insert_char(c, self.location.x + 1, self.location.y);
                    let mut x = self.location.x;
                    let  width = Terminal::size().unwrap_or_default().width;
                    x = min(width.saturating_sub(1), x.saturating_add(1));
                    self.location.x = x;

                }
                (KeyCode::Char(c), KeyModifiers::NONE) => {
                    self.view.insert_char(c, self.location.x + 1, self.location.y);
                    let mut x = self.location.x;
                    let  width = Terminal::size().unwrap_or_default().width;
                    x = min(width.saturating_sub(1), x.saturating_add(1));
                    self.location.x = x;
                }
                (KeyCode::Enter, KeyModifiers::NONE) => {
                    self.view.insert_row(self.location.x, self.location.y);
                    self.move_point(KeyCode::Down);
                    self.move_point(KeyCode::Home);                    
                }
                (KeyCode::Backspace, KeyModifiers::NONE) => {
                    if self.location.x == 0 && !self.view.get_line(self.location.y).is_empty() && self.location.y != 0 {
                        let y = self.location.y;
                        let length = self.view.join_lines(y);
                        
                        while self.location.x < length {
                            self.move_point(KeyCode::Right);
                        }
                        self.move_point(KeyCode::Up);

                    }
                    else if !self.view.get_line(self.location.y).is_empty() && self.location.x != 0 {
                        self.move_point(KeyCode::Left);
                        self.view.remove_char(self.location.x, self.location.y);
                    }
                    else if self.location.y != 0 {
                        self.view.remove_line(self.location.y);
                        self.move_point(KeyCode::End);
                        self.move_point(KeyCode::Up);
                    }

                }
                (KeyCode::Delete, KeyModifiers::NONE) => {
                    if self.location.x == self.view.line_len() && self.view.buffer_len() > self.location.y + 1 {
                        self.view.del_line(self.location.y);
                    }
                    else if self.location.x < self.view.line_len() {
                        self.view.remove_char(self.location.x, self.location.y);
                    }
                }
                (
                    KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::PageDown
                    | KeyCode::PageUp
                    | KeyCode::End
                    | KeyCode::Home,
                    _,
                ) => {
                    self.move_point(code);
                }
                (KeyCode::Up | KeyCode::Down, _,) => {
                    if code == KeyCode::Down {
                        if self.location.y + 1 < self.view.buffer_len() {
                            self.move_point(code);
                            self.view.move_line(self.location.y);
                            let line_length = self.view.line_len();
                            while self.location.x > line_length {
                                self.move_point(KeyCode::Left);
                            }
                        }
                    } 
                    else {
                        self.move_point(code);
                        self.view.move_line(self.location.y);
                        let line_length = self.view.line_len();
                        while self.location.x > line_length {
                            self.move_point(KeyCode::Left);
                        }
                    }
                }
                _ => {}
            },
            Event::Resize(width_u16, height_u16) => {
                #[allow(clippy::as_conversions)]
                let height = height_u16 as usize;
                #[allow(clippy::as_conversions)]
                let width = width_u16 as usize;
                self.view.resize(Size { height, width });
            }
            _ => {}
        }
    }

    fn open_file_prompt(&mut self) {
        let filename = self.prompt_for_filename();  // Implement this method to capture input
        if !filename.is_empty() {
            if Path::new(&filename).exists() {
                match self.view.load(&filename) {
                    Ok(()) => {
                        self.location = Location::default();  // Reset cursor position
                        println!("File loaded successfully.");
                    },
                    Err(err) => {
                        println!("{}", err);
                    }
                }
            } else {
                let _ = self.prompt_create_file(&filename);
                let _ = self.view.load(&filename);
            }
        }
    }    

    fn prompt_create_file(&self, file_name: &str) -> Result<bool, String> {
        println!("Hupsis kopsis! Tiedostoa '{}' ei ole olemassa (๑•́_•̀๑). Luodaanko tiedosto?(y/n)", file_name);
        let mut answer = String::new();

        loop {
            match read().unwrap() {
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Char(c) => {
                        answer.push(c);
                        let _ = Terminal::print(&c.to_string()); // Show typed character
                        let _ = Terminal::execute(); // Update the terminal view
                    },
                    KeyCode::Backspace => {
                        answer.pop();
                        let _ = Terminal::print("\x08 \x08"); // Move cursor back and erase
                        let _ = Terminal::execute(); // Update the terminal view
                    },
                    KeyCode::Enter => {
                        break;
                    },
                    _ => {} // Ignore other keys
                },
                _ => {} // Ignore other events
            }
        }
        let answer = answer.trim().eq_ignore_ascii_case("y");

        if answer {
            // Attempt to create the file
            match File::create(file_name) {
                Ok(mut file) => {
                    if let Err(e) = writeln!(file, "") {  // Optionally write an empty line or file header
                        return Err(format!("Tiedoston luonti epäonnistui: {}", e));
                    }
                    println!("Tiedosto luotiin onnistuneesti.");
                    Ok(true)
                },
                Err(e) => Err(format!("Tiedoston luonti epäonnistui: {}", e))
            }
        } else {
            println!("Tiedoston luonti canceloitiin.");
            Ok(false)
        }
    }

    fn prompt_for_filename(&mut self) -> String {
        let mut filename = String::new();
        let _ = Terminal::print("Anna tiedostonimi (つˆ⌣ˆ)つ: ");
        let _ = Terminal::execute(); // Make sure the prompt is visible

        loop {
            match read().unwrap() {
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Char(c) => {
                        filename.push(c);
                        let _ = Terminal::print(&c.to_string()); // Show typed character
                        let _ = Terminal::execute(); // Update the terminal view
                    },
                    KeyCode::Backspace => {
                        filename.pop();
                        let _ = Terminal::print("\x08 \x08"); // Move cursor back and erase
                        let _ = Terminal::execute(); // Update the terminal view
                    },
                    KeyCode::Enter => {
                        break;
                    },
                    _ => {} // Ignore other keys
                },
                _ => {} // Ignore other events
            }
        }

        let _ = Terminal::print("\r\n"); // Move to the next line after input is complete
        let _ = Terminal::execute(); // Finalize the input line
        filename
    }

    fn refresh_screen(&mut self) {
        let _ = Terminal::hide_caret();
        self.view.render();
        let _ = Terminal::move_caret_to(Position {
            col: self.location.x,
            row: self.location.y,
        });
        let _ = Terminal::show_caret();
        let _ = Terminal::execute();
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = Terminal::terminate();
        if self.should_quit {
            let _ = Terminal::print("Heippa ヾ(｡>﹏<｡)ﾉ\r\n");
        }
    }
}