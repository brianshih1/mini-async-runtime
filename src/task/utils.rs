use std::alloc::Layout;

/// Returns the layout for `a` followed by `b` and the offset of `b`.
///
/// This function was adapted from the currently unstable [`Layout::extend()`]
#[inline]
pub(crate) fn extend(a: Layout, b: Layout) -> (Layout, usize) {
    let new_align = a.align().max(b.align());
    let pad = padding_needed_for(a, b.align());

    let offset = a.size().checked_add(pad).unwrap();
    let new_size = offset.checked_add(b.size()).unwrap();

    let layout = Layout::from_size_align(new_size, new_align).unwrap();
    (layout, offset)
}

/// Returns the padding after `layout` that aligns the following address to
/// `align`.
///
/// This function was adapted from the currently unstable,
/// [`Layout::padding_needed_for()`]
#[inline]
pub(crate) fn padding_needed_for(layout: Layout, align: usize) -> usize {
    let len = layout.size();
    let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    len_rounded_up.wrapping_sub(len)
}
