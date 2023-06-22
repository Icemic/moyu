use quick_runtime::setup_vm;

pub fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let vm = setup_vm();

    vm.context()
        .eval("console.log('Hello %s!', 'World')")
        .unwrap();
}
