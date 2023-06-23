use quick_runtime::setup_vm;

#[tokio::main]
async fn main() {
    hai_pal::env::setup();
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    hai_pal::platform::setup();

    std::thread::spawn(move || {
        let vm = setup_vm();

        vm.context()
            .eval("console.log('Hello %s!', 'World')")
            .unwrap();

        if let Err(err) = vm.prepare_entry() {
            println!("{:?}", err);
        };
    })
    .join()
    .unwrap();
}
