use std::collections::HashMap;
use proc_macro2::Span;
use syn::parse::Parse;

#[derive(Clone)]
pub struct FromAttribute {
	pub attr: Option<String>,
	pub named_arrs: HashMap<String, (String, Span)>,
}

pub struct RelationAttribute {
	pub fk: String,
}

#[allow(dead_code)]
pub struct NamedAttribute {
	pub name: syn::Ident,
	pub eq_token: syn::Token![=],
	pub value: syn::LitStr,
}

impl Parse for FromAttribute {
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
			named_arrs.insert(attr.name.to_string(), (attr.value.value(), attr.value.span()));
		}
		Ok(FromAttribute {
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


impl Parse for RelationAttribute {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let content;
		let _ = syn::parenthesized!(content in input);
		let fk: syn::LitStr = content.parse()?;
		Ok(RelationAttribute {
			fk: fk.value(),
		})
	}
}
