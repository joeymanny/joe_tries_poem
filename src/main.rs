use poem::{listener::TcpListener, Route, Server};
use serde::Serialize;
use sqlx::{Pool, Postgres};
use tracing::trace;
use std::{ collections::HashMap, sync::OnceLock};

lazy_static::lazy_static! {
    static ref ENDPOINTS: HashMap<usize, &'static str> = HashMap::from([
        (0_usize, "/index"),
    ]);
}

static POOL: OnceLock<Pool<Postgres>> = OnceLock::new();

#[derive(Serialize, Debug, Clone)]
pub struct ListItem { listitem: Option<String>}

#[poem::handler]
async fn get_list() -> poem::web::Json<Vec<ListItem>>{
    let row: Vec<ListItem> = sqlx::query_as::<_, (String, )>("SELECT * FROM thelist").fetch_all(POOL.get().unwrap()).await.unwrap()
        .into_iter().map(|v| ListItem{ listitem: Some(v.0) }).collect();
    poem::web::Json(row)
}

#[poem::handler]
async fn post_list(data: String) -> poem::web::Html<String> {
    match format_data(data) {
        Some(v) => {
            // POOL
            sqlx::query("INSERT INTO thelist (listitem) VALUES ($1);").bind(&v).execute(POOL.get().unwrap()).await.unwrap();
            poem::web::Html(format!(include_str!("./success.html"), v))
        },
        None => poem::web::Html(include_str!("./failure.html").to_owned())
    }
}

fn format_data(data: String) -> Option<String>{
    let best_fit = data.split('&')
        .find(|v| v[0.."listitem=".len()] == *"listitem=");
    match best_fit{
        Some(mut list_item) =>{
            list_item = &list_item["listitem=".len()..]; // trim 
            if list_item.is_empty() { None } else {
                let list_item = list_item.replace('+', " ");
                let list_item = url_escape::decode(&list_item).into();
                Some(list_item)
            }
        },
        None => {
            None
        },
    }
}

#[poem::handler]
fn index() -> poem::web::Html<&'static str>{
    poem::web::Html(include_str!("./index.html"))
}


#[tokio::main]
async fn main() -> Result<(), std::io::Error>{
    color_eyre::install().unwrap();
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE/DEBUG/INFO/WARN/ERROR (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(tracing::Level::TRACE)
        // completes the builder.
        .finish()
    )
        .expect("setting default subscriber failed");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://joe@localhost:5432/thelist").await.unwrap();

    let _ = POOL.set(pool); // safety: we must be the first to set POOL

    // Make a simple query to return the given parameter (use a question mark `?` instead of `$1` for MySQL)
    let row: Vec<ListItem> = sqlx::query_as::<_, (String, )>("SELECT * FROM thelist").fetch_all(POOL.get().unwrap()).await.unwrap()
        .into_iter().map(|v| ListItem{ listitem: Some(v.0) }).collect();



    trace!("query result: {:?}", row);

// database setup done

    // let api_service =
    //     OpenApiService::new(MyApiStruct, "Hello World Service", "1.0");
    let app = Route::new().at("/thelist", poem::get(index)).at("/thelist/getlist", poem::get(get_list)).at("/thelist/postlist", poem::post(post_list));

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}