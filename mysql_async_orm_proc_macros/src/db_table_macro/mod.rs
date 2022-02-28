use std::{collections::HashMap, borrow::Borrow};
use quote::quote;
use syn::Result;

use crate::CRATE_NAME;

use self::into_db_table::{DbColumn, DbRelation, get_inner_type};

pub mod into_db_table;

pub fn get_struct_fields(input: &syn::DeriveInput) -> Result<&syn::FieldsNamed> {
	match &input.data {
		syn::Data::Struct(data) => {
			match &data.fields {
				syn::Fields::Unit => Err(syn::Error::new(input.ident.span(), "DeriveInput can't be a unit struct")),
				syn::Fields::Named(fields) => Ok(fields),
				syn::Fields::Unnamed(_) => Err(syn::Error::new(input.ident.span(), "DeriveInput can't be a tuple struct")),
			}
		}
		_ => Err(syn::Error::new(input.ident.span(), "DeriveInput must be a struct")),
	}
}

pub fn get_attributes<A: Borrow<syn::Attribute>>(attrs: impl Iterator<Item = A>) -> HashMap<String, A> {
	let mut result = HashMap::new();
	for a in attrs {
		let k: &syn::Attribute = a.borrow();
		let k = k.path.segments.iter().map(|s| s.ident.to_string()).collect();
		result.insert(k, a);
	}
	result
}

pub fn get_partial_data_fields(columns_except_pk: &Vec<DbColumn<'_>>, relations: &Vec<DbRelation<'_>>) -> Result<Vec<proc_macro2::TokenStream>> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	Ok(columns_except_pk.iter().map(|f| {
		let f_name = &f.rs_name_ident;
		let f_type = f.rs_type;
		quote! { #f_name: #f_type }
	}).chain(
		relations.iter().map(|r| {
			let f_name = r.rs_name_ident;
			let f_type = &r.ty;
			quote! { #f_name: <#f_type as #crate_name::db_table::DbTable>::DataCollector }
		})
	).collect())
}

pub fn get_partial_data_init_collectors(relations: &Vec<DbRelation<'_>>) -> Result<Vec<proc_macro2::TokenStream>> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	Ok(relations.iter().map(|r| {
		let f_name = r.rs_name_ident;
		let f_type = &r.ty;
		quote! {
			let mut #f_name = <#f_type as #crate_name::db_table::DbTable>::DataCollector::new(offset_sub);
			#f_name.push_next(row);
			let offset_sub = offset_sub + <#f_type as #crate_name::db_table::DbTable>::DataCollector::SIZE;
		}
	}).collect())
}

pub fn get_partial_data_init(columns_except_pk: &Vec<DbColumn<'_>>, relations: &Vec<DbRelation<'_>>) -> Result<Vec<proc_macro2::TokenStream>> {
	Ok(columns_except_pk.iter().enumerate().map(|(index, f)| {
		let f_name = f.rs_name_ident;
		let index = index + 1;
		quote! { #f_name: row.take_opt(offset + #index)?.ok()? }
	}).chain(
		relations.iter().map(|r| {
			let f_name = r.rs_name_ident;
			quote! { #f_name }
		})
	).collect())
}

pub fn get_partial_data_destruct(columns_except_pk: &Vec<DbColumn<'_>>, relations: &Vec<DbRelation<'_>>) -> Result<Vec<proc_macro2::TokenStream>> {
	Ok(columns_except_pk.iter().map(|f| {
		let f_name = f.rs_name_ident;
		quote! { #f_name }
	}).chain(
		relations.iter().map(|r| {
			let f_name = r.rs_name_ident;
			quote! { #f_name }
		})
	).collect())
}

pub fn get_partial_data_build(columns_except_pk: &Vec<DbColumn<'_>>, relations: &Vec<DbRelation<'_>>) -> Result<Vec<proc_macro2::TokenStream>> {
	Ok(columns_except_pk.iter().map(|f| {
		let f_name = f.rs_name_ident;
		quote! { #f_name }
	}).chain(
		relations.iter().map(|r| {
			let f_name = r.rs_name_ident;
			quote! { #f_name: #f_name.build() }
		})
	).collect())
}

pub fn get_push_next_sub(relations: &Vec<DbRelation<'_>>) -> Result<Vec<proc_macro2::TokenStream>> {
	Ok(relations.iter().map(|r| {
		let f_name = r.rs_name_ident;
		quote! {
			current.#f_name.push_next(next_row);
		}
	}).collect())
}

pub fn get_save_update_obj_fn(_: &into_db_table::DbTable) -> Result<proc_macro2::TokenStream> {
	Ok(quote!(todo!()))
}

pub fn get_save_insert_obj_fn(db_table: &into_db_table::DbTable) -> Result<proc_macro2::TokenStream> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	let columns_except_pk = db_table.columns_except_pk.iter().filter(|c| !c.readonly).collect::<Vec<_>>();
	let pk_inner_type = get_inner_type(db_table.pk.rs_type)?;
	let params: Vec<_> = columns_except_pk.iter().map(|c| {
		let c = c.rs_name_ident;
		quote! { self.#c.into() }
	}).collect();
	let insert_col_list = columns_except_pk.iter().map(|c| &c.db_name as &str).collect::<Vec<&str>>().join(",");
	let params_str = (0..columns_except_pk.len()).map(|_| "?").collect::<Vec<&str>>().join(",");
	let sql_insert = format!("INSERT INTO {} ({}) VALUES ({});", db_table.from.table, insert_col_list, params_str);
	let relation_insert = db_table.relations.iter().map(|r| {
		let rs_name = r.rs_name_ident;
		let rs_type = r.ty;
		let join_col = &r.join_col;
		quote! {
			{
				#crate_name::lazy_static! {
					static ref SUB_INS: (::std::string::String, fn(&::std::vec::Vec<#rs_type>, #pk_inner_type) -> ::std::vec::Vec<::std::vec::Vec<#crate_name::mysql_async::Value>>) = <<#rs_type as #crate_name::db_table::DbTable>::DataCollector as #crate_name::db_table::DbTableDataCollector>::get_insert_instr_as_sub(#join_col);
				}
				let (sql, iter_parse) = &*SUB_INS;
				connection.exec_batch(sql, iter_parse(&self.#rs_name, pk)).await?;
			}
		}
	});
	Ok(quote! {
		let params: ::std::vec::Vec<#crate_name::mysql_async::Value> = ::std::vec![#(#params,)*];
		connection.exec_drop(#sql_insert, params).await?;
		let pk = connection.last_insert_id().ok_or("")? as #pk_inner_type;
		
		#(#relation_insert)*
		
		::std::result::Result::Ok(pk)
	})
}

pub fn get_fn_get_insert_instr_as_sub(db_table: &into_db_table::DbTable) -> Result<proc_macro2::TokenStream> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	let columns_except_pk = db_table.columns_except_pk.iter().filter(|c| !c.readonly).collect::<Vec<_>>();
	let insert_col_list = columns_except_pk.iter().map(|c| &c.db_name as &str).collect::<Vec<&str>>().join(",");
	let params_str = (0..=columns_except_pk.len()).map(|_| "?").collect::<Vec<&str>>().join(",");
	let insert_str = format!("INSERT INTO {} ({{}},{}) VALUES ({});", db_table.from.table, insert_col_list, params_str);
	let pk_rs_name = db_table.pk.rs_name_ident;
	let params: Vec<_> = columns_except_pk.iter().map(|c| {
		let c = c.rs_name_ident;
		quote! { (&data.#c).into() }
	}).collect();
	let rs_type = db_table.from.rs_type;
	Ok(quote! {
		let insert_str = ::std::format!(#insert_str, fk);
		fn format_data<K: ::std::convert::Into<#crate_name::mysql_async::Value> + ::std::marker::Copy>(input: &::std::vec::Vec<#rs_type>, fk: K) -> ::std::vec::Vec<::std::vec::Vec<#crate_name::mysql_async::Value>> {
			input.iter().filter(|v| v.#pk_rs_name.is_none()).map(|data|
				vec![fk.into(), #(#params,)*]
			).collect()
		}
		(insert_str, format_data::<K>)
	})
}