use core::mem;

use bytes::{Buf, Bytes};

use crate::{Error, Result};

pub fn u24_from_le_bytes(b: [u8; 3]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], 0])
}

pub fn u24_to_le_bytes(n: u32) -> [u8; 3] {
    let b = n.to_le_bytes();
    b[0..3].try_into().unwrap()
}

/// Check that the type returned by the function `f` isn't bigger than
/// the remaining length of `Bytes`. This avoids panicking if the input
/// is corrupted or truncated.
///
/// Calls `f` passing `buf` as an argument, returning the resulting type
///
/// # Errors
///
/// Returns [`Error::Truncated`][crate::Error::Truncated] if `buf` isn't
/// big enought.
pub fn read_checked<F, T>(buf: &mut Bytes, f: F) -> Result<T>
where
    F: FnOnce(&mut Bytes) -> T,
{
    if mem::size_of::<T>() > buf.len() {
        return Err(Error::Truncated);
    }

    Ok(f(buf))
}

/// Convenience function to read a [u8; 4] using [`read_checked`][self::read_checked]
pub fn read_u8_len4_array(buf: &mut Bytes) -> Result<[u8; 4]> {
    read_checked(buf, |buf| {
        let mut array = [0; 4];
        buf.copy_to_slice(&mut array);
        array
    })
}

/// Convenience function to read a [u8; 8] using [`read_checked`][self::read_checked]
pub fn read_u8_len8_array(buf: &mut Bytes) -> Result<[u8; 8]> {
    read_checked(buf, |buf| {
        let mut array = [0; 8];
        buf.copy_to_slice(&mut array);
        array
    })
}

/// Check that `buf` is long enought to be split at `at`.
/// This avoids panicking if the input is corrupted or truncated.
///
/// Calls [`Bytes::split_to`][Bytes::split_to] and returns the result.
///
/// # Errors
///
/// Returns [`Error::Truncated`][crate::Error::Truncated] if `buf` isn't
/// big enought.
pub fn split_to_checked(buf: &mut Bytes, at: usize) -> Result<Bytes> {
    if at > buf.len() {
        return Err(Error::Truncated);
    }

    Ok(buf.split_to(at))
}

#[cfg(test)]
mod tests {
    use bytes::Buf;

    use super::*;

    #[test]
    fn read_u8_checked() {
        let mut buf = Bytes::from_static(&[0x00, 0x01, 0x02]);

        assert_eq!(read_checked(&mut buf, |buf| buf.get_u8()), Ok(0x00));
        assert_eq!(read_checked(&mut buf, |buf| buf.get_u8()), Ok(0x01));
        assert_eq!(read_checked(&mut buf, |buf| buf.get_u8()), Ok(0x02));

        assert_eq!(
            read_checked(&mut buf, |buf| buf.get_u8()),
            Err(Error::Truncated)
        );
    }

    #[test]
    fn split_checked() {
        let mut buf = Bytes::from_static(&[0x00, 0x01, 0x02, 0x03]);

        assert_eq!(
            split_to_checked(&mut buf, 2).as_ref(),
            Ok(&Bytes::from_static(&[0x00, 0x01]))
        );
        assert_eq!(
            split_to_checked(&mut buf, 2).as_ref(),
            Ok(&Bytes::from_static(&[0x02, 0x03]))
        );

        assert_eq!(split_to_checked(&mut buf, 2), Err(Error::Truncated));
    }
}
