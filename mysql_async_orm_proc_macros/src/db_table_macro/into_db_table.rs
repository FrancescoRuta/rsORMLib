use std::collections::HashMap;

use syn::{Result, spanned::Spanned};

use crate::db_table_parse::{FromAttribute, RelationAttribute};

use super::get_attributes;


pub struct DbTableFrom<'a> {
	pub rs_type: &'a syn::Ident,
	pub from: String,
	pub table: String,
	pub joins: String,
}

fn get_table_from<'a>(input: &'a syn::DeriveInput, struct_attributes: &HashMap<String, &syn::Attribute>) -> Result<DbTableFrom<'a>> {
	let struct_from = *struct_attributes.get("from").ok_or(syn::Error::new(input.ident.span(), "from attribute must be specified"))?;
	let mut struct_from_attribute: FromAttribute = syn::parse(struct_from.tokens.clone().into())?;
	let from = struct_from_attribute.attr.ok_or(syn::Error::new(struct_from.span(), "A db table must have a from string"))?;
	let table = if let Some(table) = struct_from_attribute.named_arrs.remove("table") {
		table
	} else {
		from.clone()
	};
	let joins = if let Some(joins) = struct_from_attribute.named_arrs.remove("joins") {
		format!(" {}", joins)
	} else {
		"".to_string()
	};
	Ok(DbTableFrom {
		rs_type: &input.ident,
		from,
		table,
		joins,
	})
}


pub struct DbColumn<'a> {
	pub rs_name: String,
	pub rs_name_ident: &'a syn::Ident,
	pub db_name: String,
	pub rs_type: &'a syn::Type,
	pub from_attribute: Option<FromAttribute>,
	pub readonly: bool,
	pub attributes: HashMap<String, &'a syn::Attribute>,
}

pub struct DbRelation<'a> {
	pub rs_name: String,
	pub rs_name_ident: &'a syn::Ident,
	pub join_col: String,
	pub ty: &'a syn::Type,
	pub attributes: HashMap<String, &'a syn::Attribute>,
}

pub struct DbTable<'a> {
	pub from: DbTableFrom<'a>,
	pub pk: DbColumn<'a>,
	pub columns_except_pk: Vec<DbColumn<'a>>,
	pub relations: Vec<DbRelation<'a>>,
}

pub fn get_db_table<'a>(input: &'a syn::DeriveInput, struct_attributes: &HashMap<String, &syn::Attribute>, fields: &'a syn::FieldsNamed) -> Result<DbTable<'a>> {
	let from: DbTableFrom = get_table_from(input, struct_attributes)?;
	let mut pk: Option<DbColumn> = None;
	let mut columns_except_pk: Vec<DbColumn> = Vec::with_capacity(fields.named.len());
	let mut relations: Vec<DbRelation> = Vec::with_capacity(fields.named.len());
	for field in &fields.named {
		let attributes = get_attributes(field.attrs.iter());
		let rs_name = field.ident.as_ref().unwrap().to_string();
		let rs_name_ident = field.ident.as_ref().unwrap();
		let from_attribute: Option<FromAttribute> = if let Some(&a) = attributes.get("from") {
			Some(syn::parse(a.tokens.clone().into())?)
		} else {
			None
		};
		let db_name = if let Some(from_attribute) = &from_attribute {
			if let Some(db_name) = &from_attribute.attr {
				db_name.clone()
			} else {
				rs_name.clone()
			}
		} else {
			rs_name.clone()
		};
		if let Some(&_pk_attribute) = attributes.get("pk") {
			if pk.is_some() {
				return Err(syn::Error::new(input.ident.span(), "There must be only one primary key"));
			} else {
				if let Some(&ro) = attributes.get("readonly") {
					return Err(syn::Error::new(ro.path.span(), "Primary key can't be readonly"));
				}
				pk = Some(DbColumn {
					db_name,
					rs_name,
					rs_name_ident,
					from_attribute,
					readonly: false,
					attributes,
					rs_type: &field.ty
				});
			}
		} else if let Some(&relation_attribute) = attributes.get("relation") {
			let RelationAttribute { fk } = syn::parse(relation_attribute.tokens.clone().into())?;
			relations.push(DbRelation {
				rs_name,
				rs_name_ident,
				join_col: fk,
				ty: get_inner_type(&field.ty)?,
				attributes,
			});
		} else {
			let readonly = attributes.get("readonly").is_some();
			if !readonly {
				if let Some(from_attribute) = &from_attribute {
					if from_attribute.named_arrs.get("table").is_some() {
						return Err(syn::Error::new(rs_name_ident.span(), "Columns from other tables must be readonly"));
					}
				}
			}
			columns_except_pk.push(DbColumn {
				db_name,
				rs_name,
				rs_name_ident,
				attributes,
				readonly,
				from_attribute,
				rs_type: &field.ty
			});
		}
	}
	Ok(DbTable {
		from,
		pk: pk.ok_or(syn::Error::new(input.ident.span(), "There must be one primary key"))?,
		columns_except_pk,
		relations,
	})
}

pub fn get_inner_type(ty: &syn::Type) -> Result<&syn::Type> {
	let get_unsupported_type_err = || Err(syn::Error::new(ty.span(), "Unsupported type."));
	let ty = if let syn::Type::Path(ty) = ty {
		ty
	} else {
		return get_unsupported_type_err();
	};
	let generics = if let Some(last_arg) = ty.path.segments.iter().last() {
		if let syn::PathArguments::AngleBracketed(generics) = &last_arg.arguments {
			&generics.args
		} else {
			return get_unsupported_type_err();
		}
	} else {
		return get_unsupported_type_err();
	};
	if generics.len() != 1 {
		return get_unsupported_type_err();
	}
	if let Some(generic) = generics.last() {
		if let syn::GenericArgument::Type(generic) = generic {
			Ok(generic)
		} else {
			get_unsupported_type_err()
		}
	} else {
		unsafe { std::hint::unreachable_unchecked() }
	}
}