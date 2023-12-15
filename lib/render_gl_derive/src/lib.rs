#![recursion_limit = "128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

#[proc_macro_derive(ComponentId)]
pub fn component_id_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let type_name = &ast.ident;
    let name = &ast.ident.to_string();
    quote! {
        impl Component for #type_name {
            fn get_id() -> ComponentID {
                #name
            }
        }
    }
    .into()
}

#[proc_macro_derive(VertexAttribPointers, attributes(location, divisor))]
pub fn vertex_attrib_pointers_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse(input).unwrap();
    generate_impl(&ast)
}

/* Example gen code:
 * // pos
 * unsafe {
 *     Cvec3::vertex_attrib_pointer(stride, 0, 0);
 * }
 *
 * // clr
 * unsafe {
 *     Cvec3::vertex_attrib_pointer(stride, 1, std::mem::size_of::<Cvec3>());
 * }
 */

fn generate_impl(ast: &syn::DeriveInput) -> proc_macro::TokenStream {
    let name = &ast.ident;
    let vertex_attrib_pointer_calls = generate_vertex_attrib_pointer_calls(&ast.data);
    quote! {
        impl Vertex for #name {
            /// Enable and configure a vertex attribute for each field in this vertex
            fn setup_vertex_attrib_pointers(gl: &Gl) {
                let stride = std::mem::size_of::<Self>();

                let offset = 0;

                #(#vertex_attrib_pointer_calls)*
            }
        }
    }
    .into()
}

fn generate_vertex_attrib_pointer_calls(body: &syn::Data) -> Vec<proc_macro2::TokenStream> {
    match body {
        syn::Data::Struct(syn::DataStruct { fields: ref s, .. }) => {
            s.iter().map(generate_vertex_attrib_pointer_call).collect()
        }
        _ => {
            panic!("Expected struct");
        }
    }
}

fn generate_vertex_attrib_pointer_call(field: &syn::Field) -> proc_macro2::TokenStream {
    let name = field.ident.as_ref().unwrap();
    let location = field
        .attrs
        .iter()
        .find_map(|a: &syn::Attribute| match a.parse_meta() {
            Ok(syn::Meta::NameValue(syn::MetaNameValue {
                path,
                eq_token: _,
                lit: syn::Lit::Int(ref lit),
            })) => {
                if path.is_ident("location") {
                    lit.base10_parse::<usize>().ok()
                } else {
                    None
                }
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Field {:?} is missing #[location = ?] attribute", name));
    let divisor = field
        .attrs
        .iter()
        .find_map(|a: &syn::Attribute| match a.parse_meta() {
            Ok(syn::Meta::NameValue(syn::MetaNameValue {
                path,
                eq_token: _,
                lit: syn::Lit::Int(ref lit),
            })) => {
                if path.is_ident("divisor") {
                    lit.base10_parse::<usize>().ok()
                } else {
                    None
                }
            }
            _ => None,
        })
        .unwrap_or(0);
    let field_type = &field.ty;
    quote! {
        let location = #location;
        unsafe {
            #field_type::vertex_attrib_pointer(gl, stride, location, offset);
            gl.VertexAttribDivisor(location as gl::types::GLuint, #divisor as gl::types::GLuint);
        }
        let offset = offset + std::mem::size_of::<#field_type>();
    }
    .into()
}
