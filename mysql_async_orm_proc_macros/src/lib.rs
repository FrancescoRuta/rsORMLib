use quote::quote;
use db_table_parse::*;

mod db_table_parse;
mod db_table_macro;

const CRATE_NAME: &'static str = "mysql_async_orm";


fn generate_unique_ident(prefix: &str) -> syn::Ident {
    let uuid = uuid::Uuid::new_v4();
    let ident = format!("{}_{}", prefix, uuid).replace('-', "_");
    syn::Ident::new(&ident, proc_macro2::Span::call_site())
}

#[proc_macro_derive(DbTable, attributes(from, pk, relation))]
pub fn db_table(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	let input: syn::DeriveInput = syn::parse(input).unwrap();
	let name = input.ident;
	let fields = if let syn::Data::Struct(data) = input.data {
		match data.fields {
			syn::Fields::Unit => {
				panic!("DeriveInput can't be a unit struct");
			},
			syn::Fields::Named(fields) => {
				fields
			},
			syn::Fields::Unnamed(_) => {
				panic!("DeriveInput can't be a tuple struct");
			}
		}
	} else {
		panic!("DeriveInput must be a struct");
	};
	let struct_from = input.attrs.iter().find(|a| a.path.segments.iter().next().map_or(false, |f| f.ident.to_string() == "from"));
	let struct_from = struct_from.expect("from attribute must be specified");
	let mut struct_from: FromAttributes = syn::parse(struct_from.tokens.clone().into()).expect("struct_from: FromAttributes");
	let from = struct_from.attr.expect("A db table must have a from string");
	let table = if let Some(table) = struct_from.named_arrs.remove("table") {
		table
	} else {
		from.clone()
	};
	let joins = if let Some(joins) = struct_from.named_arrs.remove("joins") {
		format!(" {}", joins)
	} else {
		"".to_string()
	};
	let pk: Vec<_> = fields.named.iter().filter_map(|f| f.attrs.iter().find(|a| a.path.segments.iter().next().map_or(false, |a| a.ident.to_string() == "pk")).map(|p| (f, p))).collect();
	let (fields_except_pk, relations) = {
		let all_except_pk = fields.named.iter().filter(|f| !f.attrs.iter().any(|a| a.path.segments.iter().next().map_or(false, |a| a.ident.to_string() == "pk")));
		let fields_except_pk: Vec<_> = all_except_pk.clone().filter(|f| !f.attrs.iter().any(|a| a.path.segments.iter().next().map_or(false, |a| a.ident.to_string() == "relation"))).collect();
		let relations: Vec<_> = all_except_pk.filter_map(|f| f.attrs.iter().find(|a| a.path.segments.iter().next().map_or(false, |a| a.ident.to_string() == "relation")).map(|r| (f, syn::parse::<TParse3<syn::Ident, syn::Token![,], syn::LitStr>>(r.tokens.clone().into()).expect("REL")))).collect();
		(fields_except_pk, relations)
	};
	let size = fields_except_pk.len() + 1;
	if pk.len() > 1 {
		panic!("There must be only one pk");
	} else if pk.len() == 0 {
		panic!("There must be one pk");
	}
	let (pk, pk_attribute) = pk[0];
	let pk_type = &pk.ty;
	let pk_name = &pk.ident;
	let pk = quote! { #pk_name: #pk_type };
	let pk_sql_name = if pk_attribute.tokens.is_empty() {
		format!("{}.{}", table, pk_name.as_ref().unwrap())
	} else {
		let s: syn::LitStr = syn::parse(pk_attribute.tokens.clone().into()).unwrap();
		format!("{}.{}", table, s.value())
	};
	let partial_data_fields = fields_except_pk.iter().map(|f| {
		let f_name = f.ident.as_ref().unwrap();
		let f_type = &f.ty;
		quote! { #f_name: #f_type }
	}).chain(
		relations.iter().map(|(f, TParse3(ty, _, _))| {
			let f_name = f.ident.as_ref().unwrap();
			quote! { #f_name: <#ty as #crate_name::db_table::DbTable>::DataCollector }
		})
	);
	let partial_data_init_collectors = relations.iter().map(|(f, TParse3(ty, _, _))| {
		let f_name = f.ident.as_ref().unwrap();
		quote! {
			let mut #f_name = <#ty as #crate_name::db_table::DbTable>::DataCollector::new(offset_sub);
			#f_name.push_next(row);
			let offset_sub = offset_sub + <#ty as #crate_name::db_table::DbTable>::DataCollector::SIZE;
		}
	});
	let partial_data_init = fields_except_pk.iter().enumerate().map(|(index, f)| {
		let f_name = f.ident.as_ref().unwrap();
		let index = index + 1;
		quote! { #f_name: row.take_opt(offset + #index)?.ok()? }
	}).chain(
		relations.iter().map(|(f, _)| {
			let f_name = f.ident.as_ref().unwrap();
			quote! { #f_name }
		})
	);
	let partial_data_destruct = fields_except_pk.iter().map(|f| {
		let f_name = f.ident.as_ref().unwrap();
		quote! { #f_name }
	}).chain(
		relations.iter().map(|(f, _)| {
			let f_name = f.ident.as_ref().unwrap();
			quote! { #f_name }
		})
	);
	let partial_data_build = fields_except_pk.iter().map(|f| {
		let f_name = f.ident.as_ref().unwrap();
		quote! { #f_name }
	}).chain(
		relations.iter().map(|(f, _)| {
			let f_name = f.ident.as_ref().unwrap();
			quote! { #f_name: #f_name.build() }
		})
	);
	let push_next_sub = relations.iter().map(|(f, _)| {
		let f_name = f.ident.as_ref().unwrap();
		quote! {
			current.#f_name.push_next(next_row);
		}
	});
	let mod_name = generate_unique_ident("__db_rel");
	let sql_names = [pk_sql_name.clone()].into_iter().chain(fields_except_pk.iter().map(|f| {
		let default_column_name = f.ident.as_ref().unwrap();
		if let Some(from) = f.attrs.iter().find(|a|  a.path.segments.iter().next().map_or(false, |a| a.ident.to_string() == "from")) {
			let mut from: FromAttributes = syn::parse(from.tokens.clone().into()).unwrap();
			let column_name = if let Some(column_name) = from.named_arrs.remove("column") {
				column_name
			} else if let Some(column_name) = from.attr {
				column_name
			} else {
				default_column_name.to_string()
			};
			let table = if let Some(table) = from.named_arrs.get("table") {
				table
			} else {
				&table
			};
			format!("{}.{}", table, column_name)
		} else {
			format!("{}.{}", table, default_column_name)
		}
	}));
	
	let null_format_string = (0..(fields_except_pk.len() + 1)).map(|_| "NULL").collect::<Vec<&'static str>>().join(",");
	let select_format_string = sql_names.collect::<Vec<String>>().join(",");
	let sql_fn = if relations.len() == 0 {
		quote! {
			(#table, #null_format_string.to_string(), vec![0], vec![(#select_format_string.to_string(), #joins.to_string())])
		}
	} else {
		let mut null_format_args = Vec::new();
		let mut sql_init = Vec::new();
		let mut t_len = Vec::new();
		let mut o_len = Vec::new();
		let mut o_ext = Vec::new();
		let mut v_ext = Vec::new();
		
		let mut null_format_string = null_format_string;
		let mut select_format_string = select_format_string;
		null_format_string.push_str(&(0..relations.len()).map(|_| ",{}").collect::<Vec<&'static str>>().join(""));
		select_format_string.push_str(&(0..relations.len()).map(|_| ",{}").collect::<Vec<&'static str>>().join(""));
		let null_format_string = null_format_string;
		let select_format_string = select_format_string;
		
		for (index, (_, TParse3(ty, _, fk))) in relations.iter().enumerate() {
			let f_name = syn::Ident::new(&format!("t{}", index), proc_macro2::Span::call_site());
			
			t_len.push(quote! { #f_name.3.len() });
			t_len.push(quote! { + });
			o_len.push(quote! { + #f_name.2.len() });
			
			sql_init.push(quote! { let #f_name = <#ty as #crate_name::db_table::DbTable>::DataCollector::sql(); });
			
			null_format_args.push(quote! { #f_name.1 });
			
			let from_format_string = format!("{}{{}} LEFT JOIN {{}} ON {}={{}}.{}{{}}", from, pk_sql_name, fk.value());
			let from_format_args = (0..relations.len())
				.map(|i|
					if i == index {
						quote! {select}
					} else {
						let f = syn::Ident::new(&format!("t{}", i), proc_macro2::Span::call_site());
						quote! {#f.1}
					}
				);
			
			v_ext.push(quote! {
				v.extend(#f_name.3.iter().map(|(select, from)| (
					format!(#select_format_string, #(#from_format_args,)*),
					format!(#from_format_string, #joins, #f_name.0, #f_name.0, from),
				)));
			});
			o_ext.push(quote! {
				o.extend(#f_name.2.iter().map(|i| i + current_offset));
				let current_offset = current_offset + <#ty as #crate_name::db_table::DbTable>::DataCollector::SIZE;
			});
		}
		t_len.pop();
		
		quote! {
			#(#sql_init)*
			let mut v = Vec::with_capacity(#(#t_len)*);
			let mut o = Vec::with_capacity(1 #(#o_len)*);
			#(#v_ext)*
			o.push(0);
			let current_offset = #size;
			#(#o_ext)*
			(#table, format!(#null_format_string, #(#null_format_args,)*), o, v)
		}
	};
	
	let result = quote! {
		impl #crate_name::db_table::DbTable for #name {
			type DataCollector = #mod_name::DataCollector;
			fn vec_from_rows(mut rows: Vec<#crate_name::mysql_async::Row>) -> Vec<Self> {
				let mut collector = <Self::DataCollector as #crate_name::db_table::DbTableDataCollector>::new(0);
				for row in rows.iter_mut() {
					#crate_name::db_table::DbTableDataCollector::push_next(&mut collector, row);
				}
				#crate_name::db_table::DbTableDataCollector::build(collector)
			}
		}
		mod #mod_name {
			use #crate_name::db_table::{DbTable, DbTableDataCollector};
			use super::*;
			use #crate_name::mysql_async::Row;
			
			struct PartialData {
				#pk,
				#(#partial_data_fields,)*
			}
		
			impl PartialData {
				fn new(pk: #pk_type, offset: usize, row: &mut Row) -> Option<Self> {
					let offset_sub = offset + <#name as #crate_name::db_table::DbTable>::DataCollector::SIZE;
					#(#partial_data_init_collectors)*
					Some(PartialData {
						#pk_name: pk,
						#(#partial_data_init,)*
					})
				}
		
				fn build(self) -> #name {
					let Self { #pk_name, #(#partial_data_destruct,)* } = self;
					#name {
						#pk_name,
						#(#partial_data_build,)*
					}
				}
			}
		
			pub struct DataCollector {
				offset: usize,
				current: Option<PartialData>,
				partial_result: Vec<#name>,
			}
			impl #crate_name::db_table::DbTableDataCollector for DataCollector {
				type Item = #name;
				const SIZE: usize = #size;
				
				fn sql() -> (&'static str, String, Vec<usize>, Vec<(String, String)>) { //(table_name, null_set, order_by, VEC<(select, joins)>)
					#sql_fn
				}
				
				fn new(offset: usize) -> Self {
					DataCollector {
						offset,
						current: None,
						partial_result: Vec::new(),
					}
				}
		
				fn push_next(&mut self, next_row: &mut Row) -> Option<()> {
					let #pk_name = next_row.take_opt(self.offset)?.ok()?;
					if let Some(current) = &mut self.current {
						if #pk_name == current.#pk_name {
							#(#push_next_sub)*
						} else {
							self.partial_result.push(std::mem::replace(current, PartialData::new(#pk_name, self.offset, next_row)?).build());
						}
					} else {
						self.current = PartialData::new(#pk_name, self.offset, next_row);
					}
					Some(())
				}
		
				fn build(mut self) -> Vec<#name> {
					if let Some(current) = self.current {
						self.partial_result.push(current.build());
					}
					self.partial_result
				}
			}
		}
		
	};
	result.into()
}