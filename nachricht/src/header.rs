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

/// Define codes here as enum variants aren't types (yet)
#[repr(u8)]
#[derive(Clone, Copy)]
enum Code {
    BIN = 0,
    INT = 1,
    STR = 2,
    SYM = 3,
    ARR = 4,
    REC = 5,
    MAP = 6,
    REF = 7,
}

impl Code {

    /// The minimum value of payload that doesn't fit into sz and needs multibyte encoding
    /// for the given header
    const fn sz_limit(&self) -> u8 {
        match *self {
            Code::INT => (1 << 4) - 8,
            Code::BIN => (1 << 5) - 8 - 5,
            _         => (1 << 5) - 8,
        }
    }

}

impl TryFrom<u8> for Code {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == Code::BIN as u8 => Ok(Code::BIN),
            x if x == Code::INT as u8 => Ok(Code::INT),
            x if x == Code::STR as u8 => Ok(Code::STR),
            x if x == Code::SYM as u8 => Ok(Code::SYM),
            x if x == Code::ARR as u8 => Ok(Code::ARR),
            x if x == Code::REC as u8 => Ok(Code::REC),
            x if x == Code::MAP as u8 => Ok(Code::MAP),
            x if x == Code::REF as u8 => Ok(Code::REF),
            _ => Err(()),
        }
    }
}

// sz values
const NIL: u8 = 0;
const TRU: u8 = 1;
const FAL: u8 = 2;
const F32: u8 = 3;
const F64: u8 = 4;

// Signs: these are actually u1
const POS: u8 = 0;
const NEG: u8 = 1;

/// The sign of an integer. Note that the encoder accepts negative zero but transparently translates it to positive zero.
/// Likewise, decoders will accept the wire format for negative zero (which can only be achieved by purposefully chosing
/// an inefficient encoding) but return positive zero, so that testing the output doesn't need to concern itself with
/// another special case.
#[derive(Debug, PartialEq, Clone, Copy)] 
pub enum Sign { Pos, Neg }

impl Sign {
    fn code(&self) -> u8 {
        match *self {
            Sign::Pos => POS,
            Sign::Neg => NEG,
        }
    }
}

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
    /// Integer
    Int(Sign, u64),
    /// The value describes the length in bytes of a following unicode string
    Str(usize),
    /// The value describes the length in bytes of the symbol. It gets inserted into the symbol table
    Sym(usize),
    /// The value describes the length in fields of the array.
    Arr(usize),
    /// The value describes the length in entries of the record. The header is followed
    /// by all keys of the record and subsequently by the values. This is to enable efficient
    /// encoding of recursive data structures as the record's layout can get inserted into the
    /// symbol table before the value is completed
    Rec(usize),
    /// The value describes the length in entries of the map. The fields are encoded in
    /// key value key value ... order.
    Map(usize),
    /// A reference into the symbol table. This could resolve to either a symbol or a record layout.
    /// In the former case, the symbol is the value,
    /// in the latter case, the header is followed by the fields of the record, whereas the keys
    /// are defined in the symbol table and are therefore omitted.
    Ref(usize),
}

impl Header {

    /// Returns the mnemonic of the header. This is useful for error messages.
    pub fn name(&self) -> &'static str {
        match *self {
            Header::Null      => "Null",
            Header::True      => "True",
            Header::False     => "False",
            Header::F32       => "F32",
            Header::F64       => "F64",
            Header::Bin(_)    => "Bin",
            Header::Int(_, _) => "Int",
            Header::Str(_)    => "Str",
            Header::Sym(_)    => "Sym",
            Header::Arr(_)    => "Arr",
            Header::Rec(_)    => "Rec",
            Header::Map(_)    => "Map",
            Header::Ref(_)    => "Ref",
        }
    }

    /// Returns the number of written bytes
    pub fn encode<W: Write>(&self, w: &mut W) -> Result<usize, EncodeError> {
        match *self {
            Header::Null                    => { w.write_all(&[self.code_bits() << self.shift() | NIL])?; Ok(1) },
            Header::True                    => { w.write_all(&[self.code_bits() << self.shift() | TRU])?; Ok(1) },
            Header::False                   => { w.write_all(&[self.code_bits() << self.shift() | FAL])?; Ok(1) },
            Header::F32                     => { w.write_all(&[self.code_bits() << self.shift() | F32])?; Ok(1) },
            Header::F64                     => { w.write_all(&[self.code_bits() << self.shift() | F64])?; Ok(1) },
            Header::Int(Sign::Neg, 0)       => { Header::Int(Sign::Pos, 0).encode(w) },
            Header::Int(Sign::Pos, i)       => self.encode_long_header(i, w),
            Header::Int(Sign::Neg, i)       => self.encode_long_header(i - 1, w),
            Header::Bin(i)
                | Header::Str(i)
                | Header::Sym(i)
                | Header::Arr(i)
                | Header::Rec(i)
                | Header::Map(i)
                | Header::Ref(i)            => self.encode_long_header(Self::to_u64(i)?, w)
        }
    }

    /// Returns the decoded header and the number of consumed bytes
    pub fn decode<B: ?Sized + AsRef<[u8]>>(buf: &B) -> Result<(Self, usize), DecodeError> {
        let shift = 5;
        let buf = buf.as_ref();
        if buf.len() < 1 {
            return Err(DecodeError::Eof);
        }
        let code = (buf[0] >> shift).try_into().unwrap();
        let sz = buf[0] & ((1 << shift) - 1);
        match code {
            Code::BIN => {
                match sz {
                    NIL => Ok((Header::Null,  1)),
                    TRU => Ok((Header::True,  1)),
                    FAL => Ok((Header::False, 1)),
                    F32 => Ok((Header::F32,   1)),
                    F64 => Ok((Header::F64,   1)),
                    x => Self::decode_u64(&buf[1..], x - 5, Code::BIN.sz_limit()).and_then(|(i, c)| Ok((Header::Bin(Self::to_usize(i)?), c + 1))),
                }
            },
            Code::INT => {
                let sign = sz >> (shift - 1);
                let sz = sz & ((1 << (shift - 1)) - 1);
                match sign {
                    POS => Self::decode_u64(&buf[1..], sz, Code::INT.sz_limit()).map(|(i, c)| (Header::Int(Sign::Pos, i), c + 1)),
                    NEG => Self::decode_u64(&buf[1..], sz, Code::INT.sz_limit()).map(|(i, c)| (Header::Int(Sign::Neg, i.saturating_add(1)), c + 1)),
                    _   => unreachable!(),
                }
            },
            Code::STR => Self::decode_u64(&buf[1..], sz, Code::STR.sz_limit()).and_then(|(i, c)| Ok((Header::Str(Self::to_usize(i)?), c + 1))),
            Code::SYM => Self::decode_u64(&buf[1..], sz, Code::SYM.sz_limit()).and_then(|(i, c)| Ok((Header::Sym(Self::to_usize(i)?), c + 1))),
            Code::ARR => Self::decode_u64(&buf[1..], sz, Code::ARR.sz_limit()).and_then(|(i, c)| Ok((Header::Arr(Self::to_usize(i)?), c + 1))),
            Code::REC => Self::decode_u64(&buf[1..], sz, Code::REC.sz_limit()).and_then(|(i, c)| Ok((Header::Rec(Self::to_usize(i)?), c + 1))),
            Code::MAP => Self::decode_u64(&buf[1..], sz, Code::MAP.sz_limit()).and_then(|(i, c)| Ok((Header::Map(Self::to_usize(i)?), c + 1))),
            Code::REF => Self::decode_u64(&buf[1..], sz, Code::REF.sz_limit()).and_then(|(i, c)| Ok((Header::Ref(Self::to_usize(i)?), c + 1))),
        }
    }

    #[inline]
    fn encode_long_header<W: Write>(&self, i: u64, w: &mut W) -> Result<usize, EncodeError> {
        let limit = self.code().sz_limit();
        let offset = match *self { Header::Bin(_) => 5, _ => 0 };
        if i < limit as u64 {
            w.write_all(&[self.code_bits() << self.shift() | i as u8 + offset])?;
            Ok(1)
        } else {
            let sz = Self::size(i);
            let buf = i.to_be_bytes();
            w.write_all(&[self.code_bits() << self.shift() | (sz + limit + offset - 1)])?;
            w.write_all(&buf[buf.len() - sz as usize ..])?;
            Ok(1 + sz as usize)
        }
    }

    #[inline]
    fn decode_u64(buf: &[u8], sz: u8, limit: u8) -> Result<(u64, usize), DecodeError> {
        if sz < limit {
            Ok((sz as u64, 0))
        } else {
            let bytes = sz as usize - limit as usize + 1;
            if buf.len() < bytes {
                Err(DecodeError::Eof)
            } else {
                let mut tmp = [0u8; 8];
                tmp[8 - bytes..].copy_from_slice(&buf[..bytes]);
                Ok((<u64>::from_be_bytes(tmp), bytes))
            }
        }
    }

    #[inline]
    fn code(&self) -> Code {
        match *self {
            Header::Null | Header::True | Header::False | Header::F32 | Header::F64 | Header::Bin(_) => Code::BIN,
            Header::Int(_,_)                                                                         => Code::INT,
            Header::Str(_)                                                                           => Code::STR,
            Header::Sym(_)                                                                           => Code::SYM,
            Header::Arr(_)                                                                           => Code::ARR,
            Header::Rec(_)                                                                           => Code::REC,
            Header::Map(_)                                                                           => Code::MAP,
            Header::Ref(_)                                                                           => Code::REF,
        }
    }

    #[inline]
    fn code_bits(&self) -> u8 {
        match *self {
            Header::Int(s, _) => ((self.code() as u8) << 1) | s.code(),
            _                 => self.code() as u8,
        }
    }

    #[inline]
    fn shift(&self) -> u8 {
        match *self {
            Header::Int(_,_) => 4,
            _                => 5,
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
    use super::{Sign, Header};

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
    fn negative_zero() {
        let mut buf = Vec::new();
        let _ = Header::Int(Sign::Neg, 0).encode(&mut buf);
        assert_eq!(Header::Int(Sign::Pos, 0), Header::decode(&buf).unwrap().0);
    }

    #[test]
    fn negative_max() {
        let buf = [0x3f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
        assert_eq!(Header::Int(Sign::Neg, u64::MAX), Header::decode(&buf).unwrap().0);
        let buf = [0x3f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe];
        assert_eq!(Header::Int(Sign::Neg, u64::MAX), Header::decode(&buf).unwrap().0);
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
            assert_roundtrip(Header::Int(Sign::Pos, i as u64), &mut buf);
            assert_roundtrip(Header::Int(Sign::Neg, if i == 0 { 1 } else { i } as u64), &mut buf);
            assert_roundtrip(Header::Arr(i), &mut buf);
            assert_roundtrip(Header::Map(i), &mut buf);
            assert_roundtrip(Header::Str(i), &mut buf);
            assert_roundtrip(Header::Sym(i), &mut buf);
            assert_roundtrip(Header::Rec(i), &mut buf);
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
            assert_roundtrip(Header::Int(Sign::Pos, i), &mut buf);
            assert_roundtrip(Header::Int(Sign::Neg, if i == 0 { 1 } else { i } as u64), &mut buf);
            assert_roundtrip(Header::Str(i as usize), &mut buf);
            assert_roundtrip(Header::Sym(i as usize), &mut buf);
            assert_roundtrip(Header::Arr(i as usize), &mut buf);
            assert_roundtrip(Header::Rec(i as usize), &mut buf);
            assert_roundtrip(Header::Map(i as usize), &mut buf);
            assert_roundtrip(Header::Ref(i as usize), &mut buf);
        }
    }

    #[test]
    fn inefficient_encoding() {
        let buf = [0x9f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02];
        assert_eq!(Header::Arr(2), Header::decode(&buf).unwrap().0);
    }

    fn assert_roundtrip(value: Header, buf: &mut Vec<u8>) {
        let _ = value.encode(buf);
        assert_eq!(value, Header::decode(buf).unwrap().0);
        buf.clear();
    }

}
