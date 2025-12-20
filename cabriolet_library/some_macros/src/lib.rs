extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn;
use syn::parse::{Parse, ParseStream};
use syn::{Block, Expr, Stmt, Type, parse_macro_input};
// use syn::parse::{Parse, ParseStream};

#[proc_macro]
pub fn make_answer(_item: TokenStream) -> TokenStream {
    "fn answer() -> u32 { 42 }".parse().unwrap()
}

fn comma_separate<T: Iterator<Item = proc_macro2::TokenStream>>(ts: T) -> proc_macro2::TokenStream {
    ts.fold(
        proc_macro2::TokenStream::new(),
        |acc: proc_macro2::TokenStream,
         token: proc_macro2::TokenStream|
         -> proc_macro2::TokenStream {
            if acc.is_empty() {
                token
            } else {
                let ba: proc_macro2::TokenStream = acc.into();
                let bt: proc_macro2::TokenStream = token.into();
                quote! {#ba, #bt}
            }
        },
    )
}

// Returns whether the function call is a specific function.
fn is_call_to(call: &syn::ExprCall, path: &str) -> bool {
    if let syn::Expr::Path(path_expr) = &*call.func {
        let mut path_str = quote::quote! {#path_expr}.to_string();
        path_str.retain(|c| !c.is_whitespace());
        return path_str == path;
    } else {
        false
    }
}

// Returns whether the Type is a specific type
fn is_type(the_type: &syn::Type, path: &str) -> bool {
    if let syn::Type::Path(path_expr) = &*the_type {
        let mut path_str = quote::quote! {#path_expr}.to_string();
        path_str.retain(|c| !c.is_whitespace());
        return path_str == path;
    } else {
        false
    }
}

fn expand_expr(input: &Expr, label_type: &Type) -> TokenStream {
    match input {
        Expr::Call(call_expr) => {
            let func = &call_expr.func;
            let args = comma_separate(call_expr.args.iter().map(
                |arg: &syn::Expr| -> proc_macro2::TokenStream {
                    expand_expr(arg, label_type).into()
                },
            ));

            // If it's a call to an unwrap_labeled "function"
            if is_call_to(call_expr, "unwrap_labeled") {
                quote::quote! {
                    ::secrets_structs::Labeled::unwrap_checked::<#label_type>(&mut #args).await
                }
                .into()
            } else {
                quote::quote! {
                    #func(#args)
                }
                .into()
            }
        }
        Expr::Binary(binary_expr) => {
            let mut new_binary_expr = binary_expr.clone();
            new_binary_expr.left =
                syn::parse(expand_expr(&binary_expr.left, label_type).into()).unwrap();
            new_binary_expr.right =
                syn::parse(expand_expr(&binary_expr.right, label_type).into()).unwrap();
            new_binary_expr.into_token_stream().into()
        }
        Expr::Lit(lit_expr) => match &lit_expr.lit {
            _ => lit_expr.into_token_stream().into(),
        },
        _ => input.into_token_stream().into(),
    }
}

fn expand_block(input: &syn::Block, label_type: &Type) -> TokenStream {
    let token_streams: proc_macro2::TokenStream = input
        .stmts
        .iter()
        .map(|stmt| -> proc_macro2::TokenStream {
            match stmt {
                Stmt::Local(local_expr) => match &local_expr.init {
                    Some(local_init) => {
                        let mut new_local_expr = local_expr.clone();
                        let mut new_init = local_init.clone();
                        new_init.expr =
                            syn::parse(expand_expr(&local_init.expr, label_type).into()).unwrap();
                        new_init.diverge = match &local_init.diverge {
                            Some((else_, diverge_expr)) => Some((
                                *else_,
                                syn::parse(expand_expr(&diverge_expr, label_type).into()).unwrap(),
                            )),
                            None => None,
                        };
                        new_local_expr.init = Some(new_init);
                        new_local_expr.into_token_stream().into()
                    }
                    None => local_expr.into_token_stream().into(),
                },
                Stmt::Item(item) => item.into_token_stream().into(),
                Stmt::Expr(expr, maybe_token) => {
                    let mut expr_stream: proc_macro2::TokenStream =
                        expand_expr(&expr, label_type).into();
                    match maybe_token {
                        Some(semi) => {
                            expr_stream.extend(semi.into_token_stream());
                        }
                        None => {}
                    }
                    expr_stream
                }
                Stmt::Macro(macro_expr) => macro_expr.into_token_stream().into(),
            }
        })
        .collect();
    let stream: proc_macro2::TokenStream = proc_macro2::TokenStream::from_iter(token_streams);
    stream.into()
}

impl Parse for LabeledBlock {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ty: Type = input.parse().unwrap_or_else(|_| panic!("not a type"));
        let mut inputs = syn::punctuated::Punctuated::new();
        let _or1_token: syn::Token![|] = input.parse().unwrap();
        loop {
            if input.peek(syn::Token![|]) {
                break;
            }
            let ele: syn::Ident = input.parse()?;
            inputs.push_value(ele);
            if input.peek(syn::Token![|]) {
                break;
            }
            let punct: syn::Token![,] = input.parse()?;
            inputs.push_punct(punct);
        }
        let _or2_token: syn::Token![|] = input.parse().unwrap();
        let blk: Block = input.parse().unwrap();
        Ok(LabeledBlock {
            ty,
            inputs,
            blk,
        })
    }
}

struct LabeledBlock {
    ty: Type,
    inputs: syn::punctuated::Punctuated<syn::Ident, syn::Token![,]>,
    blk: Block,
}

#[proc_macro]
pub fn labeled_block(item: TokenStream) -> TokenStream {
    let LabeledBlock {
        ty,
        inputs,
        blk,
    } = parse_macro_input!(item as LabeledBlock);

    let stream = proc_macro2::TokenStream::from(expand_block(&blk, &ty));
    if is_type(&ty, "LabelNonIdem") {
        let mut clones: Vec<_> = Vec::new();
        for ident in inputs {
            clones.push(quote!(let mut #ident = #ident.clone();));
        }
        let clones_tokens = proc_macro2::TokenStream::from_iter(clones);
        quote!(
            {
                #clones_tokens
                let tmp: ::secrets_structs::Labeled<_, #ty> = ::secrets_structs::Labeled::new({
                    #stream
                });
                tmp
            }
        )
        .into()
    } else {
        // Timely
        let mut clones: Vec<_> = Vec::new();
        for ident in inputs {
            clones.push(quote!(let mut #ident = #ident.clone();));
        }
        let clones_tokens = proc_macro2::TokenStream::from_iter(clones);
        quote!(
            {
                #clones_tokens
                let nc: ::secrets_structs::TimelyClosure<_> = Arc::new(move || {
                    #clones_tokens
                    async move {
                        #stream
                    }.boxed()
                });
                let tmp: ::secrets_structs::Labeled<_, #ty> = ::secrets_structs::Labeled::new(nc);
                tmp
            }
        )
        .into()
    }
}
