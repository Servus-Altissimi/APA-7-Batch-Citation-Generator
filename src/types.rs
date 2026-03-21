#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OutputFormat {
    Latex,
    BibTeX,
    PlainText,
    Markdown,
    Ris,
    RichText,
}

impl OutputFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Latex     => "LaTeX (.tex) - Overleaf / TeXStudio",
            Self::BibTeX    => "BibTeX (.bib) - Overleaf / TeXStudio",
            Self::PlainText => "Plain Text (.txt) - Notepad / Notepad++",
            Self::Markdown  => "Markdown (.md) - README / Obsidian",
            Self::Ris       => "RIS (.ris) - Zotero / EndNote",
            Self::RichText  => "Rich Text (.rtf) - Word / Google Docs",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Latex     => "references.tex",
            Self::BibTeX    => "references.bib",
            Self::PlainText => "references.txt",
            Self::Markdown  => "references.md",
            Self::Ris       => "references.ris",
            Self::RichText  => "references.html",
        }
    }

    pub fn all() -> &'static [OutputFormat] {
        &[Self::Latex, Self::BibTeX, Self::PlainText, Self::Markdown, Self::Ris, Self::RichText]
    }

    pub fn from_index(i: usize) -> Self {
        Self::all()[i.min(Self::all().len() - 1)]
    }

    pub fn to_index(self) -> usize {
        Self::all().iter().position(|&f| f == self).unwrap_or(0)
    }
}
