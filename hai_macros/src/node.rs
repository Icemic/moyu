use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::{Ident as Ident2, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse::Parser, parse_macro_input, AttributeArgs, Field, ItemStruct};

#[derive(Debug, Default, FromMeta)]
#[darling(default)]
struct NodeArgs {
    renderable: bool,
}

pub fn node(args: TokenStream, struct_body: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let mut ast = parse_macro_input!(struct_body as ItemStruct);

    if let syn::Fields::Named(ref mut fields) = ast.fields {
        fields.named.extend(get_node_fields());
    };

    let args = match NodeArgs::from_list(&args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let node_impl = get_node_impl(&ast.ident);
    let node_trait_impl = get_node_trait_impl(&ast.ident, args.renderable);
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
        quote! { anchor: Point},
        // pivot point
        quote! { pivot: Point},
        // translate relative to parent
        quote! { translate: Point},
        // scale relative to parent
        quote! { scale: Point},
        // rotation relative to parent
        quote! { rotation: f64},
        // skew relative to parent
        quote! { skew: Point},
        // for update transform dirty check
        quote! { _update_id: u32},
        quote! { _current_update_id: u32},
        // transform matrix relative to parent
        quote! { pub transform: Transform},
        // transform matrix relative to global
        quote! { pub global_transform: Transform },
        // children
        quote! { pub children: Vec<Arc<RwLock<dyn Node>>> },
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

fn get_node_trait_impl(struct_name: &Ident2, renderable: bool) -> TokenStream2 {
    let base = quote! {
        fn node_type(&self) -> &'static str {
            unreachable!("Should not call Node::node_type, use NodeType::node_type(&node) instead.");
        }

        fn id(&self) -> &u32 {
            &self.id
        }

        fn label(&self) -> &String {
            &self.label
        }

        fn anchor(&self) -> &Point {
            &self.anchor
        }
        fn pivot(&self) -> &Point {
            &self.pivot
        }
        fn translate(&self) -> &Point {
            &self.translate
        }
        fn scale(&self) -> &Point {
            &self.scale
        }
        fn rotation(&self) -> &f64 {
            &self.rotation
        }
        fn skew(&self) -> &Point {
            &self.skew
        }

        #[inline]
        fn set_anchor(&mut self, x: f64, y: f64) {
            self.anchor.x = x;
            self.anchor.y = y;
            self._update_id += 1;
        }
        #[inline]
        fn set_pivot(&mut self, x: f64, y: f64) {
            self.pivot.x = x;
            self.pivot.y = y;
            self._update_id += 1;
        }
        #[inline]
        fn set_translate(&mut self, x: f64, y: f64) {
            self.translate.x = x;
            self.translate.y = y;
            self._update_id += 1;
        }
        #[inline]
        fn set_x(&mut self, x: f64) {
            self.translate.x = x;
            self._update_id += 1;
        }
        #[inline]
        fn set_y(&mut self, y: f64) {
            self.translate.y = y;
            self._update_id += 1;
        }
        #[inline]
        fn set_scale(&mut self, x: f64, y: f64) {
            self.scale.x = x;
            self.scale.y = y;
            self._update_id += 1;
        }
        #[inline]
        fn set_scale_x(&mut self, x: f64) {
            self.scale.x = x;
            self._update_id += 1;
        }
        #[inline]
        fn set_scale_y(&mut self, y: f64) {
            self.scale.y = y;
            self._update_id += 1;
        }
        #[inline]
        fn set_rotation(&mut self, radian: f64) {
            self.rotation = radian;
            self._update_id += 1;
        }
        #[inline]
        fn set_skew(&mut self, x: f64, y: f64) {
            self.skew.x = x;
            self.skew.y = y;
            self._update_id += 1;
        }
        #[inline]
        fn set_skew_x(&mut self, x: f64) {
            self.skew.x = x;
            self._update_id += 1;
        }
        #[inline]
        fn set_skew_y(&mut self, y: f64) {
            self.skew.y = y;
            self._update_id += 1;
        }

        fn transform(&self) -> &Transform {
            &self.transform
        }
        fn global_transform(&self) -> &Transform {
            &self.global_transform
        }
        fn children(&self) -> &Vec<Arc<RwLock<dyn Node>>> {
            &self.children
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn get_child(&self, index: usize) -> Option<Arc<RwLock<dyn Node>>> {
            if let Some(child) = self.children.get(index) {
                return Some(child.clone());
            }
            None
        }

        #[inline]
        fn add_child(&mut self, child: Arc<RwLock<dyn Node>>) {
            self.children.push(child);
        }

        #[inline]
        fn insert_child(&mut self, index: usize, child: Arc<RwLock<dyn Node>>) {
            self.children.insert(index, child);
        }

        #[inline]
        fn insert_child_before(
            &mut self,
            before_child: Arc<RwLock<dyn Node>>,
            child: Arc<RwLock<dyn Node>>,
        ) {
            let index = self.children.iter().position(|item| {
                let l = item.read();
                let r = child.read();
                *l == *r
            });
            if index.is_none() {
                warn!("Cannot insert child before another one because the another child does not present in current children.");
            }
            self.children.insert(index.unwrap_or(0), child);
        }

        #[inline]
        fn remove_child(&mut self, child: Arc<RwLock<dyn Node>>) -> Option<Arc<RwLock<dyn Node>>> {
            if let Some(index) = self.children.iter().position(|item| {
                let l = item.read();
                let r = child.read();
                *l == *r
            }) {
                return Some(self.children.remove(index));
            }
            None
        }

        #[inline]
        fn remove_child_at(&mut self, index: usize) -> Option<Arc<RwLock<dyn Node>>> {
            if index < self.children.len() {
                return Some(self.children.remove(index));
            }
            None
        }

        #[inline]
        fn move_to(&mut self, x: f64, y: f64) {
            self.set_translate(x, y);
        }

        #[inline]
        fn update_transform(
            &mut self,
            parent_transform: &Transform,
            surface_size: &SurfaceSize,
            force: bool,
        ) {
            if force || self._update_id != self._current_update_id {
                let x = self.translate.x;
                let y = self.translate.y;
                let rotation = self.rotation;
                let scale_x = self.scale.x;
                let scale_y = self.scale.y;
                let skew_x = self.skew.x;
                let skew_y = self.skew.y;
                let pivot_x = self.pivot.x;
                let pivot_y = self.pivot.y;

                let a = (rotation + skew_y).cos() * scale_x;
                let b = (rotation + skew_y).sin() * scale_x;
                let c = -(rotation - skew_x).sin() * scale_y;
                let d = (rotation - skew_x).cos() * scale_y;
                let tx = x - ((pivot_x * a) + (pivot_y * c));
                let ty = y - ((pivot_x * b) + (pivot_y * d));

                let (logical_width, logical_height) = surface_size.logical_size();
                let scale_factor = surface_size.scale_factor();

                // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
                let tx = (x as f64 * scale_factor) / (logical_width * scale_factor) * 2.;
                let ty = (y as f64 * scale_factor) / (logical_height * scale_factor) * 2.;

                self.transform.a = a;
                self.transform.b = b;
                self.transform.c = c;
                self.transform.d = d;
                self.transform.tx = tx;
                self.transform.ty = ty;

                // refresh global transform matrix
                let mut global_transform = parent_transform.clone();
                global_transform.multiply(self.transform);
                self.global_transform = global_transform;

                self._current_update_id = self._update_id;
            }
        }
    };

    let renderable_impls = quote! {
        fn try_as_renderable(&self) -> Option<&dyn Renderable> {
            Some(self)
        }

        fn try_as_renderable_mut(&mut self) -> Option<&mut dyn Renderable> {
            Some(self)
        }
    };

    if renderable {
        quote! {
            impl Node for #struct_name {
                #base
                #renderable_impls
            }

            unsafe impl Send for #struct_name {}
        }
    } else {
        quote! {
            impl Node for #struct_name {
                #base
            }

            unsafe impl Send for #struct_name {}
        }
    }
}
