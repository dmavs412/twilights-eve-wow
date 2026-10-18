//! The seam on a byte stream: [`crate::RealmCommand`] in, `SessionEvent` and [`crate::Storage`]
//! out, each a frame of a little-endian `u32` length and a bincode body (ADR 0009).

use std::io::{self, Read, Write};

use serde::{de::DeserializeOwned, Serialize};

/// A client's first frame (#143, #144): who it is, the server's join password, and its
/// [`crate::build_id::build_id`]. A refusal is a lost `SessionEvent::Disconnected` and a close;
/// an accepted client's next frames are [`crate::RealmCommand`]s.
#[derive(Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    pub account: String,
    pub join_password: String,
    pub build_id: String,
}

/// The largest frame body either end writes or reads; a bigger one is a broken or hostile peer.
pub const MAX_FRAME: usize = 16 * 1024 * 1024;

fn invalid(e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

/// Writes `value` as one frame.
pub fn write_frame<W: Write, T: Serialize>(w: &mut W, value: &T) -> io::Result<()> {
    let body = bincode::serialize(value).map_err(invalid)?;
    if body.len() > MAX_FRAME {
        return Err(invalid(format!(
            "frame of {} bytes exceeds {MAX_FRAME}",
            body.len()
        )));
    }
    w.write_all(&(body.len() as u32).to_le_bytes())?;
    w.write_all(&body)
}

/// Reads one frame and decodes it; a truncated frame is `UnexpectedEof`.
pub fn read_frame<R: Read, T: DeserializeOwned>(r: &mut R) -> io::Result<T> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len)?;
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(invalid(format!("frame of {len} bytes exceeds {MAX_FRAME}")));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    bincode::deserialize(&body).map_err(invalid)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use benilla_protocol::SessionEvent;
    use serde::de::{self, value::Error, DeserializeSeed, IntoDeserializer, Visitor};

    use super::*;
    use crate::{RealmCommand, Storage};

    /// A deserializer that invents a value of any derived type, so every variant of a seam enum
    /// gets a round trip without a hand-written sample: `pick` chooses the outermost enum's
    /// variant (recording how many there are), nested enums take their first, and collections
    /// hold one element until `depth` runs out.
    #[derive(Clone, Copy)]
    struct Fill<'a> {
        depth: u32,
        pick: Option<u32>,
        variants: &'a Cell<usize>,
    }

    impl<'a> Fill<'a> {
        fn child(self) -> Self {
            Fill {
                depth: self.depth + 1,
                pick: None,
                variants: self.variants,
            }
        }
        fn len(self) -> usize {
            usize::from(self.depth < 5)
        }
    }

    struct Seq<'a>(Fill<'a>, usize);

    impl<'de> de::SeqAccess<'de> for Seq<'_> {
        type Error = Error;
        fn next_element_seed<T: DeserializeSeed<'de>>(
            &mut self,
            s: T,
        ) -> Result<Option<T::Value>, Error> {
            if self.1 == 0 {
                return Ok(None);
            }
            self.1 -= 1;
            s.deserialize(self.0.child()).map(Some)
        }
    }

    impl<'de> de::MapAccess<'de> for Seq<'_> {
        type Error = Error;
        fn next_key_seed<K: DeserializeSeed<'de>>(
            &mut self,
            s: K,
        ) -> Result<Option<K::Value>, Error> {
            de::SeqAccess::next_element_seed(self, s)
        }
        fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, s: V) -> Result<V::Value, Error> {
            s.deserialize(self.0.child())
        }
    }

    struct Variant<'a>(Fill<'a>, u32);

    impl<'de, 'a> de::EnumAccess<'de> for Variant<'a> {
        type Error = Error;
        type Variant = Fill<'a>;
        fn variant_seed<V: DeserializeSeed<'de>>(
            self,
            s: V,
        ) -> Result<(V::Value, Fill<'a>), Error> {
            Ok((s.deserialize(self.1.into_deserializer())?, self.0.child()))
        }
    }

    impl<'de> de::VariantAccess<'de> for Fill<'_> {
        type Error = Error;
        fn unit_variant(self) -> Result<(), Error> {
            Ok(())
        }
        fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, s: T) -> Result<T::Value, Error> {
            s.deserialize(self)
        }
        fn tuple_variant<V: Visitor<'de>>(self, len: usize, v: V) -> Result<V::Value, Error> {
            v.visit_seq(Seq(self, len))
        }
        fn struct_variant<V: Visitor<'de>>(
            self,
            f: &'static [&'static str],
            v: V,
        ) -> Result<V::Value, Error> {
            v.visit_seq(Seq(self, f.len()))
        }
    }

    macro_rules! fill_num {
        ($($m:ident $v:ident $x:expr;)*) => {$(
            fn $m<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> { v.$v($x) }
        )*};
    }

    impl<'de> de::Deserializer<'de> for Fill<'_> {
        type Error = Error;
        fill_num! {
            deserialize_bool visit_bool true;
            deserialize_i8 visit_i8 -1; deserialize_i16 visit_i16 -2; deserialize_i32 visit_i32 -3;
            deserialize_i64 visit_i64 -4; deserialize_i128 visit_i128 -5;
            deserialize_u8 visit_u8 1; deserialize_u16 visit_u16 2; deserialize_u32 visit_u32 3;
            deserialize_u64 visit_u64 4; deserialize_u128 visit_u128 5;
            deserialize_f32 visit_f32 1.5; deserialize_f64 visit_f64 2.5;
            deserialize_char visit_char 'x';
        }
        fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Error> {
            Err(de::Error::custom(
                "a seam type needs a self-describing format",
            ))
        }
        fn deserialize_str<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
            v.visit_string("x".into())
        }
        fn deserialize_string<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
            v.visit_string("x".into())
        }
        fn deserialize_bytes<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
            v.visit_byte_buf(vec![7])
        }
        fn deserialize_byte_buf<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
            v.visit_byte_buf(vec![7])
        }
        fn deserialize_option<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
            if self.len() == 0 {
                v.visit_none()
            } else {
                v.visit_some(self.child())
            }
        }
        fn deserialize_unit<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
            v.visit_unit()
        }
        fn deserialize_unit_struct<V: Visitor<'de>>(
            self,
            _: &'static str,
            v: V,
        ) -> Result<V::Value, Error> {
            v.visit_unit()
        }
        fn deserialize_newtype_struct<V: Visitor<'de>>(
            self,
            _: &'static str,
            v: V,
        ) -> Result<V::Value, Error> {
            v.visit_newtype_struct(self.child())
        }
        fn deserialize_seq<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
            v.visit_seq(Seq(self, self.len()))
        }
        fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, v: V) -> Result<V::Value, Error> {
            v.visit_seq(Seq(self, len))
        }
        fn deserialize_tuple_struct<V: Visitor<'de>>(
            self,
            _: &'static str,
            len: usize,
            v: V,
        ) -> Result<V::Value, Error> {
            v.visit_seq(Seq(self, len))
        }
        fn deserialize_map<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
            v.visit_map(Seq(self, self.len()))
        }
        fn deserialize_struct<V: Visitor<'de>>(
            self,
            _: &'static str,
            f: &'static [&'static str],
            v: V,
        ) -> Result<V::Value, Error> {
            v.visit_seq(Seq(self, f.len()))
        }
        fn deserialize_enum<V: Visitor<'de>>(
            self,
            _: &'static str,
            vs: &'static [&'static str],
            v: V,
        ) -> Result<V::Value, Error> {
            if self.pick.is_some() {
                self.variants.set(vs.len());
            }
            v.visit_enum(Variant(self, self.pick.unwrap_or(0)))
        }
        fn deserialize_identifier<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
            v.visit_u32(0)
        }
        fn deserialize_ignored_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
            v.visit_unit()
        }
    }

    /// Round-trips every variant of `T` through a frame; returns how many variants it saw.
    fn round_trip_every_variant<T: Serialize + DeserializeOwned + std::fmt::Debug>() -> usize {
        let variants = Cell::new(1);
        let mut pick = 0;
        while pick < variants.get() {
            let fill = Fill {
                depth: 0,
                pick: Some(pick as u32),
                variants: &variants,
            };
            let value = T::deserialize(fill).unwrap_or_else(|e| panic!("variant {pick}: {e}"));
            let mut bytes = Vec::new();
            write_frame(&mut bytes, &value).unwrap();
            let back: T = read_frame(&mut bytes.as_slice()).unwrap();
            assert_eq!(format!("{value:?}"), format!("{back:?}"), "variant {pick}");
            pick += 1;
        }
        variants.get()
    }

    #[test]
    fn every_seam_variant_round_trips() {
        assert!(round_trip_every_variant::<SessionEvent>() > 200);
        assert!(round_trip_every_variant::<RealmCommand>() > 40);
        assert_eq!(round_trip_every_variant::<Storage>(), 2);
    }

    #[test]
    fn oversized_length_prefix_is_an_error() {
        let bytes = ((MAX_FRAME + 1) as u32).to_le_bytes();
        let err = read_frame::<_, Storage>(&mut bytes.as_slice()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn truncated_frame_is_an_error() {
        let mut bytes = Vec::new();
        write_frame(
            &mut bytes,
            &Storage::Write {
                guid: 9,
                blob: "save".into(),
            },
        )
        .unwrap();
        bytes.truncate(bytes.len() - 2);
        let err = read_frame::<_, Storage>(&mut bytes.as_slice()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }
}
