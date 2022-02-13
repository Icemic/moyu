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
