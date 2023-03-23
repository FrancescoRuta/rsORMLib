# MySQL rsORMLib draft

This is a simple MySQL async relational ORM library. Non intended for use, it's just an exercise.

## Usage example

```rs
#[derive(DbModel)]
#[from("items")]
#[fe_export]
pub struct Item {
	#[pk]
	id: Option<u32>,
	#[from("field1")]
	renamed_field1: u8,
	field2: i32,
	code: String,
	#[relation("fk__item_id")]
	bill_of_materials: Vec<BillOfMaterials>,
}

#[derive(DbModel)]
#[from("bill_of_materials")]
#[fe_export]
pub struct BillOfMaterials {
	#[pk]
	id: Option<u32>,
	#[from("description")]
	name: String,
	#[relation("id_formula")]
	items: Vec<BillOfMaterialsItem>,
}

#[derive(DbModel)]
#[from("bill_of_materials_items", joins = "LEFT JOIN items bill_of_materials_item ON bill_of_materials_items.fk__item_id=bill_of_materials_item.id")]
#[fe_export]
pub struct BillOfMaterialsItem {
	#[pk]
	id: Option<u32>,
	fk__item_id: i32,
	qty: f64,
	#[from(table = "bill_of_materials_item")]
	#[readonly]
	code: String,
	#[from(table = "bill_of_materials_item")]
	#[readonly]
	description: String,
}
```


```rs

...

let item = Item::get_by_pk(id, &mut conn).await?

...

let insert_id = item.exec_insert(&mut conn).await?;

...

let updated_item: Item = item.exec_update(&mut conn).await?;

...

```
