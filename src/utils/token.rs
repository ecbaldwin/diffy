mod groups;

pub struct TokenIter<'a, 'f, T: ?Sized>(&'a T, &'f dyn Fn(&'a T) -> Option<usize>);

impl<'a, 'f, T: ?Sized> TokenIter<'a, 'f, T> {
    pub fn new(text: &'a T, f: &'f dyn Fn(&'a T) -> Option<usize>) -> Self {
        Self(text, f)
    }
}

impl<'a, 'f, T: super::Text + ?Sized> Iterator for TokenIter<'a, 'f, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }

        let end = if let Some(idx) = self.1(self.0) {
            idx
        } else {
            self.0.len()
        };

        let (line, remaining) = self.0.split_at(end);
        self.0 = remaining;
        Some(line)
    }
}

/// Iterator over the lines of a string, including the `\n` character.
pub struct GroupIter<'a, 'f>(TokenIter<'a, 'f, str>);

impl<'a, 'f> GroupIter<'a, 'f> {
    pub fn new(text: &'a str) -> Self {
        Self(TokenIter::<'a, 'f, str>::new(
            text,
            &|s: &'a str| -> Option<usize> {
                if let Some(c) = s.chars().nth(0) {
                    // The order of possible groups to match in order of preference
                    let groups: &[Box<&dyn groups::Grouping>] = &[
                        Box::new(&groups::Number {}),
                        Box::new(&groups::AlphaNumeric {}),
                        Box::new(&groups::Whitespace {}),
                    ];

                    for grouper in groups.iter() {
                        if !grouper.start(c) {
                            continue;
                        }
                        let mut pos = match s.find(|c: char| !grouper.belongs(c)) {
                            None => s.len(),
                            Some(pos) => pos,
                        };
                        loop {
                            loop {
                                pos -= 1;
                                if s.is_char_boundary(pos) {
                                    break;
                                }
                            }
                            match s[pos..].chars().next() {
                                // Oops, should be looking for the byte before
                                Some(c) if grouper.end(c) => return Some(pos + c.len_utf8()),
                                Some(_) => continue,
                                _ => break,
                            }
                        }
                    }
                    // By default, characters don't group at all
                    return Some(c.len_utf8());
                }
                None
            },
        ))
    }
}

impl<'a, 'f> Iterator for GroupIter<'a, 'f> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Iterator over the lines of a string, including the `\n` character.
pub struct LineIter<'a, 'f, T: ?Sized>(TokenIter<'a, 'f, T>);

impl<'a, 'f, T: super::Text + ?Sized> LineIter<'a, 'f, T> {
    pub fn new(text: &'a T) -> Self {
        Self(TokenIter::<'a, 'f, T>::new(
            text,
            &|s: &'a T| -> Option<usize> {
                match s.find("\n") {
                    Some(ndx) => Some(ndx + 1),
                    None => None,
                }
            },
        ))
    }
}

impl<'a, 'f, T: super::Text + ?Sized> Iterator for LineIter<'a, 'f, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lines() {
        let lines = indoc::indoc! {"
            line one
            line two\r
            line three
        "};

        let answer: Vec<_> =
            TokenIter::new(lines, &|s: &str| -> Option<usize> { s.find("\n") }).collect();
        assert_eq!(vec!["line one\n", "line two\r\n", "line three\n"], answer);
    }

    #[test]
    fn test_words() {
        assert_eq!(
            vec![" ", "one", " ", "two", "     ", "three", "\n"],
            GroupIter::new(" one two     three\n").collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_number() {
        assert_eq!(
            vec!["$", "1000000.00", "."],
            GroupIter::new("$1000000.00.").collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_alnum() {
        assert_eq!(
            vec!["_alpha_numeric"],
            GroupIter::new("_alpha_numeric").collect::<Vec<_>>()
        );
    }
}
