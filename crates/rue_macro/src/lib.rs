#![allow(missing_docs, clippy::missing_docs_in_private_items)]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, ExprLit, ItemFn, Lit, LitStr, Meta, parse_macro_input};

struct CommandAttrs {
    aliases: Vec<String>,
}

impl syn::parse::Parse for CommandAttrs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self {
                aliases: Vec::new(),
            });
        }

        let mut aliases = Vec::new();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            if ident == "aliases" {
                let _: syn::Token![=] = input.parse()?;
                let content;
                syn::bracketed!(content in input);
                while !content.is_empty() {
                    let lit: LitStr = content.parse()?;
                    aliases.push(lit.value());
                    if !content.is_empty() {
                        let _: syn::Token![,] = content.parse()?;
                    }
                }
            } else {
                return Err(syn::Error::new(ident.span(), "expected `aliases`"));
            }

            if !input.is_empty() {
                let _: syn::Token![,] = input.parse()?;
            }
        }

        Ok(CommandAttrs { aliases })
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
    let attrs = if attr.is_empty() {
        CommandAttrs {
            aliases: Vec::new(),
        }
    } else {
        parse_macro_input!(attr as CommandAttrs)
    };

    let func_name = &func.sig.ident;
    let cmd_name = func_name.to_string();
    let struct_name = syn::Ident::new(&format!("{func_name}_command"), func_name.span());

    let description = doc_comment_description(&func);
    let aliases = &attrs.aliases;

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
                vec![#(<#param_types as crate::command::ParseArgument>::label()),*]
                    .into_iter()
                    .map(|s| format!("<{s}>"))
                    .join(" ")
                    .into_boxed_str()
            ) as &str
        }
    };

    let expanded = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        pub struct #struct_name;

        #[async_trait::async_trait]
        impl crate::command::core::traits::Command for #struct_name {
            fn metadata(&self) -> &'static crate::command::core::traits::CommandMetadata {
                use std::sync::OnceLock;
                static META: OnceLock<crate::command::core::traits::CommandMetadata> = OnceLock::new();
                META.get_or_init(|| crate::command::core::traits::CommandMetadata {
                    name: #cmd_name,
                    aliases: &[#(#aliases),*],
                    description: #description,
                    usage: #usage_tokens,
                })
            }

            async fn execute(
                &self,
                ctx: &mut crate::command::core::context::Context<'_>,
            ) -> anyhow::Result<()> {
                #(
                    let #param_names =
                        <#param_types as crate::command::core::traits::ParseArgument>::parse(ctx)?;
                )*
                #func_name(ctx, #(#param_names),*).await
            }
        }

        #func
    };

    TokenStream::from(expanded)
}
