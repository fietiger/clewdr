mod row;
mod usage;

use leptos::{ev, prelude::*};
use row::{ExhaustedRow, InvalidRow, ValidRow};
use wasm_bindgen_futures::spawn_local;

use crate::{api, i18n::use_i18n, types::CookieStatusInfo};

#[component]
pub fn CookieVisualization() -> impl IntoView {
    let i18n = use_i18n();
    let data = RwSignal::new(CookieStatusInfo::default());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let force_refreshing = RwSignal::new(false);
    let refresh_trigger = RwSignal::new(0u32);

    provide_context(refresh_trigger);

    let fetch = move |force: bool| {
        loading.set(true);
        error.set(None);
        if force {
            force_refreshing.set(true);
        }
        spawn_local(async move {
            match api::get_cookies(force).await {
                Ok(info) => data.set(info),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
            force_refreshing.set(false);
        });
    };

    let _effect = Effect::new(move || {
        refresh_trigger.get();
        fetch(false);
    });

    let on_refresh = move |ev: ev::MouseEvent| {
        if ev.ctrl_key() || ev.meta_key() {
            fetch(true);
        } else {
            refresh_trigger.update(|v| *v += 1);
        }
    };

    view! {
        <div class="stack">
            <div class="overview">
                {move || {
                    let d = data.get();
                    let i = use_i18n();
                    let total = d.valid.len() + d.exhausted.len() + d.invalid.len();
                    view! {
                        <div class="tile">
                            <div class="tile-value">{total.to_string()}</div>
                            <div class="tile-label">{i.t("cookieStatus.tiles.total")}</div>
                        </div>
                        <div class="tile">
                            <div class="tile-value tone-success">{d.valid.len().to_string()}</div>
                            <div class="tile-label">{i.t("cookieStatus.tiles.valid")}</div>
                        </div>
                        <div class="tile">
                            <div class="tile-value tone-warn">{d.exhausted.len().to_string()}</div>
                            <div class="tile-label">{i.t("cookieStatus.tiles.exhausted")}</div>
                        </div>
                        <div class="tile">
                            <div class="tile-value tone-danger">{d.invalid.len().to_string()}</div>
                            <div class="tile-label">{i.t("cookieStatus.tiles.invalid")}</div>
                        </div>
                    }
                }}
            </div>

            <Show when=move || error.get().is_some()>
                <div class="alert alert-error">
                    {move || error.get().unwrap_or_default()}
                </div>
            </Show>

            <div class="section-title-row">
                <h2 class="section-title">{move || i18n.t("cookieStatus.pool")}</h2>
                <button
                    class="btn btn-ghost btn-sm"
                    title=move || i18n.t("cookieStatus.refreshTooltip")
                    disabled=move || loading.get()
                    on:click=on_refresh
                >
                    {move || match (loading.get(), force_refreshing.get()) {
                        (true, true) => i18n.t("cookieStatus.forceRefreshing"),
                        (true, false) => i18n.t("cookieStatus.refreshing"),
                        _ => i18n.t("cookieStatus.refresh"),
                    }}
                </button>
            </div>

            {move || {
                let d = data.get();
                let i = use_i18n();
                view! {
                    <CookieSection
                        title=i.t("cookieStatus.sections.valid")
                        color="valid"
                        count=d.valid.len()
                    >
                        {d.valid.into_iter().map(|c| view! { <ValidRow cookie=c /> }).collect::<Vec<_>>()}
                    </CookieSection>
                    <CookieSection
                        title=i.t("cookieStatus.sections.exhausted")
                        color="exhausted"
                        count=d.exhausted.len()
                    >
                        {d.exhausted.into_iter().map(|c| view! { <ExhaustedRow cookie=c /> }).collect::<Vec<_>>()}
                    </CookieSection>
                    <CookieSection
                        title=i.t("cookieStatus.sections.invalid")
                        color="invalid"
                        count=d.invalid.len()
                    >
                        {d.invalid.into_iter().map(|c| view! { <InvalidRow cookie=c /> }).collect::<Vec<_>>()}
                    </CookieSection>
                }
            }}
        </div>
    }
}

#[component]
fn CookieSection(
    title: String,
    color: &'static str,
    count: usize,
    children: Children,
) -> impl IntoView {
    let section_class = format!("cookie-section cookie-section-{color}");
    let title_class = format!("section-title section-title-{color}");
    let empty = count == 0;
    view! {
        <div class=section_class>
            <h4 class=title_class>{format!("{title} ({count})")}</h4>
            {if empty {
                view! { <p class="section-empty">{use_i18n().t("cookieStatus.noCookies")}</p> }.into_any()
            } else {
                view! { <div>{children()}</div> }.into_any()
            }}
        </div>
    }
}
