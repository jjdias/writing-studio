/// The kinds of picture a book's cover can be: the two every ebook reader
/// shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
}

impl ImageKind {
    /// The kind the bytes are, from how the file starts rather than from its
    /// name, which says nothing about what is in it.
    pub fn of(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(ImageKind::Png)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(ImageKind::Jpeg)
        } else {
            None
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpeg => "jpg",
        }
    }

    pub fn media_type(self) -> &'static str {
        match self {
            ImageKind::Png => "image/png",
            ImageKind::Jpeg => "image/jpeg",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_known_by_its_first_bytes() {
        assert_eq!(ImageKind::of(b"\x89PNG\r\n\x1a\nrest"), Some(ImageKind::Png));
        assert_eq!(ImageKind::of(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(ImageKind::Jpeg));
        assert_eq!(ImageKind::of(b"<svg>"), None);
        assert_eq!(ImageKind::of(b""), None);
    }
}
