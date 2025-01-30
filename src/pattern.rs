use crate::Pixmap;

#[derive(Debug)]
pub struct Pattern<'a> {
    pixmap: &'a Pixmap,
    filter_quality: FilterQuality
}

#[derive(Copy, Clone, Debug)]
pub enum FilterQuality {
    Nearest
}


