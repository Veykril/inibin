use serde::de::{
    self, Deserialize, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};

use crate::Result;

pub struct Deserializer<'de> {
    input: &'de [u8],
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer { input }
    }
}

/*
pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
let mut deserializer = Deserializer::from_bytes(s);
let t = T::deserialize(&mut deserializer)?;
if deserializer.input.is_empty() {
    Ok(t)
} else {
    unimplemented!()//Err(Error::TrailingCharacters)
}
}*/
