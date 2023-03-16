use crate::utils::convert::JSValue;

pub trait UpdateProps {
    fn update_properties(&mut self, _props: &mut JSValue) {
        // defaults to do nothing
    }
}
