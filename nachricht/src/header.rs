//! A `Nachricht` header is defined by a code and a value. The first three bits of the first byte
//! define a code, while the latter five bits yield an unsigned integer, consistently named `sz`
//! for size. If this integer is less than 24, its value is the value of the header. Otherwise the
//! following `sz - 23` bytes (up to eight, since `sz::MAX` is `2^5-1`) contain an unsigned integer
//! in network byte order which defines the value of the header. The interpretation of the value
//! depends on the code: it can either define the value of the whole field or the length of the
//! field's content.

use crate::error::{DecodeError, EncodeError};
use std::convert::TryFrom;
use std::io::Write;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Header {
    /// Also known as unit or nil
    Null,
    /// The boolean value true
    True,
    /// The boolean value false
    False,
    /// The following four bytes contain an IEEE-754 32-bit floating point number
    F32,
    /// The following eight bytes contain an IEEE-754 64-bit floating point number
    F64,
    /// The value describes the length of a following byte array.
    /// Note that this code also contains the five fixed length values.
    Bin(usize),
    /// The value describes a postive integer fitting into an u64
    Pos(u64),
    /// The value describes a negative integer whose negative fits into an u64
    Neg(u64),
    /// The value describes the length in fields of a container. the fields follow the header
    /// immediately.
    Bag(usize),
    /// The value describes the length in bytes of a following unicode string
    Str(usize),
    /// A symbol has the same semantics as a string except that it gets pushed into the symbol
    /// table and can be referenced from there
    Sym(usize),
    /// The value describes the length in bytes of a following key value. This in itself does not
    /// define a field and thus has to immediately followed by another header whose field will be
    /// named by this key.
    Key(usize),
    /// A reference into the symbol table. This could resolve to either a symbol (which itself
    /// resolve to a string) or a key.
    Ref(usize),
}

impl Header {

    /// Returns the number of written bytes
    pub fn encode<W: Write>(&self, w: &mut W) -> Result<usize, EncodeError> {
        match *self {
            Header::Null                    => { w.write_all(&[self.code() << 5 | 0])?; Ok(1) },
            Header::True                    => { w.write_all(&[self.code() << 5 | 1])?; Ok(1) },
            Header::False                   => { w.write_all(&[self.code() << 5 | 2])?; Ok(1) },
            Header::F32                     => { w.write_all(&[self.code() << 5 | 3])?; Ok(1) },
            Header::F64                     => { w.write_all(&[self.code() << 5 | 4])?; Ok(1) },
            Header::Pos(i) | Header::Neg(i) => self.encode_u64(i, w),
            Header::Bin(i)
                | Header::Bag(i)
                | Header::Str(i)
                | Header::Sym(i)
                | Header::Key(i)
                | Header::Ref(i)            => self.encode_u64(Self::to_u64(i)?, w)
        }
    }

    /// Returns the decoded header and the number of consumed bytes
    pub fn decode<B: ?Sized + AsRef<[u8]>>(buf: &B) -> Result<(Self, usize), DecodeError> {
        let buf = buf.as_ref();
        if buf.len() < 1 {
            return Err(DecodeError::Eof);
        }
        let code = buf[0] >> 5;
        let sz = buf[0] & 0x1f;
        match code {
            0 => {
                match sz {
                    0 => Ok((Header::Null,  1)),
                    1 => Ok((Header::True,  1)),
                    2 => Ok((Header::False, 1)),
                    3 => Ok((Header::F32,   1)),
                    4 => Ok((Header::F64,   1)),
                    x if x < 24 => Ok((Header::Bin(x as usize - 5), 1)),
                    x => Self::decode_u64(&buf[1..], x).and_then(|(i, c)| Ok((Header::Bin(Self::to_usize(i)?), c + 1))),
                }
            },
            1 => Self::decode_u64(&buf[1..], sz).map(|(i, c)| (Header::Pos(i), c + 1)),
            2 => Self::decode_u64(&buf[1..], sz).map(|(i, c)| (Header::Neg(i), c + 1)),
            3 => Self::decode_u64(&buf[1..], sz).and_then(|(i, c)| Ok((Header::Bag(Self::to_usize(i)?), c + 1))),
            4 => Self::decode_u64(&buf[1..], sz).and_then(|(i, c)| Ok((Header::Str(Self::to_usize(i)?), c + 1))),
            5 => Self::decode_u64(&buf[1..], sz).and_then(|(i, c)| Ok((Header::Sym(Self::to_usize(i)?), c + 1))),
            6 => Self::decode_u64(&buf[1..], sz).and_then(|(i, c)| Ok((Header::Key(Self::to_usize(i)?), c + 1))),
            7 => Self::decode_u64(&buf[1..], sz).and_then(|(i, c)| Ok((Header::Ref(Self::to_usize(i)?), c + 1))),
            _ => unreachable!(),
        }
    }

    #[inline]
    fn encode_u64<W: Write>(&self, i: u64, w: &mut W) -> Result<usize, EncodeError> {
        let limit = self.sz_limit();
        if i < limit as u64 {
            w.write_all(&[self.code() << 5 | i as u8 + (24 - limit)])?;
            Ok(1)
        } else {
            let sz = Self::size(i);
            let buf = i.to_be_bytes();
            w.write_all(&[self.code() << 5 | (sz + 23)])?;
            w.write_all(&buf[buf.len() - sz as usize ..])?;
            Ok(1 + sz as usize)
        }
    }

    #[inline]
    fn decode_u64(buf: &[u8], sz: u8) -> Result<(u64, usize), DecodeError> {
        if sz < 24 {
            Ok((sz as u64, 0))
        } else {
            let sz = sz as usize - 23;
            if buf.len() < sz {
                Err(DecodeError::Eof)
            } else {
                let mut tmp = [0u8; 8];
                tmp[8 - sz..].copy_from_slice(&buf[..sz]);
                Ok((<u64>::from_be_bytes(tmp), sz))
            }
        }
    }

    #[inline]
    fn code(&self) -> u8 {
        match *self {
            Header::Null | Header::True | Header::False | Header::F32 | Header::F64 | Header::Bin(_) => 0,
            Header::Pos(_)                                                                           => 1,
            Header::Neg(_)                                                                           => 2,
            Header::Bag(_)                                                                           => 3,
            Header::Str(_)                                                                           => 4,
            Header::Sym(_)                                                                           => 5,
            Header::Key(_)                                                                           => 6,
            Header::Ref(_)                                                                           => 7,
        }
    }

    #[inline]
    fn sz_limit(&self) -> u8 {
        match *self {
            Header::Bin(_) => 19,
            _              => 24,
        }
    }

    /// Returns the number of bytes needed to encode this value
    #[inline]
    fn size(value: u64) -> u8 {
        if value < 1 << 8 {
            1
        } else if value < 1 << 16 {
            2
        } else if value < 1 << 24 {
            3
        } else if value < 1 << 32 {
            4
        } else if value < 1 << 40 {
            5
        } else if value < 1 << 48 {
            6
        } else if value < 1 << 56 {
            7
        } else {
            8
        }
    }

    #[inline]
    fn to_usize(value: u64) -> Result<usize, DecodeError> {
        usize::try_from(value).map_err(|_| DecodeError::Length(value))
    }

    #[inline]
    fn to_u64(value: usize) -> Result<u64, EncodeError> {
        u64::try_from(value).map_err(|_| EncodeError::Length(value))
    }

}

#[cfg(test)]
mod tests {
    use super::Header;

    #[test]
    fn lead_bytes() {
        let mut src = [0u8; 9];
        let mut dst = Vec::with_capacity(9);
        for l in 0..u8::MAX {
            dst.clear();
            src[0] = l;
            let decoded = Header::decode(&src).unwrap().0;
            let _ = decoded.encode(&mut dst).unwrap();
        }
    }

    #[test]
    fn roundtrip_compact() {
        let mut buf = Vec::new();
        assert_roundtrip(Header::Null, &mut buf);
        assert_roundtrip(Header::True, &mut buf);
        assert_roundtrip(Header::False, &mut buf);
        assert_roundtrip(Header::F32, &mut buf);
        assert_roundtrip(Header::F64, &mut buf);
        for i in 0..24 {
            if i < 19 {
                assert_roundtrip(Header::Bin(i), &mut buf);
            }
            assert_roundtrip(Header::Pos(i as u64), &mut buf);
            assert_roundtrip(Header::Neg(i as u64), &mut buf);
            assert_roundtrip(Header::Bag(i), &mut buf);
            assert_roundtrip(Header::Str(i), &mut buf);
            assert_roundtrip(Header::Sym(i), &mut buf);
            assert_roundtrip(Header::Key(i), &mut buf);
            assert_roundtrip(Header::Ref(i), &mut buf);
        }
    }

    #[test]
    fn roundtrip_long() {
        let mut buf = Vec::new();
        // choose large prime number to make this test terminate in acceptable time, in this case
        // (2^59-1)/179951
        for i in (0..u64::MAX).step_by(3_203_431_780_337) {
            assert_roundtrip(Header::Bin(i as usize), &mut buf);
            assert_roundtrip(Header::Pos(i), &mut buf);
            assert_roundtrip(Header::Neg(i), &mut buf);
            assert_roundtrip(Header::Bag(i as usize), &mut buf);
            assert_roundtrip(Header::Str(i as usize), &mut buf);
            assert_roundtrip(Header::Sym(i as usize), &mut buf);
            assert_roundtrip(Header::Key(i as usize), &mut buf);
            assert_roundtrip(Header::Ref(i as usize), &mut buf);
        }
    }

    fn assert_roundtrip(value: Header, buf: &mut Vec<u8>) {
        let _ = value.encode(buf);
        assert_eq!(value, Header::decode(buf).unwrap().0);
        buf.clear();
    }

}
