use quick_runtime::setup_vm;

#[tokio::main]
async fn main() {
    hai_pal::env::setup();
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    hai_pal::platform::setup();

    let local = tokio::task::LocalSet::new();

    local
        .run_until(async {
            let vm = setup_vm();

            vm.context()
                .eval("console.log('Hello %s!', 'World')")
                .unwrap();

            vm.context()
                .eval("var x = setInterval(() => console.log('Hello %s!', 'World'), 1000); setTimeout(() => clearTimeout(x), 1500)")
                .unwrap();

            // if let Err(err) = vm.prepare_entry() {
            //     println!("{:?}", err);
            // };

            // block
            let future = std::future::pending();
            let () = future.await;
        })
        .await;
}
