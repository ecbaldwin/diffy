use crate::diff::{cleanup, myers};
use crate::range::{DiffRange, Range};
use crate::utils::Classifier;

use super::{Hunk, Line, Patch, NO_NEWLINE_AT_EOF};
use nu_ansi_term::{Color, Style};
use std::{
    fmt::{Display, Formatter, Result},
    io,
};

/// Struct used to adjust the formatting of a `Patch`
#[derive(Debug)]
pub struct PatchFormatter {
    with_color: bool,

    context: Style,
    delete: Style,
    insert: Style,
    hunk_header: Style,
    patch_header: Style,
    function_context: Style,
}

impl PatchFormatter {
    /// Construct a new formatter
    pub fn new() -> Self {
        Self {
            with_color: false,

            context: Style::new(),
            delete: Color::Red.normal(),
            insert: Color::Green.normal(),
            hunk_header: Color::Cyan.normal(),
            patch_header: Style::new().bold(),
            function_context: Style::new(),
        }
    }

    /// Enable formatting a patch with color
    pub fn with_color(mut self) -> Self {
        self.with_color = true;
        self
    }

    /// Returns a `Display` impl which can be used to print a Patch
    pub fn fmt_patch<'a>(&'a self, patch: &'a Patch<'a, str>) -> impl Display + 'a {
        PatchDisplay { f: self, patch }
    }

    pub fn write_patch_into<T: ToOwned + AsRef<[u8]> + ?Sized, W: io::Write>(
        &self,
        patch: &Patch<'_, T>,
        w: W,
    ) -> io::Result<()> {
        PatchDisplay { f: self, patch }.write_into(w)
    }

    fn fmt_hunk<'a>(&'a self, hunk: &'a Hunk<'a, str>) -> impl Display + 'a {
        HunkDisplay { f: self, hunk }
    }

    fn write_hunk_into<T: AsRef<[u8]> + ?Sized, W: io::Write>(
        &self,
        hunk: &Hunk<'_, T>,
        w: W,
    ) -> io::Result<()> {
        HunkDisplay { f: self, hunk }.write_into(w)
    }

    fn fmt_line<'a>(&'a self, line: &'a Line<'a, str>) -> impl Display + 'a {
        LineDisplay { f: self, line }
    }

    fn write_line_into<T: AsRef<[u8]> + ?Sized, W: io::Write>(
        &self,
        line: &Line<'_, T>,
        w: W,
    ) -> io::Result<()> {
        LineDisplay { f: self, line }.write_into(w)
    }
}

impl Default for PatchFormatter {
    fn default() -> Self {
        Self::new()
    }
}

struct PatchDisplay<'a, T: ToOwned + ?Sized> {
    f: &'a PatchFormatter,
    patch: &'a Patch<'a, T>,
}

impl<T: ToOwned + AsRef<[u8]> + ?Sized> PatchDisplay<'_, T> {
    fn write_into<W: io::Write>(&self, mut w: W) -> io::Result<()> {
        if self.patch.original.is_some() || self.patch.modified.is_some() {
            if self.f.with_color {
                write!(w, "{}", self.f.patch_header.prefix())?;
            }
            if let Some(original) = &self.patch.original {
                write!(w, "--- ")?;
                original.write_into(&mut w)?;
                writeln!(w)?;
            }
            if let Some(modified) = &self.patch.modified {
                write!(w, "+++ ")?;
                modified.write_into(&mut w)?;
                writeln!(w)?;
            }
            if self.f.with_color {
                write!(w, "{}", self.f.patch_header.suffix())?;
            }
        }

        for hunk in &self.patch.hunks {
            self.f.write_hunk_into(hunk, &mut w)?;
        }

        Ok(())
    }
}

impl Display for PatchDisplay<'_, str> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if self.patch.original.is_some() || self.patch.modified.is_some() {
            if self.f.with_color {
                write!(f, "{}", self.f.patch_header.prefix())?;
            }
            if let Some(original) = &self.patch.original {
                writeln!(f, "--- {}", original)?;
            }
            if let Some(modified) = &self.patch.modified {
                writeln!(f, "+++ {}", modified)?;
            }
            if self.f.with_color {
                write!(f, "{}", self.f.patch_header.suffix())?;
            }
        }

        for hunk in &self.patch.hunks {
            write!(f, "{}", self.f.fmt_hunk(hunk))?;
        }

        Ok(())
    }
}

struct HunkDisplay<'a, T: ?Sized> {
    f: &'a PatchFormatter,
    hunk: &'a Hunk<'a, T>,
}

impl<T: AsRef<[u8]> + ?Sized> HunkDisplay<'_, T> {
    fn write_into<W: io::Write>(&self, mut w: W) -> io::Result<()> {
        if self.f.with_color {
            write!(w, "{}", self.f.hunk_header.prefix())?;
        }
        write!(w, "@@ -{} +{} @@", self.hunk.old_range, self.hunk.new_range)?;
        if self.f.with_color {
            write!(w, "{}", self.f.hunk_header.suffix())?;
        }

        if let Some(ctx) = self.hunk.function_context {
            write!(w, " ")?;
            if self.f.with_color {
                write!(w, "{}", self.f.function_context.prefix())?;
            }
            write!(w, " ")?;
            w.write_all(ctx.as_ref())?;
            if self.f.with_color {
                write!(w, "{}", self.f.function_context.suffix())?;
            }
        }
        writeln!(w)?;

        for line in &self.hunk.lines {
            self.f.write_line_into(line, &mut w)?;
        }

        Ok(())
    }
}

impl Display for HunkDisplay<'_, str> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if self.f.with_color {
            write!(f, "{}", self.f.hunk_header.prefix())?;
        }
        write!(f, "@@ -{} +{} @@", self.hunk.old_range, self.hunk.new_range)?;
        if self.f.with_color {
            write!(f, "{}", self.f.hunk_header.suffix())?;
        }

        if let Some(ctx) = self.hunk.function_context {
            write!(f, " ")?;
            if self.f.with_color {
                write!(f, "{}", self.f.function_context.prefix())?;
            }
            write!(f, " {}", ctx)?;
            if self.f.with_color {
                write!(f, "{}", self.f.function_context.suffix())?;
            }
        }
        writeln!(f)?;

        // TODO(Carl) This isn't really optimal but I think making it more elegant would require
        // significantly restructuring the code.
        let mut is_context = true;
        let mut original = self.hunk.originals.iter();
        let mut modified = self.hunk.modifieds.iter();

        for line in &self.hunk.lines {
            if !self.f.with_color {
                write!(f, "{}", self.f.fmt_line(line))?;
                continue;
            }
            if let Line::Context(_) = line {
                is_context = true;
                write!(f, "{}", self.f.fmt_line(line))?;
                continue;
            }

            if is_context {
                is_context = false;

                let deleted = *original.next().expect("expected to find a deleted string");

                let inserted = *modified
                    .next()
                    .expect("expected to find an inserted string");

                let mut classifier = Classifier::default();
                let (deleted, deleted_ids) = classifier.classify_groups(deleted);
                let (inserted, inserted_ids) = classifier.classify_groups(inserted);
                let solution = {
                    let mut solution = myers::diff(&deleted_ids, &inserted_ids);
                    cleanup::compact(&mut solution);
                    solution
                };

                // render is a helper to paint a range with a custom painter so that we can call it
                // with a different painter for DiffRange::Equal than for DiffRange::Delete/Insert.
                let render = |f: &mut Formatter<'_>,
                              groups: &Vec<&str>,
                              idx: &mut usize,
                              start_line: &mut bool,
                              range: &Range<[u64]>,
                              line_start: &dyn Display,
                              paint: &dyn for<'a> Fn(&'a str) -> Box<dyn Display + 'a>|
                 -> Result {
                    for group in groups.iter().skip(*idx).take(range.len()) {
                        for g in group.split_inclusive('\n') {
                            if *start_line {
                                write!(f, "{}", line_start)?;
                            }
                            *start_line = g.ends_with('\n');

                            write!(f, "{}", paint(g))?;
                        }
                    }
                    *idx += range.len();
                    Ok(())
                };

                // Render deleted part of the diff hunk. Includes Equal and Delete DiffRanges
                let mut i = 0;
                let mut nl = true;
                let mut r = |a: &Range<[u64]>,
                             paint: &dyn for<'a> Fn(&'a str) -> Box<dyn Display + 'a>|
                 -> Result {
                    let first = &self.f.delete.paint("-");
                    render(f, &deleted, &mut i, &mut nl, a, first, paint)
                };
                for diff in &solution {
                    match diff {
                        DiffRange::Equal(a, _) => r(a, &|s: &str| Box::new(s))?,
                        DiffRange::Delete(a) => r(a, &|s: &str| Box::new(self.f.delete.paint(s)))?,
                        _ => {}
                    }
                }

                // Render inserted part of the diff hunk. Includes Equal and Insert DiffRanges
                let mut i = 0;
                let mut nl = true;
                let mut r = |b: &Range<[u64]>,
                             p: &dyn for<'a> Fn(&'a str) -> Box<dyn Display + 'a>|
                 -> Result {
                    let first = &self.f.insert.paint("+");
                    render(f, &inserted, &mut i, &mut nl, b, first, p)
                };
                for diff in &solution {
                    match diff {
                        DiffRange::Equal(_, b) => r(b, &|s: &str| Box::new(s))?,
                        DiffRange::Insert(b) => r(b, &|s: &str| Box::new(self.f.insert.paint(s)))?,
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }
}

struct LineDisplay<'a, T: ?Sized> {
    f: &'a PatchFormatter,
    line: &'a Line<'a, T>,
}

impl<T: AsRef<[u8]> + ?Sized> LineDisplay<'_, T> {
    fn write_into<W: io::Write>(&self, mut w: W) -> io::Result<()> {
        let (sign, line, style) = match self.line {
            Line::Context(line) => (' ', line.as_ref(), self.f.context),
            Line::Delete(line) => ('-', line.as_ref(), self.f.delete),
            Line::Insert(line) => ('+', line.as_ref(), self.f.insert),
        };

        if self.f.with_color {
            write!(w, "{}", style.prefix())?;
        }

        if sign == ' ' && line == b"\n" {
            w.write_all(line)?;
        } else {
            write!(w, "{}", sign)?;
            w.write_all(line)?;
        }

        if self.f.with_color {
            write!(w, "{}", style.suffix())?;
        }

        if !line.ends_with(b"\n") {
            writeln!(w)?;
            writeln!(w, "{}", NO_NEWLINE_AT_EOF)?;
        }

        Ok(())
    }
}

impl Display for LineDisplay<'_, str> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let (sign, line, style) = match self.line {
            Line::Context(line) => (' ', line, self.f.context),
            Line::Delete(line) => ('-', line, self.f.delete),
            Line::Insert(line) => ('+', line, self.f.insert),
        };

        if self.f.with_color {
            write!(f, "{}", style.prefix())?;
        }

        if sign == ' ' && *line == "\n" {
            write!(f, "{}", line)?;
        } else {
            write!(f, "{}{}", sign, line)?;
        }

        if self.f.with_color {
            write!(f, "{}", style.suffix())?;
        }

        if !line.ends_with('\n') {
            writeln!(f)?;
            writeln!(f, "{}", NO_NEWLINE_AT_EOF)?;
        }

        Ok(())
    }
}
