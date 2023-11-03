//! The main program of my website.

use once_cell::sync::Lazy;
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::serde::uuid::Uuid;
use rocket::{
    get, http::ContentType, launch, post, routes, serde::json::Json, tokio::sync::RwLock, Config,
    Route,
};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
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
    "/background" => wave(SVG, "webpages/background.svg"),
    "/style" => style(CSS, "webpages/style.css"),
    "/" => main_page(HTML, "webpages/main.html"),
    "/about_me" => aboute_me_page(HTML, "webpages/about_me.html"),
    "/articles" => articles_page(HTML, "webpages/articles.html"),
    "/article/<_>" => article_page(HTML, "webpages/article.html"),
}

#[get("/favicon.ico")]
const fn favicon() -> (ContentType, &'static [u8]) {
    let ico_data: &'static [u8] = include_bytes!("webpages/logo.ico");
    let content_type = ContentType::Icon;

    (content_type, ico_data)
}

/// a struct to store articles.
static ARTICLES: Lazy<RwLock<HashMap<Uuid, Article>>> = Lazy::new(|| RwLock::new(HashMap::new()));

/// The struct to represent an article.
#[derive(Serialize, Deserialize, Clone, Debug, FromForm)]
struct Article {
    /// The name of the article.
    title: String,
    /// The introduction of the article.
    intro: String,
    /// The content of the article.
    content: Vec<String>,
    /// The path of the image of the article.
    image_path: String,
}

/// a function to get an article with its id.
#[get("/article/<id>")]
async fn get_article(id: Uuid) -> Option<Json<Article>> {
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
async fn list_articles() -> Json<Vec<Uuid>> {
    Json(ARTICLES.read().await.keys().copied().collect())
}

/// a function to add an article.
#[post("/new_article", data = "<article>")]
async fn add_article(cookies: &CookieJar<'_>, article: Option<rocket::form::Form<Article>>) {
    if !is_admin(cookies) {
        println!("error: POST request without grade admin");
    } else if let Some(article) = article {
        let article = article.into_inner();
        // On cree une nouvelle id.
        let id = Uuid::new_v4();

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

#[derive(FromForm)]
struct Upload<'f> {
    file: TempFile<'f>,
}

#[post("/upload", format = "multipart/form-data", data = "<form>")]
async fn upload_image(mut form: Form<Upload<'_>>) -> std::io::Result<()> {
    let file_name: Option<&str> = form.file.name();
    if let Some(name) = file_name {
        let file_name = String::from("static/") + name + ".png";

        form.file.persist_to(file_name).await?;

        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to get file name",
        ))
    }
}

/// a function to get an image.
#[get("/image/<path>")]
fn get_image(path: String) -> (ContentType, File) {
    // on envoie l'image si il existe sinon on envoie une erreur 404
    let path = format!("static/{}", path);
    if let Ok(file) = File::open(path) {
        (ContentType::PNG, file)
    } else {
        (ContentType::HTML, File::open("webpages/404.html").unwrap())
    }
}

/// a function to login as admin.
#[get("/login/<password>")]
async fn login_admin(cookies: &CookieJar<'_>, password: String) -> Redirect {
    if sha1_hash(&password) == "5ed25af7b1ed23fb00122e13d7f74c4d8262acd8" {
        // on ajoute le cookie prive
        cookies.add_private(Cookie::new("admin", "true"));
        Redirect::to("/admin")
    } else {
        Redirect::to("/admin/login")
    }
}

#[get("/")]
fn admin_main(cookies: &CookieJar<'_>) -> (ContentType, &'static str) {
    if is_admin(cookies) {
        (ContentType::HTML, include_str!("webpages/new_article.html"))
    } else {
        (ContentType::HTML, include_str!("webpages/login.html"))
    }
}

/// a function to hash a string.
fn sha1_hash(input: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

/// a fonction to get if the user is admin.
fn is_admin(cookies: &CookieJar<'_>) -> bool {
    cookies
        .get_private("admin")
        .map_or(false, |cookie| cookie.value() == "true")
}

/// a function to check if cookie is admin.
#[get("/check_admin")]
fn check_admin(cookies: &CookieJar<'_>) -> String {
    if is_admin(cookies) {
        "true".to_string()
    } else {
        "false".to_string()
    }
}

/// The main function of the website.
#[launch]
async fn rocket() -> _ {
    let file = File::open("articles.json").expect("Failed to open articles.json");
    // on affichie si il y a une erreur
    let articles: HashMap<Uuid, Article> =
        serde_json::from_reader(file).expect("Failed to read articles");
    ARTICLES.write().await.extend(articles.into_iter());

    rocket::build()
        .configure(Config {
            port: 80,
            ..Default::default()
        })
        .mount("/", raw_routes())
        .mount("/", routes![favicon])
        .mount(
            "/api",
            routes![
                get_article,
                list_articles,
                add_article,
                upload_image,
                get_image
            ],
        )
        .mount("/admin", routes![login_admin, check_admin, admin_main])
}
