use quote::quote;
use syn::Result;

mod db_model_parse;
mod db_model_macro;

const CRATE_NAME: &'static str = "mysql_async_orm";


fn generate_unique_ident(prefix: &str) -> syn::Ident {
    let uuid = uuid::Uuid::new_v4();
    let ident = format!("{}_{}", prefix, uuid).replace('-', "_");
    syn::Ident::new(&ident, proc_macro2::Span::call_site())
}

fn db_model_macro(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	let crate_name = quote! { ::#crate_name };
	let name = &input.ident;
	let fields = db_model_macro::get_struct_fields(input)?;
	let struct_attributes = db_model_macro::get_attributes(input.attrs.iter());
	
	let db_model = db_model_macro::into_db_model::get_db_model(input, &struct_attributes, fields)?;
	let db_model_col_count = db_model.columns_except_pk.len() + 1;
	let pk_type = db_model.pk.rs_type;
	let pk_inner_type = db_model_macro::into_db_model::get_inner_type(pk_type)?;
	let pk_name_ident = db_model.pk.rs_name_ident;
	let pk_db_string = format!("{}.{}", db_model.from.table, db_model.pk.db_name);
	let partial_data_fields = db_model_macro::get_partial_data_fields(&db_model.columns_except_pk, &db_model.relations)?;
	let partial_data_init_collectors = db_model_macro::get_partial_data_init_collectors(&db_model.relations)?;
	let partial_data_init = db_model_macro::get_partial_data_init(&db_model.columns_except_pk, &db_model.relations)?;
	let partial_data_destruct = db_model_macro::get_partial_data_destruct(&db_model.columns_except_pk, &db_model.relations)?;
	let partial_data_build = db_model_macro::get_partial_data_build(&db_model.columns_except_pk, &db_model.relations)?;
	let push_next_sub = db_model_macro::get_push_next_sub(&db_model.relations)?;
	let mod_name = generate_unique_ident("__db_rel");
	let prepare_insert = db_model_macro::get_prepare_insert(&db_model)?;
	let prepare_update = db_model_macro::get_prepare_update(&db_model)?;
	let prepare_delete = db_model_macro::get_prepare_delete(&db_model)?;
	
	let get_by_pk_sql = format!("SELECT {{}} FROM {{}} {{}} WHERE {}=?", pk_db_string);
	
	let sql_names = [pk_db_string.clone()].into_iter().chain(db_model.columns_except_pk.iter().map(|f| {
		let default_column_name = f.rs_name_ident;
		if let Some(from) = &f.from_attribute {
			let mut from = from.clone();
			let column_name = if let Some(column_name) = from.named_arrs.remove("column") {
				column_name
			} else if let Some(column_name) = from.attr {
				column_name
			} else {
				default_column_name.to_string()
			};
			let expression = from.named_arrs.remove("expression");
			let table = if let Some(table) = from.named_arrs.get("table") {
				table
			} else {
				&db_model.from.table
			};
			if let Some(expression) = expression {
				expression
			} else {
				format!("{}.{}", table, column_name)
			}
		} else {
			format!("{}.{}", db_model.from.table, default_column_name)
		}
	}));
	let null_format_string = (0..(db_model.columns_except_pk.len() + 1)).map(|_| "NULL").collect::<Vec<&'static str>>().join(",");
	let joins = &db_model.from.joins;
	let table = &db_model.from.table;
	let from = &db_model.from.from;
	let select_format_string = sql_names.collect::<Vec<String>>().join(",");
	let sql_fn = if db_model.relations.len() == 0 {
		quote! {
			(#table, #null_format_string.to_string(), vec![0], vec![(#from, #select_format_string.to_string(), #joins.to_string())])
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
		null_format_string.push_str(&(0..db_model.relations.len()).map(|_| ",{}").collect::<Vec<&'static str>>().join(""));
		select_format_string.push_str(&(0..db_model.relations.len()).map(|_| ",{}").collect::<Vec<&'static str>>().join(""));
		let null_format_string = null_format_string;
		let select_format_string = select_format_string;
		
		for (index, r) in db_model.relations.iter().enumerate() {
			let ty = &r.ty;
			let fk = &r.join_col;
			let f_name = syn::Ident::new(&format!("t{}", index), proc_macro2::Span::call_site());
			
			t_len.push(quote! { #f_name.3.len() });
			t_len.push(quote! { + });
			o_len.push(quote! { + #f_name.2.len() });
			
			sql_init.push(quote! { let #f_name = <#ty as #crate_name::db_model::DbModel>::DataCollector::sql(); });
			
			null_format_args.push(quote! { #f_name.1 });
			
			let from_format_string = format!("{{}} LEFT JOIN {{}} ON {}={{}}.{} {{}}", pk_db_string, fk);
			let from_format_args = (0..db_model.relations.len())
				.map(|i|
					if i == index {
						quote! {select}
					} else {
						let f = syn::Ident::new(&format!("t{}", i), proc_macro2::Span::call_site());
						quote! {#f.1}
					}
				);
			
			v_ext.push(quote! {
				v.extend(#f_name.3.iter().map(|(_, select, from)| (
					#from,
					format!(#select_format_string, #(#from_format_args,)*),
					format!(#from_format_string, #joins, #f_name.0, #f_name.0, from),
				)));
			});
			o_ext.push(quote! {
				o.extend(#f_name.2.iter().map(|i| i + current_offset));
				let current_offset = current_offset + <#ty as #crate_name::db_model::DbModel>::DataCollector::SIZE;
			});
		}
		t_len.pop();
		
		quote! {
			#(#sql_init)*
			let mut v = Vec::with_capacity(#(#t_len)*);
			let mut o = Vec::with_capacity(1 #(#o_len)*);
			#(#v_ext)*
			o.push(0);
			let current_offset = #db_model_col_count;
			#(#o_ext)*
			(#table, format!(#null_format_string, #(#null_format_args,)*), o, v)
		}
	};
	
	let res = quote! {
		impl #crate_name::db_model::DbModel for #name {
			type DataCollector = #mod_name::DataCollector;
			type PrimaryKey = #pk_inner_type;
			fn prepare_insert(fk: ::std::option::Option<&::std::primitive::str>, data: &Self, query: &mut ::std::string::String, this_id: ::std::primitive::usize) {
				#prepare_insert
			}
			fn prepare_update(fk: ::std::option::Option<&::std::primitive::str>, new_data: &Self, old_data: &Self, query: &mut ::std::string::String, this_id: ::std::primitive::usize) {
				#prepare_update
			}
			fn prepare_delete(fk: ::std::option::Option<&::std::primitive::str>, data: &Self, query: &mut ::std::string::String, this_id: ::std::primitive::usize) {
				#prepare_delete
			}
			fn get_pk(&self) -> ::std::option::Option<Self::PrimaryKey> {
				self.#pk_name_ident
			}
		}
		impl #name {
			pub fn vec_from_rows(mut rows: Vec<#crate_name::mysql_async::Row>) -> ::std::vec::Vec<Self> {
				let mut collector = <<Self as #crate_name::db_model::DbModel>::DataCollector as #crate_name::db_model::DbModelDataCollector>::new(0);
				for row in rows.iter_mut() {
					#crate_name::db_model::DbModelDataCollector::push_next(&mut collector, row);
				}
				#crate_name::db_model::DbModelDataCollector::build(collector)
			}
			pub async fn get_by_pk(pk: #pk_inner_type, connection: &mut impl #crate_name::db_connection::QueryableConn) -> ::std::result::Result<Self, #crate_name::db_connection::DbError> {
				#crate_name::lazy_static! {
					static ref SQL: ::std::string::String = {
						let (_, _, order_by, sql) = <<#name as #crate_name::db_model::DbModel>::DataCollector as #crate_name::db_model::DbModelDataCollector>::sql();
						let sql = sql.iter().map(|(table, select, from)| format!(
							#get_by_pk_sql,
							select, table, from
						)).collect::<::std::vec::Vec<_>>().join(" UNION ALL ");
						let sql = std::format!("{} ORDER BY {};", sql, order_by.iter().map(|i| (i + 1).to_string()).collect::<::std::vec::Vec<_>>().join(","));
						sql
					};
					static ref PARAM_COUNT: usize = <<#name as #crate_name::db_model::DbModel>::DataCollector as #crate_name::db_model::DbModelDataCollector>::sql().3.len();
				}
				let params: ::std::vec::Vec<_> = (0..*PARAM_COUNT).map(|_| pk).collect();
				let sql: &str = &*SQL;
				let mut data = Self::vec_from_rows(connection.exec(sql, params).await?);
				let mut data = data.drain(..);
				data.next().ok_or(#crate_name::db_connection::DbError::Other(::std::borrow::Cow::Borrowed("Not found")))
			}
			pub async fn exec_update(&self, connection: &mut impl #crate_name::db_connection::QueryableConn) -> ::std::result::Result<Self, #crate_name::db_connection::DbError> {
				if let ::std::option::Option::Some(pk) = &self.#pk_name_ident {
					let old_value = Self::get_by_pk(*pk, connection).await?;
					let mut query = ::std::string::String::new();
					<Self as #crate_name::db_model::DbModel>::prepare_update(::std::option::Option::None, self, &old_value, &mut query, 0);
					if ::std::cfg!(debug_assertions) {
						if let ::std::result::Result::Err(error) = connection.query_drop(&query).await {
							println!("Error: {}\nIn: ```{}```", error, query);
							return ::std::result::Result::Err(error);
						}
					} else {
						connection.query_drop(query).await?;
					}
					::std::result::Result::Ok(old_value)
				} else {
					::std::result::Result::Err(#crate_name::db_connection::DbError::Other(::std::borrow::Cow::Borrowed("Pk must be Some")))
				}
			}
			pub async fn exec_delete(#pk_name_ident: #pk_inner_type, connection: &mut impl #crate_name::db_connection::QueryableConn) -> ::std::result::Result<Self, #crate_name::db_connection::DbError> {
				let old_value = Self::get_by_pk(#pk_name_ident, connection).await?;
				let mut query = ::std::string::String::new();
				<Self as #crate_name::db_model::DbModel>::prepare_delete(::std::option::Option::None, &old_value, &mut query, 0);
				if ::std::cfg!(debug_assertions) {
					if let ::std::result::Result::Err(error) = connection.query_drop(&query).await {
						println!("Error: {}\nIn: ```{}```", error, query);
						return ::std::result::Result::Err(error);
					}
				} else {
					connection.query_drop(query).await?;
				}
				Ok(old_value)
			}
			pub async fn exec_insert(&self, connection: &mut impl #crate_name::db_connection::QueryableConn) -> ::std::result::Result<#pk_inner_type, #crate_name::db_connection::DbError> {
				if self.#pk_name_ident.is_none() {
					let mut query = ::std::string::String::new();
					<Self as #crate_name::db_model::DbModel>::prepare_insert(::std::option::Option::None, self, &mut query, 0);
					query.push_str("SELECT @id_1;");
					let mut res = if ::std::cfg!(debug_assertions) {
						match connection.query_iter(&query).await {
							::std::result::Result::Ok(res) => res,
							::std::result::Result::Err(error) => {
								println!("Error: {}\nIn: ```{}```", error, query);
								return ::std::result::Result::Err(error);
							}
						}
					} else {
						connection.query_iter(query).await?
					};
					let mut last_row = ::std::option::Option::None;
					while !res.is_empty() {
						res.for_each(|row| last_row = Some(row)).await?;
					}
					let mut last_row = last_row.ok_or(#crate_name::db_connection::DbError::Other(::std::borrow::Cow::Borrowed("Unknown error")))?;
					last_row.take(0).ok_or(#crate_name::db_connection::DbError::Other(::std::borrow::Cow::Borrowed("Unknown error")))
				} else {
					::std::result::Result::Err(#crate_name::db_connection::DbError::Other(::std::borrow::Cow::Borrowed("Pk must be None")))
				}
			}
		}
		mod #mod_name {
			use #crate_name::db_model::{DbModel, DbModelDataCollector};
			use super::*;
			use #crate_name::mysql_async::Row;
			
			struct PartialData {
				#pk_name_ident: #pk_type,
				#(#partial_data_fields,)*
			}
		
			impl PartialData {
				fn new(pk: #pk_type, offset: usize, row: &mut Row) -> Option<Self> {
					let offset_sub = offset + <<#name as #crate_name::db_model::DbModel>::DataCollector as #crate_name::db_model::DbModelDataCollector>::SIZE;
					#(#partial_data_init_collectors)*
					Some(PartialData {
						#pk_name_ident: pk,
						#(#partial_data_init,)*
					})
				}
		
				fn build(self) -> #name {
					let Self { #pk_name_ident, #(#partial_data_destruct,)* } = self;
					#name {
						#pk_name_ident,
						#(#partial_data_build,)*
					}
				}
			}
		
			pub struct DataCollector {
				offset: usize,
				current: Option<PartialData>,
				partial_result: Vec<#name>,
			}
			impl #crate_name::db_model::DbModelDataCollector for DataCollector {
				type Item = #name;
				const SIZE: usize = #db_model_col_count;
				
				fn sql() -> (&'static str, String, Vec<usize>, Vec<(&'static str, String, String)>) {
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
					let #pk_name_ident = next_row.take_opt(self.offset)?.ok();
					if let Some(current) = &mut self.current {
						if #pk_name_ident == current.#pk_name_ident {
							#(#push_next_sub)*
						} else {
							self.partial_result.push(std::mem::replace(current, PartialData::new(#pk_name_ident, self.offset, next_row)?).build());
						}
					} else {
						self.current = PartialData::new(#pk_name_ident, self.offset, next_row);
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
	
	//println!("{}", res.to_string());
	
	Ok(res)
}

#[proc_macro_derive(DbModel, attributes(from, pk, relation, readonly))]
pub fn db_model(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = syn::parse_macro_input!(input as syn::DeriveInput);
	db_model_macro(&input).unwrap_or_else(syn::Error::into_compile_error).into()
}

fn sql_order_by_macro(input: proc_macro::TokenStream) -> Result<proc_macro2::TokenStream> {
	struct SqlOrderByInput(syn::Ident, syn::Token![,], Array, syn::Token![,], syn::Expr, syn::Token![,], syn::Expr, Option<syn::Token![,]>);
	impl syn::parse::Parse for SqlOrderByInput {
		fn parse(input: syn::parse::ParseStream) -> Result<Self> {
			Ok(Self(input.parse()?, input.parse()?, input.parse()?, input.parse()?, input.parse()?, input.parse()?, input.parse()?, input.parse()?))
		}
	}
	struct Array(syn::punctuated::Punctuated<syn::LitStr, syn::Token![,]>);
	impl syn::parse::Parse for Array {
		fn parse(input: syn::parse::ParseStream) -> Result<Self> {
			let content;
			let _ = syn::bracketed!(content in input);
			Ok(Self(syn::punctuated::Punctuated::parse_terminated(&content)?))
		}
	}
	let SqlOrderByInput(order_by_var, _, cols, _, sql_prefix, _, sql_suffix, _) = syn::parse(input)?;
	let cols = cols.0.into_iter().enumerate().map(|(index, col)| {
		let col = col.value();
		let index_1 = index * 2 + 1;
		let index_2 = index * 2 + 2;
		let order_by_1 = format!(" ORDER BY {} ASC ", col);
		let order_by_2 = format!(" ORDER BY {} DESC ", col);
		quote! {
			#index_1 => Ok(concat!(#sql_prefix, #order_by_1, #sql_suffix)),
			#index_2 => Ok(concat!(#sql_prefix, #order_by_2, #sql_suffix)),
		}
	});
	Ok((quote! {
		match #order_by_var {
			0 => Ok(concat!(#sql_prefix, #sql_suffix)),
			#(#cols)*
			_ => Err(()),
		}
	}).into())
}

#[proc_macro]
pub fn sql_order_by(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	sql_order_by_macro(input).unwrap_or_else(syn::Error::into_compile_error).into()
}