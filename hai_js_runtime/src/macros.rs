#[macro_export]
macro_rules! bind_object {
    (to $global:expr; of $scope:expr; $($object_name:expr => { $($func_name:expr => $func_ref:expr),+ }),+) => {{
        $(
            let object = ObjectTemplate::new($scope);
            $({
                let func = FunctionTemplate::new($scope, $func_ref);
                object.set(
                    String::new($scope, $func_name).unwrap().into(),
                    func.into(),
                );
            })+
            let object_name = String::new($scope, $object_name).unwrap().into();
            let object_instance = object.new_instance($scope).unwrap();
            $global.set($scope, object_name, object_instance.into());
        )+
    }};
}

#[macro_export]
macro_rules! bind_to_object {
    (to $instance:expr; of $scope:expr; is methods; $($func_name:expr => $func_ref:expr),*) => {{
        $(
            $instance.set(
                Utils::to_v8($scope, $func_name).into(),
                Utils::to_v8_func($scope, $func_ref).into(),
            );
        )*
    }};
    (to $instance:expr; of $scope:expr; is properties; $($prop_name:expr => ($prop_getter:expr, $prop_setter:expr)),*) => {{
        $(
            $instance.set_accessor_with_setter(
                Utils::to_v8($scope, $prop_name).into(),
                $prop_getter,
                $prop_setter,
            );
        )*
    }};
}

#[macro_export]
macro_rules! bind_function {
    (to $global:expr; of $scope:expr; $($name:expr => $func_ref:expr),+) => {{
        $(
            let func = FunctionTemplate::new($scope, $func_ref);
            let name = String::new($scope, $name).unwrap().into();
            let instance = func.get_function($scope).unwrap();
            $global.set($scope, name, instance.into());
        )+
    }};
}

#[macro_export]
macro_rules! get_v8_number {
    ($scope:expr, $var:expr, $type:ty, $default:expr) => {
        match Local::<Number>::try_from($var) {
            Ok(v) => v.value() as $type,
            Err(err) => {
                let err = format!("{}", err);
                let error_message: Local<String> = Utils::to_v8($scope, err);
                let error = Exception::type_error($scope, error_message);
                $scope.throw_exception(error);
                $default
            }
        };
    };
}

#[macro_export]
macro_rules! save_to_holder {
    ($scope:expr, $args:expr, $target:expr) => {
        let holder = $args.holder();
        let instance = Rc::new(RefCell::new($target));
        let instance_ptr = Rc::into_raw(instance) as *mut c_void;
        let field = External::new($scope, instance_ptr).into();
        holder.set_internal_field(0, field);
    };
}

#[macro_export]
macro_rules! unwrap {
    ($scope:expr, $object:expr, $t:ty, |$name:ident| $block:block) => {{
        let v: Local<External> = $object
            .get_internal_field($scope, 0)
            .unwrap()
            .try_into()
            .unwrap();
        let v_rc: Rc<RefCell<$t>> = unsafe { Rc::from_raw(v.value() as *const RefCell<$t>) };
        if let Ok($name) = v_rc.clone().try_borrow_mut() {
            // consumes the rc
            Rc::into_raw(v_rc);
            $block;
        } else {
            panic!("Failed to unwrap {}.", stringify!($t));
        }
    }};
    ($scope:expr, $object:expr, $t:ty, |mut $name:ident| $block:block) => {{
        let v: Local<External> = $object
            .get_internal_field($scope, 0)
            .unwrap()
            .try_into()
            .unwrap();
        let v_rc: Rc<RefCell<$t>> = unsafe { Rc::from_raw(v.value() as *const RefCell<$t>) };
        if let Ok(mut $name) = v_rc.clone().try_borrow_mut() {
            // consumes the rc
            Rc::into_raw(v_rc);
            $block;
        } else {
            panic!("Failed to unwrap {}.", stringify!($t));
        }
    }};
}

/**
 * do type check and return value, or throw to js.
 */
#[macro_export]
macro_rules! try_from_value_or_throw_exception {
    ($scope:ident, $v8_local_type:ty, $v8_local_value:expr) => {
        match Local::<$v8_local_type>::try_from($v8_local_value) {
            Ok(v) => v,
            Err(err) => {
                throw_exception!($scope, format!("{}", err));
                return;
            }
        }
    };
}

/**
 * do type check and return Some(value), or return None
 */
#[macro_export]
macro_rules! try_from_option_value_or_throw_exception {
    ($scope:ident, $v8_local_type:ty, $v8_local_value:expr) => {
        if $v8_local_value.is_null_or_undefined() {
            None
        } else {
            match Local::<$v8_local_type>::try_from($v8_local_value) {
                Ok(v) => Some(v),
                Err(err) => {
                    throw_exception!($scope, format!("{}", err));
                    return;
                }
            }
        }
    };
}

#[macro_export]
macro_rules! throw_exception {
    ($scope:ident, $string:expr) => {{
        let error_message: Local<String> = $string.into_v8($scope);
        let error = Exception::error($scope, error_message);
        $scope.throw_exception(error);
    }};
}

#[macro_export]
macro_rules! get_shared_state {
    ($scope:ident, $t:ty) => {{
        let shared = $scope.get_slot::<Rc<RefCell<Shared>>>().unwrap();
        let shared = shared.borrow();

        shared.state::<$t>()
    }};
}

/**
 * get value from v8 array by index.
 */
#[macro_export]
macro_rules! get_from_v8_array {
    ($scope:ident, $args:ident, $index:expr) => {{
        $args.get_index($scope, $index as u32).unwrap()
    }};
}

/**
 * get value from v8 array by index.
 */
#[macro_export]
macro_rules! get_from_v8_object {
    ($scope:ident, $args:ident, $key:expr) => {{
        let key = $key.into_v8($scope).into();
        let value = $args.get($scope, key);
        value.unwrap()
    }};
}

/**
 * check a js command parameter is not undefined, or throw to js.
 */
#[macro_export]
macro_rules! check_exist {
    ($scope:ident, $v:ident) => {
        if $v.is_null_or_undefined() {
            throw_exception!(
                $scope,
                format!("parameter {} must be specified.", stringify!($v))
            );
            return;
        }
    };
}
