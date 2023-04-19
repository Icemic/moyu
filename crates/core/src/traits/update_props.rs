#[cfg(all(not(feature = "web"), feature = "js_runtime"))]
use crate::utils::convert::JSValue;

pub trait UpdateProps {
    #[cfg(all(not(feature = "web"), feature = "js_runtime"))]
    fn update_properties(&mut self, _props: &mut JSValue) {
        // defaults to do nothing
    }
}
