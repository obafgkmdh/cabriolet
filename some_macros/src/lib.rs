extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, ItemFn, Stmt, Expr};
use syn;
// use syn::parse::{Parse, ParseStream};

#[proc_macro]
pub fn make_answer(_item: TokenStream) -> TokenStream {
    "fn answer() -> u32 { 42 }".parse().unwrap()
}

fn expand_expr(input: &Expr) -> TokenStream {
    match input {
        Expr::Binary(binary_expr) => {
            let mut new_binary_expr = binary_expr.clone();
            new_binary_expr.left = syn::parse(expand_expr(&binary_expr.left).into()).unwrap();
            new_binary_expr.right = syn::parse(expand_expr(&binary_expr.right).into()).unwrap();
            new_binary_expr.into_token_stream().into()
        },
        Expr::Lit(lit_expr) => match &lit_expr.lit {
            syn::Lit::Int(lit_int) => {
                let mut new_lit_expr = lit_expr.clone();
                new_lit_expr.lit = syn::Lit::Int(syn::LitInt::new("727", lit_int.span()));
                new_lit_expr.into_token_stream().into()
            },
            _ => lit_expr.into_token_stream().into()
        }
        _ => input.into_token_stream().into()
    }
}

fn expand_block(input: &syn::Block) -> TokenStream {
    let token_streams: proc_macro2::TokenStream = input.stmts.iter().map(|stmt| -> proc_macro2::TokenStream {
        match stmt {
            Stmt::Local(local_expr) => match &local_expr.init {
                Some(local_init) => {
                    let mut new_local_expr = local_expr.clone();
                    let mut new_init = local_init.clone();
                    new_init.expr = syn::parse(expand_expr(&local_init.expr).into()).unwrap();
                    new_init.diverge = match &local_init.diverge {
                        Some((else_, diverge_expr)) => {
                            Some((*else_, syn::parse(expand_expr(&diverge_expr).into()).unwrap()))
                        },
                        None => None
                    };
                    new_local_expr.init = Some(new_init);
                    new_local_expr.into_token_stream().into()
                },
                None => local_expr.into_token_stream().into(),
            },
            Stmt::Item(item) => {
                item.into_token_stream().into()
            },
            Stmt::Expr(expr, maybe_token) => {
                let mut expr_stream: proc_macro2::TokenStream = expand_expr(&expr).into();
                match maybe_token {
                    Some(semi) => {
                        expr_stream.extend(semi.into_token_stream());
                    },
                    None => {}
                }
                expr_stream
            },
            Stmt::Macro(macro_expr) => {
                macro_expr.into_token_stream().into()
            },
        }
    }).collect();
    let stream: proc_macro2::TokenStream = proc_macro2::TokenStream::from_iter(token_streams);
    stream.into()
}

#[proc_macro_attribute]
pub fn wysi(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let ItemFn { sig, vis, block, attrs } = input;
    let stream = proc_macro2::TokenStream::from(expand_block(&block));
    quote!(
        #(#attrs)*
        #vis #sig {
            #stream
        }
    ).into()
}
