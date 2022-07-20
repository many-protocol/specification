use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;

fn test_case_impl(_attr: TokenStream, item: TokenStream) -> Result<TokenStream, syn::Error> {
    let span = item.span();
    let func: syn::ItemFn = syn::parse2(item)
        .map_err(|_| syn::Error::new(span, "`test_case` only applies to functions.".to_string()))?;

    let fn_ident = func.sig.ident.clone();
    let fn_name = fn_ident.to_string();

    let test_ident = Ident::new(
        &format!("TESTCASE_{}_{}", fn_name, span.start().line),
        Span::call_site(),
    );

    let is_async = func.sig.asyncness.is_some();

    if is_async {
        Ok(quote! {
            #func

            #[linkme::distributed_slice(crate::tests::TEST_CASES)]
            static #test_ident: crate::tests::TestCase = crate::tests::TestCase::new(
                #fn_name,
                file!(),
                crate::tests::TestCaseFn(|c| Box::pin(std::panic::AssertUnwindSafe(#fn_ident (c)))),
            );
        })
    } else {
        Ok(quote! {
            #func

            #[linkme::distributed_slice(crate::tests::TEST_CASES)]
            static #test_ident: crate::tests::TestCase = crate::tests::TestCase::new(
                #fn_name,
                file!(),
                crate::tests::TestCaseFn(|c| {
                    async fn do_it(c: crate::tests::TestConfig) -> crate::tests::TestCaseResult {
                        #fn_ident (c)
                    }
                    Box::pin(std::panic::AssertUnwindSafe(do_t(c)))
                }),
            );
        })
    }
}

#[proc_macro_attribute]
pub fn test_case(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    test_case_impl(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
