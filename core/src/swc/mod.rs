use std::sync::Arc;
use swc::{
    common::{FileName, FilePathMapping, SourceMap},
    config::{Config, IsModule, JscConfig, ModuleConfig, Options},
    ecmascript::ast::EsVersion,
    try_with_handler, Compiler,
};
use swc_ecma_parser::{Syntax, TsConfig};

#[allow(dead_code)]
pub fn transpile(source: &str) -> String {
    let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));

    let c = Arc::new(Compiler::new(cm));

    try_with_handler(c.cm.clone(), false, |handler| {
        let option = Options {
            config: Config {
                jsc: JscConfig {
                    syntax: Some(Syntax::Typescript(TsConfig {
                        tsx: true,
                        decorators: false,
                        dts: false,
                        no_early_errors: false,
                    })),
                    target: Some(EsVersion::Es2022),
                    ..Default::default()
                },
                module: Some(ModuleConfig::Es6),
                minify: false,
                ..Default::default()
            },
            is_module: IsModule::Bool(true),
            ..Default::default()
        };

        let fm = c.cm.new_source_file(FileName::Anon, source.into());
        let out = c.process_js_file(fm, handler, &option).unwrap();

        Ok(out.code)
    })
    .unwrap()
}

#[test]
fn test_swc() {
    let s = "import xx from 'sdsf'; const a: number = 1;xx();";

    let code = transpile(s);
    println!("aaa: {}", code);
}
