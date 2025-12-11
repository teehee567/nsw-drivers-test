use chrono::{DateTime, Utc};
use leptos::prelude::*;

pub fn format_iso_date(iso_string: &str) -> String {
    if let Ok(datetime) = DateTime::parse_from_rfc3339(iso_string) {
        return datetime.format("%d %b %Y, %H:%M UTC").to_string();
    } else {
        iso_string.to_string()
    }
}

#[cfg(not(feature = "ssr"))]
pub fn format_iso_date_local(iso_string: &str) -> String {
    use wasm_bindgen::prelude::*;
    use web_sys::js_sys;

    if let Ok(_) = DateTime::parse_from_rfc3339(iso_string) {
        let date = js_sys::Date::new(&JsValue::from_str(iso_string));

        if !date.get_time().is_nan() {
            let options = js_sys::Object::new();
            js_sys::Reflect::set(&options, &"year".into(), &"numeric".into()).unwrap();
            js_sys::Reflect::set(&options, &"month".into(), &"short".into()).unwrap();
            js_sys::Reflect::set(&options, &"day".into(), &"2-digit".into()).unwrap();
            js_sys::Reflect::set(&options, &"hour".into(), &"2-digit".into()).unwrap();
            js_sys::Reflect::set(&options, &"minute".into(), &"2-digit".into()).unwrap();
            js_sys::Reflect::set(&options, &"hour12".into(), &false.into()).unwrap();

            return date
                .to_locale_time_string_with_options("en-AU", &options)
                .into();
        }
    }
    iso_string.to_string()
}

#[component]
pub fn TimeDisplay(
    #[prop(into)] iso_time: String,
    #[prop(optional)] class: Option<String>,
) -> impl IntoView {
    let (display_time, set_display_time) = create_signal(format_iso_date(&iso_time));

    #[cfg(not(feature = "ssr"))]
    create_effect(move |_| {
        set_display_time(format_iso_date_local(&iso_time));
    });

    view! {
        <span class={class.unwrap_or_default()}>
            {display_time}
        </span>
    }
}
