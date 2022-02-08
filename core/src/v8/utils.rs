use v8::{
    Array, Boolean, FunctionCallback, FunctionTemplate, HandleScope, Local, MapFnTo, NewStringType,
    Number, String,
};

pub(crate) struct Utils {}

pub trait Convert<T, U>: Sized {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: T) -> Local<'s, U>;
}

impl Convert<std::string::String, String> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: std::string::String) -> Local<'s, String> {
        String::new_from_utf8(scope, value.as_bytes(), NewStringType::Normal).unwrap()
    }
}

impl Convert<&str, String> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: &str) -> Local<'s, String> {
        String::new_from_utf8(scope, value.as_bytes(), NewStringType::Normal).unwrap()
    }
}

impl Convert<f64, Number> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: f64) -> Local<'s, Number> {
        Number::new(scope, value)
    }
}

impl Convert<i64, Number> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: i64) -> Local<'s, Number> {
        Number::new(scope, value as f64)
    }
}

impl Convert<u64, Number> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: u64) -> Local<'s, Number> {
        Number::new(scope, value as f64)
    }
}

impl Convert<f32, Number> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: f32) -> Local<'s, Number> {
        Number::new(scope, value as f64)
    }
}

impl Convert<i32, Number> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: i32) -> Local<'s, Number> {
        Number::new(scope, value as f64)
    }
}

impl Convert<u32, Number> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: u32) -> Local<'s, Number> {
        Number::new(scope, value as f64)
    }
}

impl Convert<i16, Number> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: i16) -> Local<'s, Number> {
        Number::new(scope, value as f64)
    }
}

impl Convert<u16, Number> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: u16) -> Local<'s, Number> {
        Number::new(scope, value as f64)
    }
}

impl Convert<i8, Number> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: i8) -> Local<'s, Number> {
        Number::new(scope, value as f64)
    }
}

impl Convert<u8, Number> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: u8) -> Local<'s, Number> {
        Number::new(scope, value as f64)
    }
}

impl Convert<bool, Boolean> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: bool) -> Local<'s, Boolean> {
        Boolean::new(scope, value)
    }
}

impl Convert<Vec<f64>, Array> for Utils {
    fn to_v8<'s>(scope: &mut HandleScope<'s>, value: Vec<f64>) -> Local<'s, Array> {
        let length = value.len();
        let mut _value = Vec::with_capacity(length);
        for v in value {
            _value.push(Self::to_v8(scope, v).into());
        }
        Array::new_with_elements(scope, &*_value)
    }
}

impl Utils {
    pub fn to_v8_func<'s>(
        scope: &mut HandleScope<'s>,
        value: impl MapFnTo<FunctionCallback>,
    ) -> Local<'s, FunctionTemplate> {
        FunctionTemplate::new(scope, value)
    }
}
