#![allow(missing_docs, clippy::missing_docs_in_private_items)]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;
use syn::ExprLit;
use syn::Ident;
use syn::ItemFn;
use syn::Lit;
use syn::LitStr;
use syn::Meta;
use syn::Token;
use syn::bracketed;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;

struct CommandAttrs {
    aliases: Vec<String>,
    restriction_level: Option<syn::Path>,
    category: syn::Path,
}

impl Parse for CommandAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut aliases = Vec::new();
        let mut restriction_level = None;
        let mut category = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;

            if ident == "aliases" {
                let content;
                bracketed!(content in input);
                while !content.is_empty() {
                    let lit: LitStr = content.parse()?;
                    aliases.push(lit.value());
                    if !content.is_empty() {
                        let _: Token![,] = content.parse()?;
                    }
                }
            } else if ident == "restriction_level" {
                restriction_level = Some(input.parse::<syn::Path>()?);
            } else if ident == "category" {
                category = Some(input.parse::<syn::Path>()?);
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    "expected `aliases`, `restriction_level`, or `category`",
                ));
            }

            if !input.is_empty() {
                let _: Token![,] = input.parse()?;
            }
        }

        let category = category
            .ok_or_else(|| syn::Error::new(input.span(), "missing required argument `category`"))?;

        Ok(CommandAttrs {
            aliases,
            restriction_level,
            category,
        })
    }
}

/// Extract doc comment lines from a function's attributes and join them.
fn doc_comment_description(func: &ItemFn) -> String {
    func.attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| {
            if attr.path().is_ident("doc")
                && let Meta::NameValue(nv) = &attr.meta
                && let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
            {
                let trimmed = s.value().trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Attribute macro for defining a command.
///
/// The `///` doc comment on the function becomes the command description.
/// Command usage is auto-generated from each parameter type's
/// [`ParseArgument::label()`].
///
/// # Supported attributes
///
/// - `aliases = ["a", "b"]` — alternative invocation names.
///
/// # Example
///
/// ```ignore
/// /// Ping the bot.
/// #[command]
/// pub async fn ping(ctx: &Context<'_>) -> anyhow::Result<()> {
///     ctx.reply("pong").await?;
///     Ok(())
/// }
/// ```
///
/// The macro generates a zero-sized struct `ping_command` that implements
/// [`Command`](rue_client::command::Command).
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let attrs = parse_macro_input!(attr as CommandAttrs);

    let func_name = &func.sig.ident;
    let cmd_name = func_name.to_string();
    let struct_name = syn::Ident::new(&format!("{func_name}_command"), func_name.span());

    let description = doc_comment_description(&func);
    let aliases = &attrs.aliases;
    let category = &attrs.category;
    let restriction_level = if let Some(path) = &attrs.restriction_level {
        quote! { #path }
    } else {
        quote! { Default::default() }
    };

    // Collect parameter names and types (skipping the first `&Context` param).
    let params: Vec<_> = func.sig.inputs.iter().skip(1).collect();

    let param_names: Vec<_> = params
        .iter()
        .filter_map(|p| {
            if let syn::FnArg::Typed(pt) = p
                && let syn::Pat::Ident(pi) = &*pt.pat
            {
                Some(&pi.ident)
            } else {
                None
            }
        })
        .collect();

    let param_types: Vec<_> = params
        .iter()
        .filter_map(|p| {
            if let syn::FnArg::Typed(pt) = p {
                Some(&pt.ty)
            } else {
                None
            }
        })
        .collect();

    // Auto-generate usage from ParseArgument::label() on each param type.
    let usage_tokens = if param_types.is_empty() {
        quote! { "" }
    } else {
        quote! {
            Box::leak(
                vec![#(<#param_types as crate::command::arguments::ParseArgument>::label()),*]
                    .into_iter()
                    .map(|s| format!("<{s}>"))
                    .collect::<Vec<_>>()
                    .join(" ")
                    .into_boxed_str()
            ) as &str
        }
    };

    let expanded = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        pub struct #struct_name;

        #[async_trait::async_trait]
        impl crate::command::traits::Command for #struct_name {
            fn metadata(&self) -> &'static crate::command::traits::CommandMetadata {
                use std::sync::OnceLock;
                static META: OnceLock<crate::command::traits::CommandMetadata> = OnceLock::new();
                META.get_or_init(|| crate::command::traits::CommandMetadata {
                    name: #cmd_name,
                    aliases: &[#(#aliases),*],
                    description: #description,
                    usage: #usage_tokens,
                    category: #category,
                    restriction_level: #restriction_level,
                })
            }

            async fn execute(
                &self,
                ctx: &mut crate::command::context::Context<'_>,
            ) -> anyhow::Result<()> {
                #(
                    let #param_names =
                        <#param_types as crate::command::arguments::ParseArgument>::parse(ctx)?;
                )*
                #func_name(ctx, #(#param_names),*).await
            }
        }

        #func
    };

    TokenStream::from(expanded)
}
