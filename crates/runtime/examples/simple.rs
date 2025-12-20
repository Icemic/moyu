use moyu_runtime::{get_vm, setup_vm};

#[tokio::main]
async fn main() {
    // moyu_pal::config::setup().await;
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let async_runtime_handle = moyu_pal::platform::setup();

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

            vm.context()
                .eval(include_str!("echo.client.js"), false)
                .unwrap();

            // 运行 fetch 测试
            vm.context()
                .eval(include_str!("eval.client.js"), false)
                .unwrap();

            vm.context()
                .eval(include_str!("fetch.client.js"), false)
                .unwrap();

            // 运行 DOM/脚本加载测试
            vm.context()
                .eval(include_str!("dom.client.js"), false)
                .unwrap();

            // if let Err(err) = vm.prepare_entry() {
            //     log::error!("{:?}", err);
            // };
        })
        .unwrap();

    let vm = get_vm();
    vm.block_on_ticking();

    // drop global variable
    drop(vm_handle);
    drop(async_runtime_handle);
}
