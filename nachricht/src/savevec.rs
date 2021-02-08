use std::alloc;
use crate::error::DecodeError;

// TODO: remove this once try_reserve is stable: https://github.com/rust-lang/rust/issues/48043
// Safety: Calling this with a zero-sized type is undefined behaviour.
pub fn vec_with_capacity<T>(capacity: usize) -> Result<Vec<T>, DecodeError> {
    let layout = alloc::Layout::array::<T>(capacity).map_err(|_| DecodeError::Allocation(capacity, std::mem::size_of::<T>()))?;
    match unsafe { alloc::alloc(layout) } { // this is safe because we have a proper layout
        zero if zero as usize == 0 => Err(DecodeError::Allocation(capacity, std::mem::size_of::<T>())),
        ptr => Ok(unsafe { Vec::from_raw_parts(ptr as *mut T, 0, capacity) }) // this is safe because ptr is correctly sized and aligned now
    }
}
