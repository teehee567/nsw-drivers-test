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

use crate::pages::home::LocationBookingViewModel;

use crate::pages::location_row::LocationRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortColumn {
    Name,
    Distance,
    EarliestSlot,
    PassRate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortDirection {
    Ascending,
    Descending,
}

#[component]
fn SortableHeader(
    column: SortColumn,
    current_sort: ReadSignal<SortColumn>,
    sort_direction: ReadSignal<SortDirection>,
    on_sort: impl Fn(SortColumn) + 'static,
    title: &'static str,
    mobile_title: Option<&'static str>,
) -> impl IntoView {
    let sort_icon = move || {
        if current_sort.get() == column {
            match sort_direction.get() {
                SortDirection::Ascending => "↑\u{FE0E}",
                SortDirection::Descending => "↓\u{FE0E}",
            }
        } else {
            "↕\u{FE0E}"
        }
    };

    view! {
        <th class="px-1 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
            <button
                class="flex items-center gap-1 hover:text-gray-700 transition-colors"
                on:click=move |_| on_sort(column)
            >
                {move || {
                    if let Some(mobile) = mobile_title {
                        view! {
                            <>
                                <span class="hidden md:inline">{title}</span>
                                <span class="md:hidden">{mobile}</span>
                            </>
                        }.into_any()
                    } else {
                        view! {
                            <span>{title}</span>
                        }.into_any()
                    }
                }}
                <span class="text-gray-400 font-sans" style="font-variant-emoji: text;">{sort_icon}</span>
            </button>
        </th>
    }
}

#[component]
pub fn LocationsTable(
    bookings: ReadSignal<Vec<LocationBookingViewModel>>,
    is_loading: ReadSignal<bool>,
    latitude: ReadSignal<f64>,
    longitude: ReadSignal<f64>,
    location_manager: LocationManager,
    reset_sort_trigger: ReadSignal<()>,
) -> impl IntoView {
    let booking_map = create_memo(move |_| {
        bookings
            .get()
            .into_iter()
            .map(|booking| (booking.location.clone(), booking.earliest_slot))
            .collect::<HashMap<String, Option<TimeSlot>>>()
    });

    let (sort_column, set_sort_column) = create_signal(SortColumn::Distance);
    let (sort_direction, set_sort_direction) = create_signal(SortDirection::Ascending);

    create_effect(move |_| {
        reset_sort_trigger.get();
        set_sort_column(SortColumn::Distance);
        set_sort_direction(SortDirection::Ascending);
    });

    let handle_sort_click = move |new_column: SortColumn| {
        let current_column = sort_column.get();
        if current_column == new_column {
            set_sort_direction.update(|dir| {
                *dir = match dir {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                }
            });
        } else {
            set_sort_column(new_column);
            set_sort_direction(SortDirection::Ascending);
        }
    };

    let sorted_locations = create_memo(move |_| {
        let mut locations_by_distance =
            location_manager.get_by_distance(latitude.get(), longitude.get());
        let booking_data = booking_map.get();
        let column = sort_column.get();
        let direction = sort_direction.get();

        let mut locations_with_data: Vec<_> = locations_by_distance
            .into_iter()
            .map(|(loc, distance)| {
                let location_id = loc.id.to_string();
                let earliest_slot = booking_data.get(&location_id).cloned().flatten();
                (loc, distance, earliest_slot)
            })
            .collect();

        locations_with_data.sort_by(|a, b| {
            let ordering = match column {
                SortColumn::Name => a.0.name.cmp(&b.0.name),
                SortColumn::Distance => a.1.total_cmp(&b.1),
                SortColumn::EarliestSlot => match (&a.2, &b.2) {
                    (Some(slot_a), Some(slot_b)) => slot_a.cmp(&slot_b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                },
                SortColumn::PassRate => {
                    b.0.pass_rate
                        .partial_cmp(&a.0.pass_rate)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
            };

            match direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });

        locations_with_data
    });

    view! {
        <div>
            <div class="md:hidden flex justify-center items-center bg-blue-50 p-3 mb-3 rounded-lg border border-blue-200">
                <div class="flex items-center gap-2 text-sm text-blue-800">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                        <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd" />
                    </svg>
                    <span>Tap any location to view available time slots</span>
                </div>
            </div>

            <div class="hidden md:flex mb-3 text-sm text-gray-600 bg-blue-50 p-3 rounded-md items-center gap-2 border border-blue-200">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-blue-500" viewBox="0 0 20 20" fill="currentColor">
                    <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd" />
                </svg>
                <span>Click on any row to view available time slots for that location</span>
            </div>
            <div class="overflow-x-auto">
                <table class="min-w-full bg-white border border-gray-200 rounded-lg overflow-hidden table-fixed">
                    <colgroup>
                        <col style="width: 15%;" />
                        <col style="width: 12%;" />
                        <col style="width: 28%;" />
                        <col style="width: 15%;" />
                        <col style="width: 10%;" />
                    </colgroup>
                    <thead class="bg-gray-50">
                        <tr>
                            <SortableHeader
                                column=SortColumn::Name
                                current_sort=sort_column
                                sort_direction=sort_direction
                                on_sort=handle_sort_click
                                title="Name"
                                mobile_title=None
                            />
                            <SortableHeader
                                column=SortColumn::Distance
                                current_sort=sort_column
                                sort_direction=sort_direction
                                on_sort=handle_sort_click
                                title="Distance"
                                mobile_title=Some("Dist")
                            />
                            <SortableHeader
                                column=SortColumn::EarliestSlot
                                current_sort=sort_column
                                sort_direction=sort_direction
                                on_sort=handle_sort_click
                                title="Earliest Slot"
                                mobile_title=Some("Slot")
                            />
                            <SortableHeader
                                column=SortColumn::PassRate
                                current_sort=sort_column
                                sort_direction=sort_direction
                                on_sort=handle_sort_click
                                title="Pass Rate"
                                mobile_title=Some("Pass %")
                            />
                            <th class="px-1 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">
                                <span class="sr-only">Details</span>
                            </th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-gray-200">
                        {move || {
                            let locations_data = sorted_locations.get();

                            locations_data.into_iter().map(|(loc, distance, earliest_slot)| {
                                view! {
                                    <LocationRow
                                        loc=loc
                                        distance=distance
                                        earliest_slot=earliest_slot
                                        is_loading=is_loading
                                    />
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
