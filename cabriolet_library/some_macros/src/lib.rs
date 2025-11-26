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
                    ::secrets_structs::Labeled::unwrap_checked::<#label_type>(#args)
                }
                .into()
            } else if is_call_to(call_expr, "wrap_labeled") {
                quote::quote! {
                    ::secrets_structs::Labeled::<_, #label_type>::new(#args)
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
            syn::Lit::Int(lit_int) => {
                let mut new_lit_expr = lit_expr.clone();
                new_lit_expr.lit = syn::Lit::Int(syn::LitInt::new("727", lit_int.span()));
                new_lit_expr.into_token_stream().into()
            }
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
        let blk: Block = input.parse().unwrap();
        Ok(LabeledBlock { ty, blk })
    }
}

struct LabeledBlock {
    ty: Type,
    blk: Block,
}

#[proc_macro]
pub fn labeled_block(item: TokenStream) -> TokenStream {
    let LabeledBlock { ty, blk } = parse_macro_input!(item as LabeledBlock);

    let stream = proc_macro2::TokenStream::from(expand_block(&blk, &ty));
    quote!(
        {
            let tmp: Labeled<_, #ty> = {
                #stream
            };
            tmp
        }
    )
    .into()
}
