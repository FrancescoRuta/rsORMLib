use mysql_async_orm::{db_connection::DbConnectionPool, DbTable};

mod db_date;
use db_date::*;

#[derive(DbTable, Debug)]
#[from("clienti")]
pub struct Cliente {
	#[pk]
	id: Option<u32>,
	ragione_sociale: String,
	partita_iva: String,
	codice_sdi: String,
	indirizzo: String,
	sede: u32,
	email: String,
	pec: String,
	tel1: String,
	tel2: String,
	sito_web: String,
	gruppo: u32,
	codice: u32,
	provvigione_agente: f64,
	#[relation("id_cliente")]
	premi: Vec<Premio>,
	#[relation("id_cliente")]
	metodi_di_pagamento: Vec<MetodoDiPagamento>,
	#[relation("id_cliente")]
	scontistiche: Vec<Scontistica>,
	#[relation("id_cliente")]
	spese_di_trasporto: Vec<SpeseDiTrasporto>,
	#[relation("id_cliente")]
	listing: Vec<Listing>,
	#[relation("id_cliente")]
	referenti: Vec<Referente>,
	#[relation("id_cliente")]
	agenti: Vec<Agente>,
	#[relation("id_cliente")]
	articoli_trattati: Vec<ArticoloTrattato>,
	#[relation("id_cliente")]
	sedi: Vec<Sede>,
}

#[derive(DbTable, Debug)]
#[from("clienti__premi")]
pub struct Premio {
	#[pk]
	id: Option<u32>,
	data_inizio: DBDate,
	data_fine: DBDate,
	importo_minimo: f64,
	premio: f64,
}

#[derive(DbTable, Debug)]
#[from("clienti__metodi_di_pagamento")]
pub struct MetodoDiPagamento {
	#[pk]
	id: Option<u32>,
	id_metodo_di_pagamento: u32,
	sconto_aggiuntivo: f64,
}

#[derive(DbTable, Debug)]
#[from("clienti__sconti")]
pub struct Scontistica {
	#[pk]
	id: Option<u32>,
	data_inizio: DBDate,
	data_fine: DBDate,
	sconto1: f64,
	sconto2: f64,
	sconto3: f64,
	sconto4: f64,
}

#[derive(DbTable, Debug)]
#[from("clienti__spese_trasporto")]
pub struct SpeseDiTrasporto {
	#[pk]
	id: Option<u32>,
	data_inizio: DBDate,
	data_fine: DBDate,
	porto: u32,
}

#[derive(DbTable, Debug)]
#[from("clienti__listing")]
pub struct Listing {
	#[pk]
	id: Option<u32>,
	data_inizio: DBDate,
	data_fine: DBDate,
	importo: f64,
	cadenza_erogazione: u32,
}

#[derive(DbTable, Debug)]
#[from("clienti__referenti")]
pub struct Referente {
	#[pk]
	id: Option<u32>,
	nome: String,
	ruolo: u32,
	telefono: String,
	cellulare: String,
	email: String,
}

#[derive(DbTable, Debug)]
#[from("clienti__agenti", joins = "LEFT JOIN agenti ON clienti__agenti.id_agente=agenti.id")]
pub struct Agente {
	#[pk]
	id: Option<u32>,
	id_agente: u32,
}

#[derive(DbTable, Debug)]
#[from("clienti__articoli", joins = "LEFT JOIN articoli ON clienti__articoli.id_articolo=articoli.id LEFT JOIN unita_di_misura ON articoli.unita_misura=unita_di_misura.id")]
pub struct ArticoloTrattato {
	#[pk]
	id: Option<u32>,
	id_articolo: u32,
	applica_listino_personalizzato: bool,
	listino_personalizzato: f64,
	provvigione_agente: f64,
	#[from("simbolo", table = "unita_di_misura")]
	#[readonly]
	#[allow(dead_code)]
	unita_di_misura: String,
	#[from(table = "articoli")]
	#[readonly]
	#[allow(dead_code)]
	codice: String,
	#[from(table = "articoli")]
	#[readonly]
	#[allow(dead_code)]
	descrizione: String,
}

#[derive(DbTable, Debug)]
#[from("clienti__sedi")]
pub struct Sede {
	#[pk]
	id: Option<u32>,
	comune: u32,
	descrizione: String,
	cap: u32,
	indirizzo: String,
	iban: String,
	banca: String,
	codice: u32,
	abi: u32,
	cad: u32,
}

#[tokio::main]
async fn main() {
	let pool = DbConnectionPool::new("mysql://application:@localhost:3306/chimiclean?pool_min=16&pool_max=256");
	let mut conn = pool.get_conn().await.unwrap();
	
	
	let mut cliente = Cliente::get_by_pk(1, &mut conn).await.unwrap();
	println!("{:#?}", cliente);
	
	cliente.id = None;
	cliente.premi.iter_mut().for_each(|v| v.id = None);
	cliente.metodi_di_pagamento.iter_mut().for_each(|v| v.id = None);
	cliente.scontistiche.iter_mut().for_each(|v| v.id = None);
	cliente.spese_di_trasporto.iter_mut().for_each(|v| v.id = None);
	cliente.listing.iter_mut().for_each(|v| v.id = None);
	cliente.referenti.iter_mut().for_each(|v| v.id = None);
	cliente.agenti.iter_mut().for_each(|v| v.id = None);
	cliente.articoli_trattati.iter_mut().for_each(|v| v.id = None);
	cliente.sedi.iter_mut().for_each(|v| v.id = None);
	
	cliente.ragione_sociale.push_str(" - clone");
	let id = cliente.exec_insert(&mut conn).await.unwrap();
	println!("{}", id);
	
	
	let cliente = Cliente::get_by_pk(id, &mut conn).await.unwrap();
	println!("{:#?}", cliente);
	
}