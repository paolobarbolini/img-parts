use bytes::{Bytes, BytesMut};

// the MP Header comes right after "MPF\0"
const MP_HEADER_OFFSET: usize = 4;

// the MP Entry tag of the MP Index IFD
const MP_ENTRY: u16 = 0xB002;

/// Updates the MP Entries in the `contents` of an MPF segment after the first
/// image grew by `size_delta` bytes, `offset_delta` of which after the MPF
/// segment, moving the other images away from it.
///
/// Returns `None` if the contents can't be parsed or updated.
pub(super) fn update_entries(contents: &[u8], size_delta: i64, offset_delta: i64) -> Option<Bytes> {
    // offsets are relative to the MP Header
    let header = contents.get(MP_HEADER_OFFSET..)?;
    let big_endian = match header.get(..4)? {
        b"MM\0*" => true,
        b"II*\0" => false,
        _ => return None,
    };

    let ifd = usize::try_from(read_u32(header, 4, big_endian)?).ok()?;
    let tags = read_u16(header, ifd, big_endian)?;
    let (entries, len) = (0..usize::from(tags)).find_map(|i| {
        let tag = ifd + 2 + 12 * i;
        if read_u16(header, tag, big_endian)? != MP_ENTRY {
            return None;
        }

        let len = read_u32(header, tag + 4, big_endian)?;
        let entries = read_u32(header, tag + 8, big_endian)?;
        Some((usize::try_from(entries).ok()?, usize::try_from(len).ok()?))
    })?;
    if entries.checked_add(len)? > header.len() {
        return None;
    }

    let mut contents = BytesMut::from(contents);
    let mut write_u32 = |pos: usize, value: u32| {
        let bytes = if big_endian {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        let pos = MP_HEADER_OFFSET + pos;
        contents[pos..pos + 4].copy_from_slice(&bytes);
    };
    for i in 0..len / 16 {
        let entry = entries + 16 * i;
        let offset = read_u32(header, entry + 8, big_endian)?;
        if i == 0 && offset == 0 {
            // the size of the first image, which has no offset, is only
            // informational, so it's left as is if it was already wrong
            let size = i64::from(read_u32(header, entry + 4, big_endian)?) + size_delta;
            if let Ok(size) = u32::try_from(size) {
                write_u32(entry + 4, size);
            }
        } else if offset != 0 {
            let offset = u32::try_from(i64::from(offset) + offset_delta).ok()?;
            write_u32(entry + 8, offset);
        }
    }

    Some(contents.freeze())
}

fn read_u16(buf: &[u8], pos: usize, big_endian: bool) -> Option<u16> {
    let bytes = buf.get(pos..pos.checked_add(2)?)?.try_into().ok()?;
    Some(if big_endian {
        u16::from_be_bytes(bytes)
    } else {
        u16::from_le_bytes(bytes)
    })
}

fn read_u32(buf: &[u8], pos: usize, big_endian: bool) -> Option<u32> {
    let bytes = buf.get(pos..pos.checked_add(4)?)?.try_into().ok()?;
    Some(if big_endian {
        u32::from_be_bytes(bytes)
    } else {
        u32::from_le_bytes(bytes)
    })
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    // an MPF segment listing a first image of 1000 bytes and
    // a second one of 500 bytes, 2000 bytes after the MP Header
    fn mpf(big_endian: bool) -> Vec<u8> {
        let u16_bytes = |value: u16| {
            if big_endian {
                value.to_be_bytes()
            } else {
                value.to_le_bytes()
            }
        };
        let u32_bytes = |value: u32| {
            if big_endian {
                value.to_be_bytes()
            } else {
                value.to_le_bytes()
            }
        };

        let mut contents = b"MPF\0".to_vec();
        contents.extend_from_slice(if big_endian { b"MM\0*" } else { b"II*\0" });
        contents.extend_from_slice(&u32_bytes(8));
        // MP Index IFD with an MP Entry tag pointing to 2 entries
        contents.extend_from_slice(&u16_bytes(1));
        contents.extend_from_slice(&u16_bytes(MP_ENTRY));
        contents.extend_from_slice(&u16_bytes(7));
        contents.extend_from_slice(&u32_bytes(32));
        contents.extend_from_slice(&u32_bytes(26));
        contents.extend_from_slice(&u32_bytes(0));
        for (size, offset) in [(1000, 0), (500, 2000)] {
            contents.extend_from_slice(&u32_bytes(0));
            contents.extend_from_slice(&u32_bytes(size));
            contents.extend_from_slice(&u32_bytes(offset));
            contents.extend_from_slice(&[0; 4]);
        }
        contents
    }

    fn entries(contents: &[u8], big_endian: bool) -> Vec<(u32, u32)> {
        let header = &contents[MP_HEADER_OFFSET..];
        (0..2)
            .map(|i| {
                let entry = 26 + 16 * i;
                let size = read_u32(header, entry + 4, big_endian).unwrap();
                let offset = read_u32(header, entry + 8, big_endian).unwrap();
                (size, offset)
            })
            .collect()
    }

    #[test]
    fn update() {
        for big_endian in [false, true] {
            let contents = update_entries(&mpf(big_endian), 10, 4).unwrap();
            assert_eq!(entries(&contents, big_endian), [(1010, 0), (500, 2004)]);

            let contents = update_entries(&mpf(big_endian), -10, -4).unwrap();
            assert_eq!(entries(&contents, big_endian), [(990, 0), (500, 1996)]);

            // the size of the first image was wrong to begin with
            let contents = update_entries(&mpf(big_endian), -1001, -4).unwrap();
            assert_eq!(entries(&contents, big_endian), [(1000, 0), (500, 1996)]);
        }
    }

    #[test]
    fn malformed() {
        let contents = mpf(false);
        for len in 0..contents.len() {
            assert_eq!(update_entries(&contents[..len], 10, 4), None);
        }

        // the offset can't become negative
        assert_eq!(update_entries(&contents, 0, -2001), None);
    }
}
