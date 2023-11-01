//! The main program of my website.

#![deny(clippy::missing_docs_in_private_items)]
// #![deny(clippy::unwrap_in_result)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(warnings)]

mod id_gen;
use id_gen::*;

use once_cell::sync::Lazy;
use rocket::form::FromForm;
use rocket::{
    get, http::ContentType, launch, post, routes, serde::json::Json, tokio::sync::RwLock, Config,
    Route,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::vec;
use tokio::spawn;

/// a macro to define a route.
macro_rules! raw_files {
    {$($route:expr => $name:ident($content_type:ident, $path:expr),)*} => {
        $(
            #[doc = "a fonction to serve the file"]
            #[get($route)]
            const fn $name() -> (ContentType, &'static str) {
                (ContentType::$content_type, include_str!($path))
            }
        )*

        fn raw_routes() -> Vec<Route> {
            routes![$($name),*]
        }
    };
}

raw_files! {
    "/wave" => wave(SVG, "webpages/wave.svg"),
    "/style" => style(CSS, "webpages/style.css"),
    "/" => visualize(HTML, "test.html"),
}

/// a struct to store articles.
static ARTICLES: Lazy<RwLock<HashMap<u64, Article>>> = Lazy::new(|| RwLock::new(HashMap::new()));

/// The struct to represent an article.
#[derive(Serialize, Deserialize, Clone, Debug, FromForm)]
struct Article {
    /// The name of the article.
    title: String,
    /// The introduction of the article.
    intro: String,
    /// The content of the article.
    content: String,
}
/// a function to get an article with its id.
#[get("/article/<id>")]
async fn get_article(id: u64) -> Option<Json<Article>> {
    // On récupère l'accès aux articles qui sont dans un RwLock puis,
    // on récupère l'article et si il existe on le convertis en Json
    // sinon on renvoit None ce qui a pour effet de faire une erreur 404
    ARTICLES
        .read()
        .await
        .get(&id)
        .map(|article| Json(article.clone()))
}

/// a function to list all articles.
#[get("/articles")]
async fn list_articles() -> Json<Vec<u64>> {
    Json(ARTICLES.read().await.keys().copied().collect())
}

/// a function to add an article.
#[post("/new_article", data = "<article>")]
async fn add_article(article: Option<rocket::form::Form<Article>>) {
    if let Some(article) = article {
        let article = article.into_inner();
        // On cree une nouvelle id.
        let id = IdGenerator::with_used_ids(ARTICLES.read().await.keys().copied().collect())
            .generate_unique_id();

        // On stocke l'article dans un la static.
        ARTICLES.write().await.insert(id, article);

        let handle = spawn(async {
            // on ecrit sur le fichier la nouvelle id.
            let mut file = File::create("articles.json").expect("Failed to create articles.json");

            // on fait le json
            let json_content = serde_json::to_string_pretty(&*ARTICLES.read().await);
            json_content.map_or_else(
                |e| {
                    println!("error: {:?}", e);
                },
                |json_content| {
                    file.write_all(json_content.as_bytes())
                        .expect("Failed to write articles.json");
                },
            )
        })
        .await;
        if let Err(e) = handle {
            println!("error: {:?}", e);
        }
    } else {
        println!("error: POST request without same data");
    }
}

/// The main function of the website.
#[launch]
async fn rocket() -> _ {
    let file = File::open("articles.json").expect("Failed to open articles.json");
    // on affichie si il y a une erreur
    let articles: HashMap<u64, Article> =
        serde_json::from_reader(file).expect("Failed to read articles");
    ARTICLES.write().await.extend(articles.into_iter());

    rocket::build()
        .configure(Config {
            port: 80,
            ..Default::default()
        })
        .mount("/", raw_routes())
        .mount("/api", routes![get_article, list_articles, add_article])
}
