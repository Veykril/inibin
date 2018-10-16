use serde::de::{self, Deserialize, DeserializeSeed, MapAccess, Visitor};

use std::io;
use std::marker::PhantomData;

use crate::IniBin;
use crate::{error::Error, error::Result};

struct State {
    pub struct_name_hash: u32,
    pub fields: &'static [&'static str],
    pub current_field_hash: u32,
}

pub struct Deserializer<'de> {
    inibin: IniBin,
    stack: Vec<State>,
    _pd: PhantomData<&'de ()>,
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(input: &'de [u8]) -> io::Result<Self> {
        Ok(Deserializer {
            inibin: IniBin::from_bytes(input)?,
            stack: Vec::new(),
            _pd: PhantomData,
        })
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let current_frame = self.stack.last_mut().unwrap();
        match self.inibin.map().get(&current_frame.current_field_hash) {
            Some(val) => match val.clone() {
                crate::Value::U8(val) => visitor.visit_u8(val),
                crate::Value::I16(val) => visitor.visit_i16(val),
                crate::Value::I32(val) => visitor.visit_i32(val),
                crate::Value::I64(val) => visitor.visit_i64(val),
                crate::Value::F32(val) => visitor.visit_f32(val),
                crate::Value::Bool(val) => visitor.visit_bool(val),
                crate::Value::Vec(val) => unimplemented!(),
                crate::Value::String(val) => visitor.visit_string(val),
            },
            None => Err(Error::FieldNotFound(current_frame.current_field_hash)),
        }
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.stack.push(State {
            fields: fields,
            current_field_hash: 0,
            struct_name_hash: crate::inibin_hash(name, ""),
        });
        visitor.visit_map(MapThing {
            de: self,
            len: fields.len(),
        })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let current_frame = self.stack.last_mut().unwrap();
        if !current_frame.fields.is_empty() {
            //My MapAccess should prevent this if i am not mistaken
            let field = current_frame.fields[0];
            current_frame.current_field_hash =
                crate::inibin_incremental_hash(current_frame.struct_name_hash, field);
            current_frame.fields = &current_frame.fields[1..];
            visitor.visit_str(field)
        } else {
            unreachable!()
        }
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeUnsupported)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 f32 string
    }
}

pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(s)?;
    T::deserialize(&mut deserializer)
}

/// What the hell do I name this
struct MapThing<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    len: usize,
}

impl<'a, 'de: 'a> MapAccess<'de> for MapThing<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.len > 0 {
            self.len -= 1;
            seed.deserialize(&mut *self.de).map(Some)
        } else {
            self.de.stack.pop();
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}
