pub use anyhow::Error;
use std::sync::Arc;
pub use swc::TransformOutput;
use swc::{
    config::{Config, IsModule, JscConfig, ModuleConfig, Options},
    try_with_handler, Compiler, HandlerOpts,
};
use swc_core::ast::EsVersion;
use swc_core::common::{FileName, FilePathMapping, SourceMap};
use swc_ecma_parser::{EsConfig, Syntax, TsConfig};

#[derive(Debug, Clone)]
pub enum ScriptType {
    Typescript,
    Javascript,
}

pub fn transpile(source: &str, script_type: &ScriptType) -> Result<TransformOutput, Error> {
    let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));

    let c = Arc::new(Compiler::new(cm));

    try_with_handler(c.cm.clone(), HandlerOpts::default(), |handler| {
        let syntax = match script_type {
            ScriptType::Typescript => Syntax::Typescript(TsConfig {
                tsx: true,
                decorators: true,
                dts: false,
                no_early_errors: false,
            }),
            ScriptType::Javascript => Syntax::Es(EsConfig {
                jsx: true,
                decorators: true,
                export_default_from: true,
                ..Default::default()
            }),
        };

        let option = Options {
            config: Config {
                jsc: JscConfig {
                    syntax: Some(syntax),
                    target: Some(EsVersion::Es2022),
                    ..Default::default()
                },
                module: Some(ModuleConfig::Es6),
                minify: false.into(),
                is_module: IsModule::Bool(true),
                ..Default::default()
            },
            ..Default::default()
        };

        let fm = c.cm.new_source_file(FileName::Anon, source.into());
        c.process_js_file(fm, handler, &option)
    })
}

#[cfg(test)]
mod tests {
    use crate::{transpile, ScriptType};

    #[test]
    fn module_compiler_typescript() {
        let s = "import xx from 'sdsf'; const a: number = 1;xx();";
        let code = transpile(s, &ScriptType::Typescript).unwrap().code;
        assert_eq!(code, "import xx from 'sdsf';\nconst a = 1;\nxx();\n");
    }

    #[test]
    fn module_compiler_typescript_tsx() {
        let s = "function abc(){return <div foo='bar' />}";
        let code = transpile(s, &ScriptType::Typescript).unwrap().code;
        assert_eq!(code, "function abc() {\n    return /*#__PURE__*/ React.createElement(\"div\", {\n        foo: \"bar\"\n    });\n}\n");
    }

    #[test]
    fn module_compiler_javascript() {
        let s = "let arr = []; for (const item of arr) { doSomething(item); }";
        let code = transpile(s, &ScriptType::Javascript).unwrap().code;
        assert_eq!(
            code,
            "let arr = [];\nfor (const item of arr){\n    doSomething(item);\n}\n"
        );
    }

    #[test]
    fn module_compiler_javascript_jsx() {
        let s = "function abc(){return <div foo='bar' />}";
        let code = transpile(s, &ScriptType::Javascript).unwrap().code;
        assert_eq!(code, "function abc() {\n    return /*#__PURE__*/ React.createElement(\"div\", {\n        foo: \"bar\"\n    });\n}\n");
    }
}
