use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation;

pub trait AsGrapheme: UnicodeSegmentation {
    fn grapheme_len(&self) -> usize {
        self.graphemes(true).count()
    }

    fn grapheme_truncate(&self, new_len: usize) -> Cow<Self>
    where
        Self: ToOwned,
        <Self as ToOwned>::Owned: for<'a> FromIterator<&'a str>,
    {
        (self.grapheme_len() <= new_len)
            .then_some(Cow::Borrowed(self))
            .unwrap_or_else(|| Cow::Owned(self.graphemes(true).take(new_len).collect()))
    }
}

impl AsGrapheme for str {}

#[cfg(test)]
mod test {}
