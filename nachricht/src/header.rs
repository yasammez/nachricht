//! A `Nachricht` header is defined by a code and a value. The first three bits of the first byte
//! define a code, while the latter five bits yield an unsigned integer, consistently named `sz`
//! for size. If this integer is less than 24, its value is the value of the header. Otherwise the
//! following `sz - 23` bytes (up to eight, since `sz::MAX` is `2^5-1`) contain an unsigned integer
//! in network byte order which defines the value of the header. The interpretation of the value
//! depends on the code: it can either define the value of the whole field or the length of the
//! field's content.

use crate::error::*;
use std::io::Write;

#[derive(Debug, PartialEq)]
pub struct Header(pub Code, pub u64);

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Code {
    /// The value describes a postive integer fitting into an u64
    Intp      = 0,
    /// The value describes a negative integer whose negative fits into an u64
    Intn      = 1,
    /// The value describes the length of a following byte array
    Bytes     = 2,
    /// The value describes the length in bytes of a following unicode string
    Str       = 3,
    /// The value describes the length in bytes of a following key value. This in itself does not
    /// define a field and thus has to immediately followed by another header whose field will be
    /// named by this key.
    Key       = 4,
    /// The value describes the length in fields of a container. the fields follow the header
    /// immediately.
    Container = 5,
    /// Collection code for types whose lengths are either fixed or whose values fit into five
    /// bits.
    Fixed     = 6,
    Reserved  = 7,
}

impl Code {

    /// `data` must be smaller than 8
    pub fn from_bits(data: u8) -> Self {
        match data {
            x if x == Code::Intp as u8      => Code::Intp,
            x if x == Code::Intn as u8      => Code::Intn,
            x if x == Code::Bytes as u8     => Code::Bytes,
            x if x == Code::Str as u8       => Code::Str,
            x if x == Code::Key as u8       => Code::Key,
            x if x == Code::Container as u8 => Code::Container,
            x if x == Code::Fixed as u8     => Code::Fixed,
            x if x == Code::Reserved as u8  => Code::Reserved,
            _ => unreachable!(),
        }
    }

    pub fn to_bits(&self) -> u8 {
        *self as u8
    }

}

impl Header {

    pub fn encode<W: Write>(&self, w: &mut W) -> Result<usize, EncodeError> {
        let code = self.0;
        let value = self.1;
        if value < 24 {
            w.write_all(&[code.to_bits() << 5 | value as u8])?;
            Ok(1)
        } else {
            let sz = Self::size(value);
            let buf = value.to_be_bytes();
            w.write_all(&[code.to_bits() << 5 | (sz + 23)])?;
            w.write_all(&buf[buf.len() - sz as usize ..])?;
            Ok(1 + sz as usize)
        }
    }

    pub fn decode<B: ?Sized + AsRef<[u8]>>(buf: &B) -> Result<(Self, &[u8]), DecodeError> {
        let buf = buf.as_ref();
        if buf.len() < 1 {
            return Err(DecodeError::Eof);
        }
        let lead_byte = buf[0];
        let code = Code::from_bits(lead_byte >> 5);
        let sz = lead_byte & 0x1f;
        if sz < 24 {
            Ok((Header(code, sz as u64), &buf[1..]))
        } else {
            let (value, tail) = Self::decode_value(&buf[1..], sz - 23)?;
            Ok((Header(code, value), tail))
        }
    }

    #[inline]
    fn decode_value<B: ?Sized + AsRef<[u8]>>(buf: &B, sz: u8) -> Result<(u64, &[u8]), DecodeError> {
        let buf = buf.as_ref();
        let sz = sz as usize;
        let mut tmp = [0u8; 8];
        if buf.len() < sz {
            Err(DecodeError::Eof)
        } else {
            tmp[8 - sz..].copy_from_slice(&buf[..sz]);
            let tmp = <u64>::from_be_bytes(tmp);
            Ok((tmp, &buf[sz..]))
        }
    }

    /// Returns the number of bytes needed to encode this value
    #[inline]
    pub fn size(value: u64) -> u8 {
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

}

#[cfg(test)]
mod tests {
    use super::{Header, Code};

    #[test]
    fn roundtrip_compact() {
        let mut buf = Vec::new();
        for i in 0..24 {
            assert_roundtrip(Header(Code::Intp, i), &mut buf);
            assert_roundtrip(Header(Code::Intn, i), &mut buf);
            assert_roundtrip(Header(Code::Bytes, i), &mut buf);
            assert_roundtrip(Header(Code::Str, i), &mut buf);
            assert_roundtrip(Header(Code::Key, i), &mut buf);
            assert_roundtrip(Header(Code::Container, i), &mut buf);
            assert_roundtrip(Header(Code::Fixed, i), &mut buf);
            assert_roundtrip(Header(Code::Reserved, i), &mut buf);
        }
    }

    #[test]
    fn roundtrip_long() {
        let mut buf = Vec::new();
        // choose large prime number to make this test terminate in acceptable time, in this case
        // (2^59-1)/179951
        for i in (0..u64::MAX).step_by(3_203_431_780_337) {
            assert_roundtrip(Header(Code::Intp, i), &mut buf);
            assert_roundtrip(Header(Code::Intn, i), &mut buf);
            assert_roundtrip(Header(Code::Bytes, i), &mut buf);
            assert_roundtrip(Header(Code::Str, i), &mut buf);
            assert_roundtrip(Header(Code::Key, i), &mut buf);
            assert_roundtrip(Header(Code::Container, i), &mut buf);
            assert_roundtrip(Header(Code::Fixed, i), &mut buf);
            assert_roundtrip(Header(Code::Reserved, i), &mut buf);
        }
    }

    fn assert_roundtrip(value: Header, buf: &mut Vec<u8>) {
        let _ = value.encode(buf);
        assert_eq!(value, Header::decode(buf).unwrap().0);
        buf.clear();
    }

}
