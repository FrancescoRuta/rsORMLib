use std::{collections::HashMap, borrow::Borrow};
use quote::quote;
use syn::Result;

use crate::CRATE_NAME;

use self::into_db_table::{DbColumn, DbRelation};

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

pub fn get_prepare_insert(db_table: &into_db_table::DbTable) -> Result<proc_macro2::TokenStream> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	let columns_except_pk = db_table.columns_except_pk.iter().filter(|c| !c.readonly).collect::<Vec<_>>();
	let insert_col_list = columns_except_pk.iter().map(|c| &c.db_name as &str).collect::<Vec<&str>>().join(",");
	let insert_str_with_fk = format!("INSERT INTO {} ({{}},{}) VALUES (@id_{{}}", db_table.from.table, insert_col_list);
	let insert_str_without_fk = format!("INSERT INTO {} ({}) VALUES (", db_table.from.table, insert_col_list);
	let pk_rs_name = db_table.pk.rs_name_ident;
	let query_push_with_fk = columns_except_pk.iter().map(|col| {
		let rs_name = col.rs_name_ident;
		quote! {
			query.push(',');
			query.push_str(&#crate_name::mysql_async::Value::from(&data.#rs_name).as_sql(false));
		}
	});
	let query_push_without_fk = columns_except_pk.iter().enumerate().map(|(index, col)| {
		let comma = if index == 0 { quote!() } else { quote!(query.push(',');) };
		let rs_name = col.rs_name_ident;
		quote! {
			#comma
			query.push_str(&#crate_name::mysql_async::Value::from(&data.#rs_name).as_sql(false));
		}
	});
	let relations: Vec<_> = db_table.relations.iter().map(|r| {
		let rs_type = r.ty;
		let rs_name = r.rs_name_ident;
		let join_col = &r.join_col;
		quote! {
			for row in &data.#rs_name {
				<#rs_type as #crate_name::db_table::DbTable>::prepare_insert(Some(#join_col), row, query, this_id);
			}
		}
	}).collect();
	Ok(quote! {
		if data.#pk_rs_name.is_none() {
			let this_id = this_id + 1;
			if let Some(fk_db_name) = fk {
				query.push_str(&::std::format!(#insert_str_with_fk, fk_db_name, this_id - 1));
				#(#query_push_with_fk)*
				query.push_str(");SET @id_");
				query.push_str(&format!("{}", this_id));
				query.push_str(" = LAST_INSERT_ID();");
				
				#(#relations)*
			} else {
				query.push_str(#insert_str_without_fk);
				#(#query_push_without_fk)*
				query.push_str(");SET @id_");
				query.push_str(&format!("{}", this_id));
				query.push_str(" = LAST_INSERT_ID();");
				
				#(#relations)*
			}
		}
	})
}