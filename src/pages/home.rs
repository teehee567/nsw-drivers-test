use std::collections::HashMap;
use std::time::Duration;

use leptos::prelude::*;
use leptos::server_fn::error::NoCustomError;
use reqwest::header;
use serde::{Deserialize, Serialize};
use web_sys::wasm_bindgen::prelude::Closure;

use crate::data::location::LocationManager;
use crate::data::shared_booking::TimeSlot;
use crate::pages::location_table::LocationsTable;
use crate::utils::date::TimeDisplay;
use crate::utils::geocoding::geocode_address;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationBookingViewModel {
    pub location: String,
    pub earliest_slot: Option<TimeSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingResponse {
    pub bookings: Vec<LocationBookingViewModel>,
    pub last_updated: Option<String>,
    pub etag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationDetailBookingResponse {
    pub location: String,
    pub slots: Vec<TimeSlot>,
    pub etag: String,
}

#[server(GetBookings)]
pub async fn get_location_bookings(
    client_etag: String,
) -> Result<Option<BookingResponse>, ServerFnError> {
    use crate::data::booking::BookingManager;
    use axum::http::HeaderValue;
    use axum::http::StatusCode;

    let response = expect_context::<leptos_axum::ResponseOptions>();

    let (booking_data, server_etag) = BookingManager::get_data();
    if client_etag == server_etag {
        // WARN: for some reason this makes it open in hte browser
        // response.set_status(StatusCode::NOT_MODIFIED);
        return Ok(None);
    }

    let view_models: Vec<_> = booking_data
        .results
        .iter()
        .map(|location_booking| {
            let earliest_slot = location_booking
                .slots
                .iter()
                .filter(|slot| slot.availability)
                .min_by(|a, b| a.start_time.cmp(&b.start_time))
                .cloned();

            LocationBookingViewModel {
                location: location_booking.location.clone(),
                earliest_slot,
            }
        })
        .collect();

    Ok(Some(BookingResponse {
        bookings: view_models,
        last_updated: booking_data.last_updated.clone(),
        etag: server_etag,
    }))
}

#[server(GetLocationDetails)]
pub async fn get_location_details(
    location_id: String,
    client_etag: String,
) -> Result<Option<LocationDetailBookingResponse>, ServerFnError> {
    use crate::data::booking::BookingManager;

    let (location_booking, server_etag) = BookingManager::get_location_data(location_id).ok_or(
        ServerFnError::<NoCustomError>::ServerError("Location not found".into()),
    )?;

    if client_etag == server_etag {
        // WARN: for some reason this makes it open in hte browser
        // response.set_status(StatusCode::NOT_MODIFIED);
        return Ok(None);
    }

    Ok(Some(LocationDetailBookingResponse {
        location: location_booking.location,
        slots: location_booking.slots,
        etag: server_etag,
    }))
}

#[component]
pub fn HomePage() -> impl IntoView {
    let (address_input, set_address_input) = create_signal(String::new());
    let (latitude, set_latitude) = create_signal(-33.8688197);
    let (longitude, set_longitude) = create_signal(151.2092955);
    let (current_location_name, set_current_location_name) = create_signal("Sydney".to_string());
    let (geocoding_status, set_geocoding_status) = create_signal::<Option<String>>(None);
    let (is_loading, set_is_loading) = create_signal(false);

    let (last_updated, set_last_updated) = create_signal::<Option<String>>(None);

    let (bookings, set_bookings) = create_signal(Vec::<LocationBookingViewModel>::new());
    let (is_fetching_bookings, set_is_fetching_bookings) = create_signal(false);

    let (booking_etag, set_booking_etag) = create_signal(String::new());

    let (reset_sort_trigger, set_reset_sort_trigger) = create_signal(());

    let location_manager = LocationManager::new();

    let fetch_bookings = move || {
        set_is_fetching_bookings(true);

        leptos::task::spawn_local(async move {
            match get_location_bookings(booking_etag.get_untracked()).await {
                Ok(data) => {
                    match data {
                        Some(data) => {
                            set_bookings(data.bookings);
                            set_last_updated(data.last_updated);
                            set_booking_etag(data.etag);
                        }
                        None => {}
                    };
                }
                Err(err) => {
                    leptos::logging::log!("Error fetching bookings: {:?}", err);
                }
            }
            set_is_fetching_bookings(false);
        });
    };

    #[cfg(not(feature = "ssr"))]
    fetch_bookings();

    #[cfg(not(feature = "ssr"))]
    Effect::new(move |_| {
        leptos::logging::log!("Setting up client-side refresh mechanism");

        let handle = set_interval_with_handle(
            move || {
                leptos::logging::log!("Triggering refresh");
                fetch_bookings();
            },
            Duration::from_secs(600),
        )
        .expect("failed to set interval");

        on_cleanup(move || {
            handle.clear();
        });

        || {}
    });

    let handle_geocode = move |_| {
        let address = address_input.get();
        if address.is_empty() {
            set_geocoding_status(Some("Please enter a location".to_string()));
            return;
        }

        set_geocoding_status(Some("Searching...".to_string()));
        set_is_loading(true);

        leptos::task::spawn_local(async move {
            match geocode_address(&address).await {
                Ok(result) => {
                    set_latitude(result.latitude);
                    set_longitude(result.longitude);
                    set_current_location_name(result.display_name);
                    set_geocoding_status(None);
                    set_is_loading(false);
                    set_reset_sort_trigger(());
                }
                Err(err) => {
                    set_geocoding_status(Some(format!("Error: {}", err)));
                    set_is_loading(false);
                }
            }
        });
    };

    use leptos::wasm_bindgen::JsCast;
    use web_sys::Geolocation;

    #[cfg(not(feature = "ssr"))]
    {
        create_effect(move |_| {
            if let Some(window) = web_sys::window() {
                if let Ok(geolocation) = window.navigator().geolocation() {
                    let success_callback = Closure::<dyn FnMut(web_sys::Position)>::new(
                        move |position: web_sys::Position| {
                            set_latitude(position.coords().latitude());
                            set_longitude(position.coords().longitude());
                            set_address_input(format!(
                                "{}, {}",
                                position.coords().latitude(),
                                position.coords().longitude()
                            ));
                            handle_geocode(());
                        },
                    );

                    let _ =
                        geolocation.get_current_position(success_callback.as_ref().unchecked_ref());

                    success_callback.forget();
                }
            }
        });
    }

    view! {
        <div class="max-w-4xl mx-auto p-4">
            <div class="bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded relative mb-6" role="alert">
                <strong class="font-bold">UP as of 29/01/2026: </strong>
                <span class="block sm:inline">Currently up, refresh times will go down as i continue monitoring</span>
            </div>

            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold text-gray-800">NSW Available Drivers Tests</h2>
            </div>

            <div class="mb-6">
                <div class="flex flex-wrap gap-4 items-end">
                    <div class="flex flex-col flex-grow">
                        <label for="address" class="text-sm font-medium text-gray-700 mb-1">
                            Search by Postcode, Address, or Suburb:
                        </label>
                        <input
                            id="address"
                            type="text"
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                            placeholder="e.g., Sydney, 2000, 42 Wallaby Way"
                            prop:value={address_input}
                            on:input=move |ev| set_address_input(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" {
                                    handle_geocode(());
                                }
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500 italic">Your search is securely processed through nominatim.org, a trusted open-source geolocation service. No personal or identifying information is shared during this process.</p>
                    </div>
                </div>

                <div class="flex items-center gap-4 mt-2 w-full">
                    <button
                        class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors"
                        on:click=move |_| handle_geocode(())
                    >
                        Search
                    </button>

                    <div class="ml-auto text-sm text-gray-500">
                        {move || match last_updated.get() {
                            Some(time) => view! {
                                <span>"Data last updated: " <TimeDisplay iso_time={time} /></span>
                            }.into_any(),
                            None => view! { <span>"Data last updated: unknown"</span> }.into_any(),
                        }}
                    </div>
                </div>

                <div class="mt-2">
                    {move || {
                        match geocoding_status.get() {
                            Some(status) => view! {
                                <div class="text-sm mt-2 text-amber-600">
                                    {status}
                                </div>
                            }.into_any(),
                            None => view! { <div class="hidden"></div> }.into_any()
                        }
                    }}
                </div>

                <div class="mt-4 flex flex-wrap gap-4 items-end">
                    <div class="flex flex-wrap gap-4">
                        <div class="flex flex-col">
                            <label class="text-sm font-medium text-gray-700 mb-1">Current Coordinates:</label>
                            <div class="text-sm text-gray-600">
                                {move || format!("Lat: {:.6}, Lng: {:.6}", latitude.get(), longitude.get())}
                            </div>
                        </div>

                        <div class="flex flex-col">
                            <label class="text-sm font-medium text-gray-700 mb-1">Location:</label>
                            <div class="text-sm text-gray-600 max-w-md truncate">
                                {move || current_location_name.get()}
                            </div>
                        </div>
                    </div>
                </div>

                <p class="mt-1 text-xs text-gray-500 italic">
                  "Disclaimer: Pass rates shown are calculated based on the "
                  <span class="text-amber-600">center</span> " of the customer's local government
                  area (LGA) and weighted according to proximity to nearby testing centers. "
                  <span class="text-amber-600">These rates are estimates only.</span>
                  " Data is from 2022-2025 C Class Driver tests."
                </p>
            </div>

            <LocationsTable
                bookings=bookings
                is_loading=is_fetching_bookings
                latitude=latitude
                longitude=longitude
                location_manager=location_manager.clone()
                reset_sort_trigger=reset_sort_trigger
            />

            <div class="mt-6 flex justify-between items-center">
                <div class="text-sm text-gray-500">
                    <p>Location search results are made using "https://nominatim.org/" and are always done on your browser, your location information never touches our servers</p>
                    <p>Note: Distances are calculated using the Haversine formula and represent "as the crow flies" distance.</p>
                    <p>You can support me by giving me a github star</p>
                </div>

                <div class="flex gap-2">
                    <a
                        href="https://github.com/teehee567/nsw-drivers-test"
                        target="_blank"
                        class="px-3 py-1.5 bg-gray-800 text-white rounded-md hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 transition-colors inline-flex items-center justify-center gap-2"
                    >
                        <i class="fab fa-github"></i>
                        <span>View on GitHub</span>
                    </a>
                </div>
            </div>
        </div>
    }
}
