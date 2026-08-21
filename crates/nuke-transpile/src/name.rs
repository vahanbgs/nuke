use std::borrow::Cow;

pub(crate) fn transliterate(field: &str) -> Cow<'_, str> {
    if field.contains('_') {
        Cow::Owned(field.replace('_', "-"))
    } else {
        Cow::Borrowed(field)
    }
}
