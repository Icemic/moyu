use doufu_runtime::{get_vm, setup_vm};
use log::error;

#[tokio::main]
async fn main() {
    doufu_pal::config::setup().await;
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let async_runtime_handle = doufu_pal::platform::setup();

    let vm_handle = setup_vm();

    std::thread::Builder::new()
        .name("quickjs".to_string())
        .spawn(|| {
            let vm = get_vm();

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

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // drop global variable
    drop(vm_handle);
    drop(async_runtime_handle);
}
