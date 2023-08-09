use darling::ToTokens;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemFn};
use syn::{GenericArgument, Pat, PathArguments, ReturnType, Type};

/// `fn some_api(core: Arc<Core>, foo: u32, bar: Option<String>) -> Result<(), String>`
/// to
/// `fn some_api(scope: &mut HandleScope, args: Local<Array>)`
pub fn entry(args: TokenStream, func_body: TokenStream) -> TokenStream {
    let _args = parse_macro_input!(args as AttributeArgs);
    let item = parse_macro_input!(func_body as ItemFn);

    let sig = item.sig;
    let body = item.block;

    let fn_name = sig.ident;
    let inputs = sig.inputs;

    let output = sig.output;

    let mut variable_blocks = vec![];
    let mut variable_names = vec![];

    let mut i = 0;
    for input in inputs.iter() {
        let variable_block = match input {
            syn::FnArg::Receiver(_r) => {
                panic!("does not support &self or &mut self as parameter.");
            }
            syn::FnArg::Typed(pat) => {
                let arg_name = &pat.pat;
                let arg_type = get_type_string(&pat.ty);
                let t = &pat.ty;

                if let Pat::Ident(arg_name_ident) = &**arg_name {
                    let ident = arg_name_ident.ident.clone();
                    variable_names.push(&arg_name_ident.ident);

                    match arg_type.as_str() {
                        "Arc<Core>" => {
                            i -= 1;
                            quote! {
                                let #ident = get_core();
                            }
                        }
                        "JSValue" => {
                            quote! {
                                let #ident = {
                                    JSValue::new(__context, __args[#i as usize])
                                };
                            }
                        }
                        _ => {
                            quote! {
                                let #ident: #t = {
                                    let value = JSValue::new(__context, __args[#i as usize]);
                                    from_js(&value)?
                                };
                            }
                        }
                    }
                } else {
                    quote!()
                }
            }
        };
        variable_blocks.push(variable_block);

        i += 1;
    }

    let fn_name_inner = format!("_{}_inner", fn_name);
    let fn_name_inner = Ident::new(fn_name_inner.as_str(), Span::call_site());

    // compared to the original function, only the function name is different
    // with a `__inner` suffix added
    let inner_func = quote! {
        #[inline]
        fn #fn_name_inner(#inputs) #output #body
    };

    let head = quote! {
        pub fn #fn_name(__context: *mut JSContext, __args: &[RawJSValue]) -> Result<Option<RawJSValue>>
    };

    let mut return_block = quote! {};

    if let ReturnType::Type(_, ty) = output {
        if let Type::Path(t) = &*ty {
            let name = get_type_string(&ty);

            if name.starts_with("Result<") {
                if !name.starts_with("Result<()") {
                    let return_type = &t.path.segments[0];
                    if let PathArguments::AngleBracketed(a) = &return_type.arguments {
                        let result_ok_type = a.args[0].to_token_stream();
                        return_block = quote! {
                            let ret = to_js::<#result_ok_type>(__context, &ret)?;
                            Ok(Some(unsafe { ret.extract() }))
                        };
                    }
                } else {
                    return_block = quote! { Ok(None::<RawJSValue>) };
                }
            } else {
                panic!("return type must be Result<T, E>.");
            }
        } else {
            panic!("return type must be Result<T, E>.");
        }
    }

    let body = quote! {
        #(#variable_blocks)*

        let ret = #fn_name_inner(#(#variable_names,)*).map_err(|err| anyhow::anyhow!(err));

        let ret = ret?;

        #return_block
    };

    quote! {
      #head {
          #body
      }
      #inner_func
    }
    .into()
}

fn get_type_string(_type: &Box<Type>) -> std::string::String {
    match &**_type {
        Type::Array(arr) => arr.into_token_stream().to_string(),
        Type::BareFn(_) => panic!("Unsupported BareFn type."),
        Type::Group(_) => panic!("Unsupported Group type."),
        Type::ImplTrait(_) => panic!("Unsupported ImplTrait type."),
        Type::Infer(_) => panic!("Unsupported Infer type."),
        Type::Macro(_) => panic!("Unsupported Macro type."),
        Type::Never(_) => panic!("Unsupported Never type."),
        Type::Paren(_) => panic!("Unsupported Paren type."),
        Type::Path(a) => {
            if let Some(_) = a.qself {
                panic!("Do not support path type with self");
            }
            let path = &a.path;
            if let Some(_) = path.leading_colon {
                panic!("Do not support path type with leading colon");
            }

            let mut s = "".to_string();

            for seg in path.segments.iter() {
                let ident = seg.ident.to_string();
                let arguments = &seg.arguments;

                match arguments {
                    PathArguments::None => {
                        s.push_str(ident.as_str());
                        s.push_str("::");
                    }
                    PathArguments::AngleBracketed(generic_value) => {
                        s.push_str(ident.as_str());
                        s.push('<');
                        let args = &generic_value.args;

                        for arg in args.iter() {
                            if let GenericArgument::Type(t) = arg {
                                let ident = get_type_string(&Box::new(t.clone()));
                                s.push_str(ident.as_str());
                                s.push_str(",");
                            } else {
                                panic!("Unsupported GenericArgument format: {:?}", arg);
                            }
                        }

                        // remove additional `,`
                        s.pop();
                        s.push('>');
                        s.push_str("::");
                    }
                    _ => panic!("Unsupported type path format."),
                }
            }
            s.pop();
            s.pop();
            s
        }
        Type::Ptr(_) => panic!("Unsupported Ptr type."),
        Type::Reference(a) => a.to_token_stream().to_string(),
        Type::Slice(_) => panic!("Unsupported Slice type."),
        Type::TraitObject(_) => panic!("Unsupported TraitObject type."),
        Type::Tuple(tuple) => tuple.to_token_stream().to_string(),
        Type::Verbatim(a) => a.to_string(),
        _ => panic!("unsupport type!"),
    }
}
