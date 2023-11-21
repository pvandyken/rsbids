use core::fmt;
use std::{
    marker::PhantomData,
    ops::Deref,
    path::{Path, PathBuf},
};

use self_cell::self_cell;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

#[derive(Debug, Clone)]
pub struct UtfView<'a>(pub &'a str);

impl<'a> From<&'a str> for UtfView<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl Deref for UtfView<'_> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

self_cell!(
    pub struct UtfPath {
        owner: PathBuf,

        #[covariant]
        dependent: UtfView,
    }

    impl {Debug}
);
impl Serialize for UtfPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let p = self.borrow_dependent();
        serializer.serialize_str(p)
    }
}

struct UtfPathVisitor(PhantomData<PathBuf>);
impl<'de> Visitor<'de> for UtfPathVisitor {
    type Value = UtfPath;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a str")
    }

    fn visit_str<E: de::Error>(self, deserializer: &str) -> Result<Self::Value, E> {
        Ok(UtfPath::new(PathBuf::from(deserializer), |s| {
            s.to_str().expect("Path should be valid unicode").into()
        }))
    }
}

impl<'de> Deserialize<'de> for UtfPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(UtfPathVisitor(PhantomData))
    }
}

impl Clone for UtfPath {
    fn clone(&self) -> Self {
        Self::new(self.borrow_owner().clone(), |s| {
            s.to_str().expect("Shouldn't fail on clone").into()
        })
    }
}

impl TryFrom<PathBuf> for UtfPath {
    type Error = PathBuf;
    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.to_str().is_none() {
            Err(value)
        } else {
            Ok(Self::new(value, |s| s.to_str().unwrap().into()))
        }
    }
}

impl AsRef<Path> for UtfPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<str> for UtfPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl UtfPath {
    #[inline]
    pub fn as_path(&self) -> &Path {
        self.borrow_owner()
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.borrow_dependent()
    }
}
