use std::collections::HashMap;
use std::time::Duration;

use leptos::prelude::*;
use leptos::server_fn::error::NoCustomError;
use reqwest::header;
use serde::{Deserialize, Serialize};
use web_sys::wasm_bindgen::prelude::Closure;

use crate::data::location::LocationManager;
use crate::data::shared_booking::TimeSlot;
use crate::utils::date::format_iso_date;
use crate::utils::geocoding::geocode_address;

use crate::pages::home::get_location_details;

#[component]
pub fn ExpandedLocationDetails(location_id: String, expanded: ReadSignal<bool>) -> impl IntoView {
    let (slots, set_slots) = create_signal(Vec::<TimeSlot>::new());
    let (is_loading, set_is_loading) = create_signal(false);
    let (error, set_error) = create_signal::<Option<String>>(None);

    let (location_etag, set_location_etag) = create_signal(String::new());

    let slots_by_date = create_memo(move |_| {
        let mut grouped: HashMap<String, Vec<TimeSlot>> = HashMap::new();

        for slot in slots.get().iter() {
            if slot.availability {
                if let Some(date_part) = slot.start_time.split_whitespace().next() {
                    let entry = grouped
                        .entry(date_part.to_string())
                        .or_insert_with(Vec::new);
                    entry.push(slot.clone());
                }
            }
        }

        let mut dates: Vec<_> = grouped.into_iter().collect();
        dates.sort_by(|(date_a, _), (date_b, _)| {
            let parts_a: Vec<&str> = date_a.split('/').collect();
            let parts_b: Vec<&str> = date_b.split('/').collect();

            if parts_a.len() == 3 && parts_b.len() == 3 {
                let year_compare = parts_a[2].cmp(parts_b[2]);
                if year_compare != std::cmp::Ordering::Equal {
                    return year_compare;
                }

                let month_compare = parts_a[1].cmp(parts_b[1]);
                if month_compare != std::cmp::Ordering::Equal {
                    return month_compare;
                }

                return parts_a[0].cmp(parts_b[0]);
            }

            date_a.cmp(date_b)
        });

        dates
    });

    create_effect(move |_| {
        if expanded.get() {
            let location_id_clone = location_id.clone();

            set_is_loading(true);
            set_error(None);

            leptos::task::spawn_local(async move {
                match get_location_details(location_id_clone, location_etag.get_untracked()).await {
                    Ok(response) => match response {
                        Some(response) => {
                            set_slots(response.slots);
                            set_location_etag(response.etag);
                        }
                        None => {}
                    },
                    Err(err) => {
                        set_error(Some(format!("Error loading details: {}", err)));
                    }
                }
                set_is_loading(false);
            });
        }
    });

    view! {
        <Show when=move || expanded.get()>
            <tr>
                <td colspan="5" class="px-6 py-4 bg-gray-50">
                    {move || {
                        if is_loading.get() {
                            view! {
                                <div class="flex justify-center items-center py-4">
                                    <div class="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500"></div>
                                </div>
                            }.into_any()
                        } else if let Some(err) = error.get() {
                            view! {
                                <div class="text-red-500 py-2">{err}</div>
                            }.into_any()
                        } else {
                            let dates = slots_by_date.get();

                            if dates.is_empty() {
                                view! {
                                    <div class="text-gray-500 py-2 text-center">No available slots</div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="max-h-80 overflow-y-auto">
                                        <h3 class="text-lg font-medium mb-2">Available Times</h3>
                                        <div class="space-y-4">
                                            {dates.into_iter().map(|(date, slots)| {
                                                view! {
                                                    <div class="border-b border-gray-200 pb-2">
                                                        <h4 class="font-medium text-gray-700 mb-1">{date}</h4>
                                                        <div class="flex flex-wrap gap-2">
                                                            {slots.into_iter().map(|slot| {
                                                                let time_only = slot.start_time
                                                                    .split_whitespace()
                                                                    .nth(1)
                                                                    .unwrap_or(&slot.start_time)
                                                                    .to_string();

                                                                view! {
                                                                    <span class="inline-block bg-green-100 text-green-800 px-2 py-1 text-sm rounded">
                                                                        {time_only}
                                                                    </span>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }
                    }}
                </td>
            </tr>
        </Show>
    }
}
