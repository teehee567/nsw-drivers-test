#![recursion_limit = "512"]
use std::fs::File;
use std::io::Read;

use axum::Router;
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use nsw_closest_display_lib::app::{shell, App};
use nsw_closest_display_lib::data::booking::BookingManager;
use nsw_closest_display_lib::data::location::Location;
use nsw_closest_display_lib::settings::Settings;

// FIX: HACKY
fn get_location_names() -> Vec<String> {
    fn parse_locations() -> Vec<Location> {
        let mut file = File::open("data/centres.json").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        serde_json::from_str(&contents).unwrap_or_else(|e| {
            log::error!("Failed to parse locations: {}", e);
            Vec::new()
        })
    }

    parse_locations()
        .into_iter()
        .map(|location| location.id.to_string())
        .collect()
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let data_file_path = "data/bookings.json";
    match BookingManager::init_from_file(data_file_path) {
        Ok(_) => println!("BookingManager initialized from file"),
        Err(e) => println!("Failed to initialize BookingManager from file: {}", e),
    }

    let settings = Settings::from_yaml("settings.yaml").unwrap();

    let location_id = get_location_names();

    if settings.scraping_enabled {
        BookingManager::start_background_updates(location_id, data_file_path.to_string(), settings);
    } else {
        println!("Scraping is disabled. Running in UI-only mode.");
    }

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
