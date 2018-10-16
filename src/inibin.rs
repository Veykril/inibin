use byteorder::{ReadBytesExt, LE};
use indexmap::IndexMap;
use smallvec::SmallVec;

use std::io;

const BIT_I32: u8 = 0;
const BIT_F32: u8 = 1;
const BIT_F32_DIV_10: u8 = 2;
const BIT_I16: u8 = 3;
const BIT_I8: u8 = 4;
const BIT_BOOL: u8 = 5;
const BIT_F32_3_DIV_10: u8 = 6;
const BIT_F32_3: u8 = 7;
const BIT_F32_2_DIV_10: u8 = 8;
const BIT_F32_2: u8 = 9;
const BIT_F32_4_DIV_10: u8 = 10;
const BIT_F32_4: u8 = 11;
const BIT_STRING: u8 = 12;

use std::iter::once;

pub fn inibin_incremental_hash(prev: u32, next: &str) -> u32 {
    next.bytes().fold(prev, |hash, b| {
        hash.wrapping_mul(65599)
            .wrapping_add(u32::from(to_lower(b)))
    })
}

pub fn inibin_hash(section: &str, ident: &str) -> u32 {
    section
        .bytes()
        .chain(once(b'*'))
        .chain(ident.bytes())
        .fold(0u32, |hash, b| {
            hash.wrapping_mul(65599)
                .wrapping_add(u32::from(to_lower(b)))
        })
}

#[test]
fn test() {
    assert_eq!(
        inibin_hash("DeathTimeScaling", "StartTime"),
        inibin_incremental_hash(inibin_hash("DeathTimeScaling", ""), "StartTime")
    );
}

#[inline]
fn to_lower(b: u8) -> u8 {
    if b >= b'A' && b <= b'Z' {
        b - b'A' + b'a'
    } else {
        b
    }
}

#[cfg(not(feature = "serde"))]
#[derive(Clone, PartialOrd, PartialEq, Debug)]
pub enum Value {
    Integer(i64),
    Float(f32),
    Vec(SmallVec<[f32; 4]>),
    String(String),
}

#[cfg(feature = "serde")]
#[derive(Clone, PartialOrd, PartialEq, Debug)]
pub enum Value {
    U8(u8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    Bool(bool),
    Vec(SmallVec<[f32; 4]>),
    String(String),
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

macro_rules! impl_from_ty {
    (Vec = $( $from_ty:ty ),*) => {
        $(
            impl From<$from_ty> for Value {
                fn from(val: $from_ty) -> Self {
                    Value::Vec(val.into_iter().map(|v| *v).collect())
                }
            }
        )*
    };
    ($( $variant:ident = $( $from_ty:ty ),*);*;) => {
        $(
            $(
                impl From<$from_ty> for Value {
                    fn from(val: $from_ty) -> Self {
                        Value::$variant(val as _)
                    }
                }
            )*
        )*
    };
}

#[cfg(not(feature = "serde"))]
impl_from_ty! {
    Integer = i8, u8, i16, u16, i32, u32, i64, u64, bool;
    Float = f32;
}
#[cfg(feature = "serde")]
impl_from_ty! {
    U8 = i8, u8;
    I16 = i16, u16;
    I32 = i32, u32;
    I64 = i64, u64;
    Bool = bool;
    F32 = f32;
}
impl_from_ty!(Vec = [f32; 2], [f32; 3], [f32; 4]);

#[derive(Debug)]
pub struct IniBin {
    map: IndexMap<u32, Value>,
}

impl IniBin {
    pub fn map(&self) -> &IndexMap<u32, Value> {
        &self.map
    }

    pub fn into_map(self) -> IndexMap<u32, Value> {
        self.map
    }

    pub fn get(&self, section: &str, ident: &str) -> Option<&Value> {
        self.map.get(&inibin_hash(section, ident))
    }
}

// parsing
impl IniBin {
    pub fn from_bytes(b: &[u8]) -> io::Result<Self> {
        Self::from_reader(io::Cursor::new(b))
    }

    pub fn from_reader<R>(mut r: R) -> io::Result<Self>
    where
        R: io::Read + io::Seek,
    {
        let version = r.read_u8()?;
        if version == 0x01 {
            r.seek(io::SeekFrom::Current(3))?;
            Self::from_v1(r)
        } else if version == 0x02 {
            Self::from_v2(r)
        } else {
            Err(io::Error::from(io::ErrorKind::InvalidData))
        }
    }

    fn from_v1<R: io::Read>(mut r: R) -> io::Result<Self> {
        let entry_count = r.read_u32::<LE>()? as usize;
        let data_count = r.read_u32::<LE>()? as usize;

        let mut pairs: Vec<(u32, u32)> = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            pairs.push((r.read_u32::<LE>()?, r.read_u32::<LE>()?));
        }

        let mut buffer: Vec<u8> = Vec::with_capacity(data_count);
        r.read_to_end(&mut buffer)?;

        let mut inibin = IniBin {
            map: IndexMap::with_capacity(entry_count),
        };
        for (key, offset) in pairs {
            inibin.read_string(key, &buffer[offset as usize..])?;
        }
        Ok(inibin)
    }

    fn from_v2<R: io::Read>(mut r: R) -> io::Result<Self>
    where
        R: io::Read + io::Seek,
    {
        let mut inibin = IniBin {
            map: IndexMap::with_capacity(16),
        };
        let str_len = r.read_u16::<LE>()?;
        let flags: u16 = r.read_u16::<LE>()?;

        if is_bit_set(flags, BIT_I32) {
            inibin.read_section_numbers(&mut r, |r| r.read_i32::<LE>())?;
        }
        if is_bit_set(flags, BIT_F32) {
            inibin.read_section_numbers(&mut r, |r| r.read_f32::<LE>())?;
        }
        if is_bit_set(flags, BIT_F32_DIV_10) {
            inibin.read_section_numbers(&mut r, |r| r.read_u8().map(|b| f32::from(b) / 0.1))?;
        }
        if is_bit_set(flags, BIT_I16) {
            inibin.read_section_numbers(&mut r, |r| r.read_i16::<LE>())?;
        }
        if is_bit_set(flags, BIT_I8) {
            inibin.read_section_numbers(&mut r, |r| r.read_i8())?;
        }
        if is_bit_set(flags, BIT_BOOL) {
            inibin.read_section_bools(&mut r)?;
        }
        if is_bit_set(flags, BIT_F32_3_DIV_10) {
            inibin.read_section_numbers(&mut r, |r| {
                Ok([
                    f32::from(r.read_u8()?) * 0.1,
                    f32::from(r.read_u8()?) * 0.1,
                    f32::from(r.read_u8()?) * 0.1,
                ])
            })?;
        }
        if is_bit_set(flags, BIT_F32_3) {
            inibin.read_section_numbers(&mut r, |r| {
                Ok([
                    r.read_f32::<LE>()?,
                    r.read_f32::<LE>()?,
                    r.read_f32::<LE>()?,
                ])
            })?;
        }
        if is_bit_set(flags, BIT_F32_2_DIV_10) {
            inibin.read_section_numbers(&mut r, |r| {
                Ok([f32::from(r.read_u8()?) * 0.1, f32::from(r.read_u8()?) * 0.1])
            })?;
        }
        if is_bit_set(flags, BIT_F32_2) {
            inibin
                .read_section_numbers(&mut r, |r| Ok([r.read_f32::<LE>()?, r.read_f32::<LE>()?]))?;
        }
        if is_bit_set(flags, BIT_F32_4_DIV_10) {
            inibin.read_section_numbers(&mut r, |r| {
                Ok([
                    f32::from(r.read_u8()?) * 0.1,
                    f32::from(r.read_u8()?) * 0.1,
                    f32::from(r.read_u8()?) * 0.1,
                    f32::from(r.read_u8()?) * 0.1,
                ])
            })?;
        }
        if is_bit_set(flags, BIT_F32_4) {
            inibin.read_section_numbers(&mut r, |r| {
                Ok([
                    r.read_f32::<LE>()?,
                    r.read_f32::<LE>()?,
                    r.read_f32::<LE>()?,
                    r.read_f32::<LE>()?,
                ])
            })?;
        }
        if is_bit_set(flags, BIT_STRING) {
            inibin.read_section_strings(&mut r, str_len as usize)?;
        }

        Ok(inibin)
    }

    fn read_keys<R: io::Read>(mut r: R) -> io::Result<Vec<u32>> {
        let count = r.read_u16::<LE>()? as usize;
        let mut keys: Vec<u32> = Vec::with_capacity(count);
        for _ in 0..count {
            keys.push(r.read_u32::<LE>()?);
        }
        Ok(keys)
    }

    fn read_section_numbers<R, T, F>(&mut self, mut r: R, read_fn: F) -> io::Result<()>
    where
        R: io::Read,
        T: Into<Value>,
        F: Fn(&mut R) -> io::Result<T>,
    {
        let keys = Self::read_keys(&mut r)?;
        for key in keys {
            self.map.insert(key, read_fn(&mut r)?.into());
        }
        Ok(())
    }

    fn read_section_bools<R: io::Read>(&mut self, mut r: R) -> io::Result<()> {
        let keys = Self::read_keys(&mut r)?;
        let mut b = 0;
        for (idx, key) in keys.into_iter().enumerate() {
            let idx = idx % 8;
            if idx == 0 {
                b = r.read_u8()?;
            }
            self.map.insert(key, Value::from(b & (1 << idx) != 0));
        }
        Ok(())
    }

    fn read_section_strings<R: io::Read>(&mut self, mut r: R, str_len: usize) -> io::Result<()> {
        let keys = Self::read_keys(&mut r)?;
        let mut offsets: Vec<u16> = Vec::with_capacity(keys.len());

        for _ in 0..keys.len() {
            offsets.push(r.read_u16::<LE>()?);
        }
        let mut buffer: Vec<u8> = Vec::with_capacity(str_len);
        buffer.resize(str_len, 0);
        r.read_exact(&mut buffer)?;
        for (key, offset) in keys.into_iter().zip(offsets) {
            self.read_string(key, &buffer[offset as usize..])?;
        }
        Ok(())
    }

    fn read_string(&mut self, key: u32, buf: &[u8]) -> io::Result<()> {
        let end = buf
            .iter()
            .position(|b| *b == 0)
            .unwrap_or_else(|| buf.len());
        self.map.insert(
            key,
            Value::String(
                String::from_utf8(buf[..end].to_owned())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            ),
        );
        Ok(())
    }
}

// todo writing
impl IniBin {}

#[inline(always)]
fn is_bit_set(flags: u16, bit: u8) -> bool {
    flags & (1 << bit) != 0
}
