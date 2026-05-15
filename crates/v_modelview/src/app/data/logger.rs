use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Logger {
    lines: VecDeque<String>,
}

impl Logger {
    pub fn log<S: ToString>(&mut self, text: S) {
        while self.lines.len() > 99 {
            self.lines.pop_front();
        }

        let text = text.to_string();
        println!("{text}");
        self.lines.push_back(text);
    }

    pub fn lines(&self) -> (&[String], &[String]) {
        self.lines.as_slices()
    }
}
