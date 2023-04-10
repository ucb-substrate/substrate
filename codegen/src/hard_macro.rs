use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, Error, Fields, Ident, ItemStruct};

#[derive(Debug, FromMeta)]
struct HardMacroArgs {
    name: String,
    pdk: String,
    path_fn: Ident,
    #[darling(default)]
    spice_subckt_name: Option<String>,
    #[darling(default)]
    gds_cell_name: Option<String>,
    #[darling(default)]
    toml_fn: Option<Ident>,
}

pub(crate) fn hard_macro_inner(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemStruct);

    let args = match HardMacroArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    if input.fields != Fields::Unit {
        return Error::new(input.fields.span(), "expected unit struct")
            .to_compile_error()
            .into();
    }

    let name = args.name;
    let spice_subckt_name = args.spice_subckt_name.unwrap_or_else(|| name.clone());
    let gds_cell_name = args.gds_cell_name.unwrap_or_else(|| name.clone());
    let _pdk = args.pdk;
    let ident = input.ident.clone();
    let path_fn = args.path_fn;
    let _toml_fn = args.toml_fn;

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
    let Component = quote!(#substrate::component::Component);
    #[allow(non_snake_case)]
    let NoParams = quote!(#substrate::component::NoParams);
    #[allow(non_snake_case)]
    let Result = quote!(#substrate::error::Result);
    #[allow(non_snake_case)]
    let SubstrateCtx = quote!(#substrate::data::SubstrateCtx);
    #[allow(non_snake_case)]
    let SchematicCtx = quote!(#substrate::schematic::context::SchematicCtx);
    #[allow(non_snake_case)]
    let View = quote!(#substrate::component::View);
    #[allow(non_snake_case)]
    let ErrorSource = quote!(#substrate::error::ErrorSource);
    #[allow(non_snake_case)]
    let Error = quote!(#substrate::component::error::Error);
    #[allow(non_snake_case)]
    let LayoutCtx = quote!(#substrate::layout::context::LayoutCtx);

    quote!(
        #input

        impl #Component for #ident {
            type Params = #NoParams;

            fn new(_params: &Self::Params, ctx: &#SubstrateCtx) -> #Result<Self> {
                Ok(Self)
            }

            fn name(&self) -> #ArcStr {
                #arcstr::literal!(#name)
            }

            fn schematic(&self, ctx: &mut #SchematicCtx) -> #Result<()> {
                let spice_path = #path_fn(ctx.inner(), #name, #View::Schematic).ok_or(#ErrorSource::Component(
                    #Error::ViewUnsupported(#View::Schematic),
                ))?;
                ctx.import_spice(#spice_subckt_name, spice_path)?;
                Ok(())
            }

            fn layout(&self, ctx: &mut #LayoutCtx) -> #Result<()> {
                let layout_path = #path_fn(ctx.inner(), #name, #View::Layout).ok_or(#ErrorSource::Component(
                    #Error::ViewUnsupported(#View::Layout),
                ))?;
                ctx.from_gds_flattened(layout_path, #gds_cell_name)?;
                Ok(())
            }

        }
    )
    .into()
}
