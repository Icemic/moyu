use proc_macro::TokenStream;
use proc_macro2::{Ident as Ident2, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse::Parser, parse_macro_input, Field, ItemStruct};

#[proc_macro_attribute]
pub fn node(_args: TokenStream, struct_body: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(struct_body as ItemStruct);

    if let syn::Fields::Named(ref mut fields) = ast.fields {
        fields.named.extend(get_node_fields());
    };

    let node_impl = get_node_impl(&ast.ident);
    let node_trait_impl = get_node_trait_impl(&ast.ident);
    let extra_traits = get_node_extra_traits(&ast.ident);

    quote! {
        #ast

        #node_impl

        #node_trait_impl

        #extra_traits
    }
    .into()
}

fn get_node_fields() -> Vec<Field> {
    let fields: Vec<TokenStream2> = vec![
        // Debug label
        quote! { pub label: String},
        // id
        quote! { pub id: u32},
        // anchor point
        quote! { pub anchor: PointF},
        // translate relative to parent
        quote! { pub translate: Point},
        // transform matrix relative to parent
        quote! { pub transform: Transform},
        // transform matrix relative to global
        quote! { pub transform_to_global: Transform },
        // children
        quote! { pub children: Vec<Arc<Mutex<dyn Node + Send>>> },
    ];

    fields
        .into_iter()
        .map(|field| syn::Field::parse_named.parse2(field).unwrap())
        .collect()
}

fn get_node_extra_traits(struct_name: &Ident2) -> TokenStream2 {
    quote! {
        impl PartialEq for #struct_name {
            fn eq(&self, other: &#struct_name) -> bool {
                self.id == other.id
            }
        }
    }
}

fn get_node_impl(struct_name: &Ident2) -> TokenStream2 {
    quote! {
        impl #struct_name {

        }
    }
}

fn get_node_trait_impl(struct_name: &Ident2) -> TokenStream2 {
    quote! {
        impl Node for #struct_name {
            fn node_type(&self) -> &'static str { todo!() }

            fn id(&self) -> &u32 {
                &self.id
            }

            fn label(&self) -> &String {
                &self.label
            }

            fn anchor(&self) -> &PointF {
                &self.anchor
            }
            fn translate(&self) -> &Point {
                &self.translate
            }
            fn transform(&self) -> &Transform {
                &self.transform
            }
            fn transform_to_global(&self) -> &Transform {
                &self.transform_to_global
            }
            fn children(&self) -> &Vec<Arc<Mutex<dyn Node + Send>>> {
                &self.children
            }

            // fn anchor_mut(&mut self) -> &mut PointF {
            //     &mut self.anchor
            // }
            // fn translate_mut(&mut self) -> &mut Point {
            //     &mut self.translate
            // }
            // fn transform_mut(&mut self) -> &mut Transform {
            //     &mut self.transform
            // }
            // fn transform_to_global_mut(&mut self) -> &mut Transform {
            //     &mut self.transform_to_global
            // }
            // fn children_mut(&mut self) -> &mut Vec<Arc<Mutex<dyn Node + Send>>> {
            //     &mut self.children
            // }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }

            fn get_child(&self, index: usize) -> Option<Arc<Mutex<dyn Node + Send>>> {
                if let Some(child) = self.children.get(index) {
                    return Some(child.clone());
                }
                None
            }

            fn add_child(&mut self, child: Arc<Mutex<dyn Node + Send>>) {
                self.children.push(child);
            }

            fn insert_child(&mut self, index: usize, child: Arc<Mutex<dyn Node + Send>>) {
                self.children.insert(index, child);
            }

            fn insert_child_before(
                &mut self,
                before_child: Arc<Mutex<dyn Node + Send>>,
                child: Arc<Mutex<dyn Node + Send>>,
            ) {
                let index = self.children.iter().position(|item| {
                    let l = item.lock().unwrap();
                    let r = child.lock().unwrap();
                    *l == *r
                });
                if index.is_none() {
                    warn!("Cannot insert child before another one because the another child does not present in current children.");
                }
                self.children.insert(index.unwrap_or(0), child);
            }

            fn remove_child(&mut self, child: Arc<Mutex<dyn Node + Send>>) -> Option<Arc<Mutex<dyn Node + Send>>> {
                if let Some(index) = self.children.iter().position(|item| {
                    let l = item.lock().unwrap();
                    let r = child.lock().unwrap();
                    *l == *r
                }) {
                    return Some(self.children.remove(index));
                }
                None
            }

            fn remove_child_at(&mut self, index: usize) -> Option<Arc<Mutex<dyn Node + Send>>> {
                if index < self.children.len() {
                    return Some(self.children.remove(index));
                }
                None
            }

            fn move_to(&mut self, x: i32, y: i32) {
                self.translate.x = x;
                self.translate.y = y;
            }

            fn calculate_transform(
                &mut self,
                parent_transform: &Transform,
                logical_size: LogicalSize<f64>,
                scale_factor: f64,
            ) {
                let x = self.translate.x;
                let y = self.translate.y;

                // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
                let tx = (x as f64 * scale_factor) / (logical_size.width * scale_factor) * 2.;
                let ty = (y as f64 * scale_factor) / (logical_size.height * scale_factor) * 2.;

                self.transform.tx = tx;
                self.transform.ty = ty;

                // TODO: rotate, scale and skew

                // refresh global transform matrix
                let mut transform_to_global = parent_transform.clone();
                transform_to_global.multiply(self.transform);
                self.transform_to_global = transform_to_global;
            }
        }
    }
}
