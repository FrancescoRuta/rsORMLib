use std::{collections::HashMap, borrow::Borrow};
use quote::quote;
use syn::Result;

use crate::CRATE_NAME;

use self::into_db_model::{DbColumn, DbRelation};

pub mod into_db_model;

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
			quote! { #f_name: <#f_type as #crate_name::db_model::DbModel>::DataCollector }
		})
	).collect())
}

pub fn get_partial_data_init_collectors(relations: &Vec<DbRelation<'_>>) -> Result<Vec<proc_macro2::TokenStream>> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	Ok(relations.iter().map(|r| {
		let f_name = r.rs_name_ident;
		let f_type = &r.ty;
		quote! {
			let mut #f_name = <#f_type as #crate_name::db_model::DbModel>::DataCollector::new(offset_sub);
			#f_name.push_next(row);
			let offset_sub = offset_sub + <#f_type as #crate_name::db_model::DbModel>::DataCollector::SIZE;
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

pub fn get_prepare_insert(db_model: &into_db_model::DbModel) -> Result<proc_macro2::TokenStream> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	let columns_except_pk = db_model.columns_except_pk.iter().filter(|c| !c.readonly).collect::<Vec<_>>();
	let insert_col_list = columns_except_pk.iter().map(|c| &c.db_name as &str).collect::<Vec<&str>>().join(",");
	let insert_str_with_fk = format!("INSERT INTO {} ({{}},{}) VALUES (@id_{{}}", db_model.from.table, insert_col_list);
	let insert_str_without_fk = format!("INSERT INTO {} ({}) VALUES (", db_model.from.table, insert_col_list);
	let pk_rs_name = db_model.pk.rs_name_ident;
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
	let relations: Vec<_> = db_model.relations.iter().map(|r| {
		let rs_type = r.ty;
		let rs_name = r.rs_name_ident;
		let join_col = &r.join_col;
		quote! {
			for row in &data.#rs_name {
				<#rs_type as #crate_name::db_model::DbModel>::prepare_insert(Some(#join_col), row, query, this_id);
			}
		}
	}).collect();
	Ok(quote! {
		if data.#pk_rs_name.is_none() {
			let this_id = this_id + 1;
			if let Some(fk_db_name) = fk {
				query.push_str(&::std::format!(#insert_str_with_fk, fk_db_name, this_id - 1));
				#(#query_push_with_fk)*
			} else {
				query.push_str(#insert_str_without_fk);
				#(#query_push_without_fk)*
			}
			query.push_str(");SET @id_");
			query.push_str(&format!("{}", this_id));
			query.push_str(" = LAST_INSERT_ID();");
			
			#(#relations)*
		}
	})
}

pub fn get_prepare_delete(db_model: &into_db_model::DbModel) -> Result<proc_macro2::TokenStream> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	let delete_str_with_fk = format!("DELETE FROM {} WHERE {{}}=@id_{{}} AND {}=@id_{{}};", db_model.from.table, db_model.pk.db_name);
	let delete_str_without_fk = format!("DELETE FROM {} WHERE {}=@id_{{}};", db_model.from.table, db_model.pk.db_name);
	let pk_rs_name = db_model.pk.rs_name_ident;
	let relations: Vec<_> = db_model.relations.iter().map(|r| {
		let rs_type = r.ty;
		let rs_name = r.rs_name_ident;
		let join_col = &r.join_col;
		quote! {
			for row in &data.#rs_name {
				<#rs_type as #crate_name::db_model::DbModel>::prepare_delete(Some(#join_col), row, query, this_id);
			}
		}
	}).collect();
	Ok(quote! {
		if let Some(pk) = data.#pk_rs_name {
			let this_id = this_id + 1;
			query.push_str(&::std::format!("SET @id_{}={};", this_id, pk));
			#(#relations)*
			query.push_str(&if let Some(fk_db_name) = fk {
				::std::format!(#delete_str_with_fk, fk_db_name, this_id - 1, this_id)
			} else {
				::std::format!(#delete_str_without_fk, this_id)
			});
		}
	})
}


pub fn get_prepare_update(db_model: &into_db_model::DbModel) -> Result<proc_macro2::TokenStream> {
	let crate_name = syn::Ident::new(CRATE_NAME, proc_macro2::Span::call_site());
	let columns_except_pk = db_model.columns_except_pk.iter().filter(|c| !c.readonly).collect::<Vec<_>>();
	let update_str_prefix = format!("UPDATE {} SET ", db_model.from.table);
	let update_str_with_fk_suffix = format!(" WHERE {{}}=@id_{{}} AND {}=@id_{{}};", db_model.pk.db_name);
	let update_str_without_fk_suffix = format!(" WHERE {}=@id_{{}};", db_model.pk.db_name);
	let pk_rs_name = db_model.pk.rs_name_ident;
	let query_push_update_cols = columns_except_pk.iter().enumerate().map(|(index, c)| {
		let comma = if index == 0 { quote!() } else { quote!(query.push(',');) };
		let db_name_str = format!("{}=", c.db_name);
		let rs_name = c.rs_name_ident;
		quote! {
			#comma
			query.push_str(#db_name_str);
			query.push_str(&#crate_name::mysql_async::Value::from(&new_data.#rs_name).as_sql(false));
		}
	});
	let relations: Vec<_> = db_model.relations.iter().map(|r| {
		let rs_type = r.ty;
		let rs_name = r.rs_name_ident;
		let join_col = &r.join_col;
		quote! {
			let mut old_rows = ::std::collections::HashMap::new();
			for row in &old_data.#rs_name {
				old_rows.insert(<#rs_type as #crate_name::db_model::DbModel>::get_pk(row).unwrap(), row);
			}
			for row in &new_data.#rs_name {
				if let Some(pk) = <#rs_type as #crate_name::db_model::DbModel>::get_pk(row) {
					if let Some(old_row) = old_rows.remove(&pk) {
						<#rs_type as #crate_name::db_model::DbModel>::prepare_update(Some(#join_col), row, old_row, query, this_id);
					}
				} else {
					<#rs_type as #crate_name::db_model::DbModel>::prepare_insert(Some(#join_col), row, query, this_id);
				}
			}
			for (_, row) in old_rows {
				<#rs_type as #crate_name::db_model::DbModel>::prepare_delete(Some(#join_col), row, query, this_id);
			}
		}
	}).collect();
	if query_push_update_cols.len() != 0 {
		Ok(quote! {
			if let Some(pk) = new_data.#pk_rs_name {
				let this_id = this_id + 1;
				query.push_str(&::std::format!("SET @id_{}={};", this_id, pk));
				query.push_str(#update_str_prefix);
				#(#query_push_update_cols)*
				if let Some(fk_db_name) = fk {
					query.push_str(&::std::format!(#update_str_with_fk_suffix, fk_db_name, this_id - 1, this_id));
				} else {
					query.push_str(&::std::format!(#update_str_without_fk_suffix, this_id));
				}
				#(#relations)*
			}
		})
	} else {
		Ok(quote! {
			if let Some(pk) = new_data.#pk_rs_name {
				let this_id = this_id + 1;
				query.push_str(&::std::format!("SET @id_{}={};", this_id, pk));
				#(#relations)*
			}
		})
	}
}