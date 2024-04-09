use mysql_async_orm::{db_connection::DbConnectionPool, DbModel};

#[derive(DbModel, Debug)]
#[from("articoli")]
pub struct Articolo {
	#[pk]
	id: Option<usize>,
	#[from(expression = "articoli.unita_misura")]
	#[readonly]
	unita_di_misura: u8,
	classificazione: i32,
	codice: String,
	descrizione: String,
	codice_ean: String,
	descrizione_estesa: String,
	note: String,
	scorta_minima: f64,
	#[from("giorni_riordino")]
	giorni_di_riordino: i32,
	qta_minima_ordinabile: f64,
	altezza: f64,
	larghezza: f64,
	profondita: f64,
	colli_per_bancale: i32,
	pezzi_per_blister: i32,
	formato_flacone: f64,
	listino_imponibile: f64,
	#[from("prezzo_minimo_vendita_imponibile")]
	prezzo_minimo_di_vendita_imponibile: f64,
	#[from("prezzo_minimo_vendita2_imponibile")]
	prezzo_cessione_minimo_di_vendita_imponibile: f64,
	#[from("prezzo_minimo_vendita_promo_imponibile")]
	prezzo_cessione_minimo_di_vendita_promo_imponibile: f64,
	private_label: bool,
	#[from("distinte_nascoste")]
	distinte_nascoste_in_produzione: bool,
	tipo_controllo_qualita: u8,
	parametri_controllo_qualita: String,
	#[relation("id_articolo_prodotto")]
	distinte_base: Vec<DistintaBase>,
}

#[derive(DbModel, Debug)]
#[from("produzione__formule")]
pub struct DistintaBase {
	#[pk]
	id: Option<usize>,
	#[from("descrizione")]
	nome: String,
	#[relation("id_formula")]
	articoli: Vec<ArticoloDistintaBase>,
}
//joins = [articoli articolo_distinta on "produzione__distinta_base.id_articolo=articolo_distinta.id"]
#[derive(DbModel, Debug)]
#[from("produzione__distinta_base", joins = r#"articoli articolo_distinta "produzione__distinta_base.id_articolo=articolo_distinta.id", unita_di_misura unita_di_misura_articolo_distinta "articolo_distinta.unita_misura=unita_di_misura_articolo_distinta.id""#)]
pub struct ArticoloDistintaBase {
	#[pk]
	id: Option<usize>,
	id_articolo: i32,
	qta: f64,
	#[from("simbolo", table = "unita_di_misura_articolo_distinta")]
	#[readonly]
	unita_di_misura: String,
	#[from(table = "articolo_distinta")]
	#[readonly]
	codice: String,
	#[from(table = "articolo_distinta")]
	#[readonly]
	descrizione: String,
	#[from("ultimo_costo", table = "articolo_distinta")]
	#[readonly]
	costo: f64,
}

#[tokio::main]
async fn main() {
	let pool = DbConnectionPool::new("mysql://application:@localhost:3306/chimiclean?pool_min=16&pool_max=256");
	let mut conn = pool.get_conn().await.unwrap();
	
	
	let articolo = Articolo::get_by_pk(1, &mut conn).await.unwrap();
	println!("{:#?}", articolo)
}