use std::collections::HashMap;
use syn::parse::Parse;

pub struct FromAttributes {
	pub attr: Option<String>,
	pub named_arrs: HashMap<String, String>,
}
#[allow(dead_code)]
pub struct NamedAttribute {
	pub name: syn::Ident,
	pub eq_token: syn::Token![=],
	pub value: syn::LitStr,
}
pub struct TParse3<T0, T1, T2>(pub T0, pub T1, pub T2);


impl<T0: Parse, T1: Parse, T2: Parse> Parse for TParse3<T0, T1, T2> {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let content;
		let _ = syn::parenthesized!(content in input);
		let input = &content;
		Ok(TParse3(input.parse()?, input.parse()?, input.parse()?))
	}
}

impl Parse for FromAttributes {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let content;
		let _ = syn::parenthesized!(content in input);
		let input = &content;
		let (attr, named_arrs_punct): (_, syn::punctuated::Punctuated<NamedAttribute, syn::Token![,]>) = if input.peek(syn::LitStr) {
			let attr: syn::LitStr = input.parse()?;
			(Some(attr.value()), if input.peek(syn::Token![,]) {
				let _: syn::Token![,] = input.parse()?;
				syn::punctuated::Punctuated::parse_terminated(input)?
			} else {
				syn::punctuated::Punctuated::new()
			})
		} else {
			(None, syn::punctuated::Punctuated::parse_terminated(input)?)
		};
		let mut named_arrs = HashMap::new();
		for attr in named_arrs_punct {
			named_arrs.insert(attr.name.to_string(), attr.value.value());
		}
		Ok(FromAttributes {
			attr,
			named_arrs,
		})
	}
}

impl Parse for NamedAttribute {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		Ok(NamedAttribute {
			name: input.parse()?,
			eq_token: input.parse()?,
			value: input.parse()?,
		})
	}
}
