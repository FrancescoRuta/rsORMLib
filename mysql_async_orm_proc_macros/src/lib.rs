use quote::quote;
use syn::Result;

mod db_table_parse;
mod db_table_macro;

const CRATE_NAME: &'static str = "mysql_async_orm";


fn generate_unique_ident(prefix: &str) -> syn::Ident {
    let uuid = uuid::Uuid::new_v4();
    let ident = format!("{}_{}", prefix, uuid).replace('-', "_");
    syn::Ident::new(&ident, proc_macro2::Span::call_site())
}

fn db_table_macro(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	let crate_name = quote! { ::#crate_name };
	let name = &input.ident;
	let fields = db_table_macro::get_struct_fields(input)?;
	let struct_attributes = db_table_macro::get_attributes(input.attrs.iter());
	
	let db_table = db_table_macro::into_db_table::get_db_table(input, &struct_attributes, fields)?;
	let db_table_col_count = db_table.columns_except_pk.len() + 1;
	let pk_type = db_table.pk.rs_type;
	let pk_inner_type = db_table_macro::into_db_table::get_inner_type(pk_type)?;
	let pk_name_ident = db_table.pk.rs_name_ident;
	let pk_db_string = format!("{}.{}", db_table.from.table, db_table.pk.db_name);
	let partial_data_fields = db_table_macro::get_partial_data_fields(&db_table.columns_except_pk, &db_table.relations)?;
	let partial_data_init_collectors = db_table_macro::get_partial_data_init_collectors(&db_table.relations)?;
	let partial_data_init = db_table_macro::get_partial_data_init(&db_table.columns_except_pk, &db_table.relations)?;
	let partial_data_destruct = db_table_macro::get_partial_data_destruct(&db_table.columns_except_pk, &db_table.relations)?;
	let partial_data_build = db_table_macro::get_partial_data_build(&db_table.columns_except_pk, &db_table.relations)?;
	let push_next_sub = db_table_macro::get_push_next_sub(&db_table.relations)?;
	let mod_name = generate_unique_ident("__db_rel");
	let prepare_insert = db_table_macro::get_prepare_insert(&db_table)?;
	let prepare_update = db_table_macro::get_prepare_update(&db_table)?;
	let prepare_delete = db_table_macro::get_prepare_delete(&db_table)?;
	
	let get_by_pk_sql = format!("SELECT {{}} FROM {{}} WHERE {}=?", pk_db_string);
	
	let sql_names = [pk_db_string.clone()].into_iter().chain(db_table.columns_except_pk.iter().map(|f| {
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
			let table = if let Some(table) = from.named_arrs.get("table") {
				table
			} else {
				&db_table.from.table
			};
			format!("{}.{}", table, column_name)
		} else {
			format!("{}.{}", db_table.from.table, default_column_name)
		}
	}));
	let null_format_string = (0..(db_table.columns_except_pk.len() + 1)).map(|_| "NULL").collect::<Vec<&'static str>>().join(",");
	let joins = &db_table.from.joins;
	let table = &db_table.from.table;
	let from = &db_table.from.from;
	let select_format_string = sql_names.collect::<Vec<String>>().join(",");
	let sql_fn = if db_table.relations.len() == 0 {
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
		null_format_string.push_str(&(0..db_table.relations.len()).map(|_| ",{}").collect::<Vec<&'static str>>().join(""));
		select_format_string.push_str(&(0..db_table.relations.len()).map(|_| ",{}").collect::<Vec<&'static str>>().join(""));
		let null_format_string = null_format_string;
		let select_format_string = select_format_string;
		
		for (index, r) in db_table.relations.iter().enumerate() {
			let ty = &r.ty;
			let fk = &r.join_col;
			let f_name = syn::Ident::new(&format!("t{}", index), proc_macro2::Span::call_site());
			
			t_len.push(quote! { #f_name.3.len() });
			t_len.push(quote! { + });
			o_len.push(quote! { + #f_name.2.len() });
			
			sql_init.push(quote! { let #f_name = <#ty as #crate_name::db_table::DbTable>::DataCollector::sql(); });
			
			null_format_args.push(quote! { #f_name.1 });
			
			let from_format_string = format!("{}{{}} LEFT JOIN {{}} ON {}={{}}.{}{{}}", from, pk_db_string, fk);
			let from_format_args = (0..db_table.relations.len())
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
			let current_offset = #db_table_col_count;
			#(#o_ext)*
			(#table, format!(#null_format_string, #(#null_format_args,)*), o, v)
		}
	};
	
	let res = quote! {
		impl #crate_name::db_table::DbTable for #name {
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
			fn vec_from_rows(mut rows: Vec<#crate_name::mysql_async::Row>) -> ::std::vec::Vec<Self> {
				let mut collector = <<Self as #crate_name::db_table::DbTable>::DataCollector as #crate_name::db_table::DbTableDataCollector>::new(0);
				for row in rows.iter_mut() {
					#crate_name::db_table::DbTableDataCollector::push_next(&mut collector, row);
				}
				#crate_name::db_table::DbTableDataCollector::build(collector)
			}
			async fn get_by_pk(pk: #pk_inner_type, connection: &mut #crate_name::db_connection::DbConnection) -> ::std::result::Result<Self, #crate_name::db_connection::DbError> {
				#crate_name::lazy_static! {
					static ref SQL: ::std::string::String = {
						let (_, _, order_by, sql) = <<#name as #crate_name::db_table::DbTable>::DataCollector as #crate_name::db_table::DbTableDataCollector>::sql();
						let sql = sql.iter().map(|(select, from)| format!(
							#get_by_pk_sql,
							select, from
						)).collect::<::std::vec::Vec<_>>().join(" UNION ALL ");
						let sql = std::format!("{} ORDER BY {};", sql, order_by.iter().map(|i| (i + 1).to_string()).collect::<::std::vec::Vec<_>>().join(","));
						sql
					};
					static ref PARAM_COUNT: usize = <<#name as #crate_name::db_table::DbTable>::DataCollector as #crate_name::db_table::DbTableDataCollector>::sql().3.len();
				}
				let params: ::std::vec::Vec<_> = (0..*PARAM_COUNT).map(|_| pk).collect();
				let sql: &str = &*SQL;
				let mut data = Self::vec_from_rows(connection.exec(sql, params).await?);
				let mut data = data.drain(..);
				data.next().ok_or(#crate_name::db_connection::DbError::Other(::std::borrow::Cow::Borrowed("Not found")))
			}
			async fn exec_update(&self, connection: &mut #crate_name::db_connection::DbConnection) -> ::std::result::Result<Self, #crate_name::db_connection::DbError> {
				if let Some(pk) = &self.#pk_name_ident {
					let old_value = Self::get_by_pk(*pk, connection).await?;
					let mut query = String::new();
					<Self as #crate_name::db_table::DbTable>::prepare_update(::std::option::Option::None, self, &old_value, &mut query, 0);
					connection.query_drop(query).await?;
					Ok(old_value)
				} else {
					::std::result::Result::Err(#crate_name::db_connection::DbError::Other(::std::borrow::Cow::Borrowed("Pk must be Some")))
				}
			}
			async fn exec_delete(#pk_name_ident: #pk_inner_type, connection: &mut #crate_name::db_connection::DbConnection) -> ::std::result::Result<Self, #crate_name::db_connection::DbError> {
				let old_value = Self::get_by_pk(#pk_name_ident, connection).await?;
				let mut query = String::new();
				<Self as #crate_name::db_table::DbTable>::prepare_delete(::std::option::Option::None, &old_value, &mut query, 0);
				connection.query_drop(query).await?;
				Ok(old_value)
			}
			async fn exec_insert(&self, connection: &mut #crate_name::db_connection::DbConnection) -> ::std::result::Result<#pk_inner_type, #crate_name::db_connection::DbError> {
				if self.#pk_name_ident.is_none() {
					let mut query = String::new();
					<Self as #crate_name::db_table::DbTable>::prepare_insert(::std::option::Option::None, self, &mut query, 0);
					query.push_str("SELECT @id_1;");
					let mut res = connection.query_iter(query).await?;
					let mut last_row = None;
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
			use #crate_name::db_table::{DbTable, DbTableDataCollector};
			use super::*;
			use #crate_name::mysql_async::Row;
			
			struct PartialData {
				#pk_name_ident: #pk_type,
				#(#partial_data_fields,)*
			}
		
			impl PartialData {
				fn new(pk: #pk_type, offset: usize, row: &mut Row) -> Option<Self> {
					let offset_sub = offset + <<#name as #crate_name::db_table::DbTable>::DataCollector as #crate_name::db_table::DbTableDataCollector>::SIZE;
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
			impl #crate_name::db_table::DbTableDataCollector for DataCollector {
				type Item = #name;
				const SIZE: usize = #db_table_col_count;
				
				fn sql() -> (&'static str, String, Vec<usize>, Vec<(String, String)>) {
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

#[proc_macro_derive(DbTable, attributes(from, pk, relation, readonly))]
pub fn db_table(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = syn::parse_macro_input!(input as syn::DeriveInput);
	db_table_macro(&input).unwrap_or_else(syn::Error::into_compile_error).into()
}