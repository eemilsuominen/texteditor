use std::io::{Error, Write};
use std::fs::{read_to_string, File};

#[derive(Default)]
pub struct Buffer {
    pub lines: Vec<String>,
    pub file_name: Option<String>,
}

impl Buffer {
    pub fn load(file_name: &str) -> Result<Self, Error> {
        let contents = read_to_string(file_name)?;
        let mut lines = Vec::new();
        for value in contents.lines() {
            lines.push(String::from(value));
        }
        Ok(Self { lines, file_name: Some(file_name.to_string()) })
    }
    
    pub fn save(&self) -> Result<(), Error> {
        if let Some(file_name) = &self.file_name {
            let mut file = File::create(file_name)?;
            for i in &self.lines {
                file.write(i.as_bytes())?;
                file.write_all(b"\n")?;
            }
        }
        
        Ok(())
    }
    pub fn is_empty(&self) -> bool{
        self.lines.is_empty()
    }
    pub fn refresh_buffer(&mut self, line: String, pos: usize) {
        if !self.is_empty() {
            self.lines.remove(pos);
            self.lines.insert(pos, line);
        }
        else {
            self.lines.push(line);
        }
    }
    pub fn get_line(&mut self, pos: usize) -> Vec<char>{
        let ret: Vec<char>;
        if let Some(line) = self.lines.get(pos) {
            ret = line.chars().collect();
        }
        else {
            ret = Vec::new();
        }
        ret
        
    }

    pub fn insert(&mut self, string: String, y: usize) {
        if self.lines.len() > y {
            self.lines.insert(y, string);
        }
        else {
            self.lines.push(string);
        }

    }

}