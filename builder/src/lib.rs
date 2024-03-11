use proc_macro::TokenStream;
use quote::quote;
use syn::{self, spanned::Spanned, Result};
#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let st = syn::parse_macro_input!(input as syn::DeriveInput);
    match do_expand(&st) {
        Ok(token_stream) => token_stream.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn do_expand(st: &syn::DeriveInput) -> Result<TokenStream> {
    // 需要使用 extra-traits
    // eprintln!("{:?}", st.data);
    let struct_name_literal = st.ident.to_string();
    let builder_name_literal = format!("{}Builder", struct_name_literal);

    // span 信息主要用于发生编译错误时，编译器给用户指示出错误的位置
    let builder_ident = syn::Ident::new(&builder_name_literal, st.span());

    // 模板代码(quote)中不可以使用`.`来访问结构体成员，所以要在模板代码外面将标识符放到一个独立的变量中
    let struct_ident = &st.ident;

    let fields = get_fields_from_derive_input(st)?;
    // eprintln!("fields:\n {:?}", fields);

    // proc_macro2 需要使用这个库
    let builder_struct_fields_def = generate_builder_struct_fields_def(fields)?;
    let builder_struct_factory_init_clauses = generate_builder_struct_factory_init_clauses(fields)?;

    let setter_functions = generate_setter_functions(fields)?;

    let ret = quote!(
        pub struct #builder_ident{
                #builder_struct_fields_def
        }
        impl #struct_ident {
            pub fn builder() -> #builder_ident{
                    #builder_ident{
                        #(#builder_struct_factory_init_clauses),*
                    }

            }
        }
        impl #builder_ident{
            #setter_functions
        }
    );
    Ok(ret.into())
}

type StructFields = syn::punctuated::Punctuated<syn::Field, syn::Token!(,)>;

fn get_fields_from_derive_input(d: &syn::DeriveInput) -> syn::Result<&StructFields> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = d.data
    {
        return Ok(named);
    }
    Err(syn::Error::new_spanned(
        d,
        "Must define on a Struct, not Enum".to_string(),
    ))
}

fn generate_builder_struct_fields_def(fields: &StructFields) -> Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    let token_stream = quote! {
        #(#idents: std::option::Option<#types>),*
    };
    Ok(token_stream)
}

fn generate_builder_struct_factory_init_clauses(
    fields: &StructFields,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let init_clauses: Vec<_> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            quote! {
                #ident: std::option::Option::None
            }
        })
        .collect();

    Ok(init_clauses)
}

fn generate_setter_functions(fields: &StructFields) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    // 创建一个空的TokenStream
    let mut final_tokenstream = proc_macro2::TokenStream::new();

    for (ident, type_) in idents.iter().zip(types.iter()) {
        let tokenstream_piece = quote! {
            fn #ident(&mut self, #ident: #type_) -> &mut Self {
                self.#ident = std::option::Option::Some(#ident);
                self
            }
        };
        // 不断追加新的TokenStream片段到一个公共的TokenStream上
        final_tokenstream.extend(tokenstream_piece);
    }

    Ok(final_tokenstream)
}
