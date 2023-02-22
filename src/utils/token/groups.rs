use unicode_categories::UnicodeCategories;

pub trait Grouping {
    fn start(&self, c: char) -> bool {
        self.belongs(c)
    }
    fn belongs(&self, c: char) -> bool;
    fn end(&self, c: char) -> bool {
        self.belongs(c)
    }
}

pub struct Number;

impl Grouping for Number {
    fn start(&self, c: char) -> bool {
        c.is_numeric()
    }
    fn belongs(&self, c: char) -> bool {
        c.is_numeric() || c == '.'
    }
    fn end(&self, c: char) -> bool {
        c.is_numeric()
    }
}

pub struct AlphaNumeric;

impl Grouping for AlphaNumeric {
    fn belongs(&self, c: char) -> bool {
        c.is_alphanumeric() || c.is_punctuation_connector()
    }
}

pub struct Whitespace;

impl Grouping for Whitespace {
    fn belongs(&self, c: char) -> bool {
        c.is_whitespace()
    }
}
