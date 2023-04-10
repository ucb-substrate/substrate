use itertools::{Either, Itertools};
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Error, Field, Fields, Ident, ItemStruct};

pub(crate) fn derive_interface_inner(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemStruct);

    let ident = item.ident;
    let parent_ident = format_ident!("{}Parent", ident);
    let inputs_ident = format_ident!("{}Inputs", ident);
    let outputs_ident = format_ident!("{}Outputs", ident);

    let generics = &item.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let ty_generic_idents: Vec<&Ident> = generics.type_params().map(|param| &param.ident).collect();

    let fields: Vec<(Field, bool)> = match item.fields {
        Fields::Named(named) => {
            match named
                .named
                .into_iter()
                .map(|field| {
                    let is_input = field.attrs.iter().any(|attr| attr.path.is_ident("input"));
                    let is_output = field.attrs.iter().any(|attr| attr.path.is_ident("output"));
                    if is_input ^ is_output {
                        Ok((field, is_input))
                    } else {
                        Err(Error::new(
                            field.span(),
                            "field should be annotated with exactly one of `input` or `output`",
                        ))
                    }
                })
                .collect()
            {
                Ok(vec) => vec,
                Err(err) => {
                    return err.to_compile_error().into();
                }
            }
        }
        _ => {
            return Error::new(item.fields.span(), "expected struct with named fields")
                .to_compile_error()
                .into();
        }
    };

    let (inputs, outputs): (Vec<_>, Vec<_>) =
        fields
            .into_iter()
            .partition_map(|(field, is_input)| match is_input {
                true => Either::Left(field),
                false => Either::Right(field),
            });

    // Import path for substrate.
    let substrate = match crate_name("substrate").expect("substrate is present in `Cargo.toml`") {
        FoundCrate::Itself => quote!(crate),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
    };

    // Useful paths.
    let arcstr = quote!(#substrate::deps::arcstr);
    #[allow(non_snake_case)]
    let ArcStr = quote!(#arcstr::ArcStr);
    #[allow(non_snake_case)]
    let Port = quote!(#substrate::digital::module::Port);
    #[allow(non_snake_case)]
    let Direction = quote!(#substrate::digital::module::Direction);
    #[allow(non_snake_case)]
    let Wire = quote!(#substrate::digital::wire::Wire);
    #[allow(non_snake_case)]
    let Interface = quote!(#substrate::digital::Interface);
    #[allow(non_snake_case)]
    let DigitalCtx = quote!(#substrate::digital::context::DigitalCtx);
    #[allow(non_snake_case)]
    let PhantomData = quote!(::std::marker::PhantomData);
    #[allow(non_snake_case)]
    let Instance = quote!(#substrate::digital::module::Instance);
    #[allow(non_snake_case)]
    let ModulePort = quote!(#substrate::digital::ModulePort);
    #[allow(non_snake_case)]
    let WireKey = quote!(#substrate::digital::wire::WireKey);
    #[allow(non_snake_case)]
    let ParentModulePort = quote!(#substrate::digital::ParentModulePort);

    // Struct field identifiers.
    let input_idents: Vec<&Ident> = inputs
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect();
    let output_idents: Vec<&Ident> = outputs
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect();

    // Code for initializing the interface input.
    let input_port_inits = input_idents.iter().map(
        |&ident| {

        quote!( #ident: ctx.port(#arcstr::literal!(stringify!(#ident)), self.#ident.clone()) )
        }
        );

    // Code for initializing the output of the parent interface.
    let output_parent_inits = output_idents
        .iter()
        .map(|&ident| quote!(#ident: ctx.instance_output(self.#ident.clone())));

    // Port declarations
    let input_port_decls = input_idents.iter().map(|&ident| {
        quote!( #Port {
            direction: #Direction::Input,
            name: #arcstr::literal!(stringify!(#ident))
        })
    });
    let output_port_decls = output_idents.iter().map(|&ident| {
        quote!( #Port {
            direction: #Direction::Output,
            name: #arcstr::literal!(stringify!(#ident))
        })
    });

    // Field declarations for input and output structs.
    let input_field_decls = inputs.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        quote!(#ident: #Wire<#ty>)
    });
    let output_field_decls: Vec<proc_macro2::TokenStream> = outputs
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            let ty = &field.ty;

            quote!(#ident: #Wire<#ty>)
        })
        .collect();

    // Field declarations with wrapping `Option`s for building purposes.
    let input_field_decls_option = inputs.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        quote!(#ident: Option<#Wire<#ty>>)
    });

    // Match arms for converting a `str` to its corresponding port.
    let input_match_strs = input_idents
        .iter()
        .map(|&ident| quote!(stringify!(#ident) => self.#ident._inner()));
    let output_match_strs: Vec<proc_macro2::TokenStream> = output_idents
        .iter()
        .map(|&ident| quote!(stringify!(#ident) => self.#ident._inner()))
        .collect();

    // Match arms for converting a `str` to its corresponding port that is wrapped by an Option.
    let input_match_strs_option = input_idents
        .iter()
        .map(|&ident| quote!(stringify!(#ident) => self.#ident.as_ref().unwrap()._inner()));

    // Functions for setting inputs of the parent interface.
    let input_setters = inputs.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        quote!(
            pub fn #ident(mut self , #ident: #Wire<#ty>) -> Self {
                self.#ident = Some(#ident);
                self
            }
        )
    });

    // Functions for getting output wires of the parent interface.
    let output_getters = outputs.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        quote!(
            pub fn #ident(&self) -> #Wire<#ty> {
                self.#ident
            }
        )
    });

    // `param: T` mappings for defining a `new` function for the output struct.
    let output_new_params = outputs.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        quote!( #ident: #Wire<#ty> )
    });

    quote!(
        impl #impl_generics #Interface for #ident #ty_generics #where_clause {
            type Parent = #parent_ident #ty_generics;
            type Input = #inputs_ident #ty_generics;
            type Output = #outputs_ident #ty_generics;

            fn input(&self, ctx: &mut #DigitalCtx) -> Self::Input {
                Self::Input {
                    #(#input_port_inits,)*
                    phantom: #PhantomData,
                }
            }

            fn parent(&self, inst: #Instance, ctx: &mut #DigitalCtx) -> Self::Parent {
                Self::Parent {
                    #(#input_idents: None,)*
                    #(#output_parent_inits,)*
                    _inst: inst,
                }
            }

            fn ports() -> Vec<#Port> {
                vec![
                    #(#input_port_decls,)*
                    #(#output_port_decls,)*
                ]
            }
        }

        pub struct #parent_ident #generics {
            #(#input_field_decls_option,)*
            #(#output_field_decls,)*
            _inst: #Instance,
        }

        impl #impl_generics #ModulePort for #parent_ident #ty_generics #where_clause {
            fn port(&self, name: &str) -> #WireKey {
                match name {
                    #(#input_match_strs_option,)*
                    #(#output_match_strs,)*
                    _ => panic!("unexpected port"),
                }
            }
        }

        impl #impl_generics #ParentModulePort for #parent_ident #ty_generics #where_clause {
            #[inline]
            fn instance(&self) -> &#Instance {
                &self._inst
            }

            #[inline]
            fn into_instance(self) -> #Instance {
                self._inst
            }
        }

        impl #impl_generics #parent_ident #ty_generics #where_clause {
            #(#input_setters)*
            #(#output_getters)*

            pub fn name(mut self, name: impl Into<#ArcStr>) -> Self {
                self._inst.set_name(name);
                self
            }

            pub fn finish(self, ctx: &mut #DigitalCtx) {
                ctx.add_instance::<#ident #ty_generics>(self)
            }
        }

        pub struct #inputs_ident #generics {
            #(#input_field_decls,)*
            phantom: #PhantomData<(#(#ty_generic_idents),*)>,
        }

        impl #impl_generics #ModulePort for #inputs_ident #ty_generics #where_clause {
            fn port(&self, name: &str) -> #WireKey {
                match name {
                    #(#input_match_strs,)*
                    _ => panic!("unexpected port"),
                }
            }
        }

        pub struct #outputs_ident #generics {
            #(#output_field_decls,)*
            phantom: #PhantomData<(#(#ty_generic_idents),*)>,
        }


        impl #impl_generics #ModulePort for #outputs_ident #ty_generics #where_clause {
            fn port(&self, name: &str) -> #WireKey {
                match name {
                    #(#output_match_strs,)*
                    _ => panic!("unexpected port"),
                }
            }
        }

        impl #impl_generics #outputs_ident #ty_generics #where_clause {
            fn new(#(#output_new_params),*) -> Self {
                Self {
                    #(#output_idents,)*
                    phantom: #PhantomData,
                }
            }
        }
    )
    .into()
}
