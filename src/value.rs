use std::borrow::Cow;

/// One prop value: text, a number, true or false, or a list.
///
/// You rarely build one by hand. Anything that converts with `.into()` works:
/// `&str`, `String`, `bool`, every integer type, `f32`, `f64`, `Option<T>`
/// (`None` means "leave it out"), and `Vec<T>` or arrays for lists.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// No value. The prop is left out. In per-call page props it removes a
    /// configured prop with the same key.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A whole number too big for `i64`.
    UInt(u64),
    /// A number with a fraction.
    Float(f64),
    /// Text.
    Str(String),
    /// A list of values.
    List(Vec<Value>),
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

macro_rules! from_signed {
    ($($ty:ty),*) => {$(
        impl From<$ty> for Value {
            fn from(value: $ty) -> Self {
                Value::Int(value as i64)
            }
        }
    )*};
}
from_signed!(i8, i16, i32, i64, isize);

macro_rules! from_unsigned {
    ($($ty:ty),*) => {$(
        impl From<$ty> for Value {
            fn from(value: $ty) -> Self {
                let value = value as u64;
                if value <= i64::MAX as u64 {
                    Value::Int(value as i64)
                } else {
                    Value::UInt(value)
                }
            }
        }
    )*};
}
from_unsigned!(u8, u16, u32, u64, usize);

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float(value)
    }
}

impl From<f32> for Value {
    /// Goes through text so `1.1_f32` stays `1.1` and does not become `1.100000023841858`.
    fn from(value: f32) -> Self {
        Value::Float(value.to_string().parse().unwrap_or(value as f64))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Str(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Str(value)
    }
}

impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Value::Str(value.clone())
    }
}

impl From<Cow<'_, str>> for Value {
    fn from(value: Cow<'_, str>) -> Self {
        Value::Str(value.into_owned())
    }
}

impl From<char> for Value {
    fn from(value: char) -> Self {
        Value::Str(value.to_string())
    }
}

impl From<&Value> for Value {
    fn from(value: &Value) -> Self {
        value.clone()
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        value.map_or(Value::Null, Into::into)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        Value::List(value.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>, const N: usize> From<[T; N]> for Value {
    fn from(value: [T; N]) -> Self {
        Value::List(value.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value> + Clone> From<&[T]> for Value {
    fn from(value: &[T]) -> Self {
        Value::List(value.iter().cloned().map(Into::into).collect())
    }
}

/// An ordered list of props: key and value pairs.
///
/// Build it with the [`props!`](crate::props) macro, with [`Props::with`], or
/// collect it from an iterator of pairs. Setting a key that is already there
/// replaces its value and keeps its place.
///
/// ```
/// use privatus_analytics::{props, Props};
///
/// let a = props! { "plan" => "pro", "seats" => 5, "beta" => true };
/// let b = Props::new().with("plan", "pro").with("seats", 5).with("beta", true);
/// assert_eq!(a, b);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Props(Vec<(String, Value)>);

impl Props {
    /// No props.
    pub fn new() -> Self {
        Props(Vec::new())
    }

    /// Sets a prop and returns the props, for chaining.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.insert(key, value);
        self
    }

    /// Sets a prop. A key that is already there gets the new value.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        let key = key.into();
        let value = value.into();
        match self.0.iter_mut().find(|(existing, _)| *existing == key) {
            Some(slot) => slot.1 = value,
            None => self.0.push((key, value)),
        }
    }

    /// The value for a key, if there is one.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// The number of props.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no props.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The props in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl<K: Into<String>, V: Into<Value>> FromIterator<(K, V)> for Props {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut props = Props::new();
        for (key, value) in iter {
            props.insert(key, value);
        }
        props
    }
}

impl<K: Into<String>, V: Into<Value>> Extend<(K, V)> for Props {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

impl IntoIterator for Props {
    type Item = (String, Value);
    type IntoIter = std::vec::IntoIter<(String, Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Props {
    type Item = (&'a String, &'a Value);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (String, Value)>,
        fn(&'a (String, Value)) -> (&'a String, &'a Value),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|(k, v)| (k, v))
    }
}

/// Builds [`Props`] from `key => value` pairs.
///
/// ```
/// use privatus_analytics::props;
///
/// let props = props! { "plan" => "pro", "role" => "admin", "seats" => 5 };
/// assert_eq!(props.len(), 3);
/// let none = props! {};
/// assert!(none.is_empty());
/// ```
#[macro_export]
macro_rules! props {
    () => { $crate::Props::new() };
    ($($key:expr => $value:expr),+ $(,)?) => {{
        let mut props = $crate::Props::new();
        $( props.insert($key, $value); )+
        props
    }};
}
