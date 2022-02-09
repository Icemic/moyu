use std::path::{Path, PathBuf};

use v8::{
    Array, Boolean, FunctionCallback, FunctionTemplate, HandleScope, Local, MapFnTo, NewStringType,
    Number, String,
};

pub trait IntoV8<T> {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, T>;
}

impl IntoV8<String> for std::string::String {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, String> {
        String::new_from_utf8(scope, self.as_bytes(), NewStringType::Normal).unwrap()
    }
}

impl IntoV8<String> for &str {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, String> {
        String::new_from_utf8(scope, self.as_bytes(), NewStringType::Normal).unwrap()
    }
}

impl IntoV8<Number> for f64 {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Number> {
        Number::new(scope, self)
    }
}

impl IntoV8<Number> for i64 {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Number> {
        Number::new(scope, self as f64)
    }
}

impl IntoV8<Number> for u64 {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Number> {
        Number::new(scope, self as f64)
    }
}

impl IntoV8<Number> for f32 {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Number> {
        Number::new(scope, self as f64)
    }
}

impl IntoV8<Number> for i32 {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Number> {
        Number::new(scope, self as f64)
    }
}

impl IntoV8<Number> for u32 {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Number> {
        Number::new(scope, self as f64)
    }
}

impl IntoV8<Number> for i16 {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Number> {
        Number::new(scope, self as f64)
    }
}

impl IntoV8<Number> for u16 {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Number> {
        Number::new(scope, self as f64)
    }
}

impl IntoV8<Number> for i8 {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Number> {
        Number::new(scope, self as f64)
    }
}

impl IntoV8<Number> for u8 {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Number> {
        Number::new(scope, self as f64)
    }
}

impl IntoV8<Boolean> for bool {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Boolean> {
        Boolean::new(scope, self)
    }
}

impl IntoV8<Array> for Vec<f64> {
    fn into_v8<'s>(self, scope: &mut HandleScope<'s>) -> Local<'s, Array> {
        let length = self.len();
        let mut _self = Vec::with_capacity(length);
        for v in self {
            _self.push(v.into_v8(scope).into());
        }
        Array::new_with_elements(scope, &*_self)
    }
}

#[allow(dead_code)]
pub fn v8_func<'s>(
    scope: &mut HandleScope<'s>,
    value: impl MapFnTo<FunctionCallback>,
) -> Local<'s, FunctionTemplate> {
    FunctionTemplate::new(scope, value)
}

pub fn try_find_file(dir: &PathBuf, filename: &str, extensions: Vec<&str>) -> Option<PathBuf> {
    let p = PathBuf::from(dir).join(filename);
    for ext in extensions {
        let p = p.with_extension(ext);
        if p.exists() {
            return Some(p.canonicalize().unwrap());
        }
    }
    None
}
