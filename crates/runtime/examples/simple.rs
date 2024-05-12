use hai_runtime::setup_vm;
use log::error;

#[tokio::main]
async fn main() {
    hai_pal::env::setup();
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    hai_pal::platform::setup();

    std::thread::Builder::new()
        .name("quickjs".to_string())
        .spawn(|| {
            let vm = setup_vm();

            vm.context()
                .eval("console.log('Hello %s!', 'World')", false)
                .unwrap();

            vm.context()
                .eval("var x = setInterval(() => console.log('Hello %s!', 'World'), 1000); setTimeout(() => clearTimeout(x), 1500)", false)
                .unwrap();


            if let Err(err) = vm.prepare_entry() {
                error!("{:?}", err);
            };

            vm.block_on_ticking();
        })
        .unwrap();

    loop {
        let future = std::future::pending();
        let () = future.await;
    }
}
