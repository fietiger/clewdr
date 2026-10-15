use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::{
    api,
    i18n::use_i18n,
    types::{RequestEntry, TrafficStats},
    utils::format_timestamp,
};

/// The windows the chart can show, with the label key for each.
const WINDOWS: [(&str, &str); 3] = [
    ("1h", "traffic.window1h"),
    ("24h", "traffic.window24h"),
    ("7d", "traffic.window7d"),
];

/// How often the page refreshes itself while it is on screen.
const REFRESH_MS: u32 = 10_000;

/// Recent request activity: totals, a per-bucket bar chart, and a log.
///
/// The counts come from the proxy's own recorder, so only requests to the
/// inference endpoints are represented — the dashboard's polling is excluded
/// upstream, which is why the chart does not climb on its own refresh.
#[component]
pub fn TrafficPage() -> impl IntoView {
    let i18n = use_i18n();
    let window = RwSignal::new("24h".to_string());
    let stats = RwSignal::new(TrafficStats::default());
    let requests = RwSignal::new(Vec::<RequestEntry>::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);
    let refresh_trigger = RwSignal::new(0u32);

    let fetch = move || {
        loading.set(true);
        error.set(None);
        let selected = window.get_untracked();
        spawn_local(async move {
            match api::get_stats(&selected).await {
                Ok(s) => stats.set(s),
                Err(e) => error.set(Some(e)),
            }
            if let Ok(log) = api::get_requests(60).await {
                requests.set(log.requests);
            }
            loading.set(false);
        });
    };

    // Reload on mount, on window change, and on every manual or timed bump.
    Effect::new(move || {
        window.get();
        refresh_trigger.get();
        fetch();
    });

    // The interval is owned by this component, so switching tabs drops it
    // rather than leaving a timer running against an unmounted view.
    let interval = gloo_timers::callback::Interval::new(REFRESH_MS, move || {
        refresh_trigger.update(|v| *v += 1);
    });
    on_cleanup(move || drop(interval));

    view! {
        <div class="stack">
            <div class="overview">
                {move || {
                    let totals = stats.get().totals;
                    let i = use_i18n();
                    view! {
                        <div class="tile">
                            <div class="tile-value">{totals.total.to_string()}</div>
                            <div class="tile-label">{i.t("traffic.tiles.total")}</div>
                        </div>
                        <div class="tile">
                            <div class="tile-value tone-success">{totals.ok.to_string()}</div>
                            <div class="tile-label">{i.t("traffic.tiles.ok")}</div>
                        </div>
                        <div class="tile">
                            <div class="tile-value tone-danger">{totals.error.to_string()}</div>
                            <div class="tile-label">{i.t("traffic.tiles.error")}</div>
                        </div>
                    }
                }}
            </div>

            <Show when=move || error.get().is_some()>
                <div class="alert alert-error">{move || error.get().unwrap_or_default()}</div>
            </Show>

            <div class="section-title-row">
                <h2 class="section-title">{move || i18n.t("traffic.chartTitle")}</h2>
                <button
                    class="btn btn-ghost btn-sm"
                    disabled=move || loading.get()
                    on:click=move |_| refresh_trigger.update(|v| *v += 1)
                >
                    {move || if loading.get() {
                        i18n.t("cookieStatus.refreshing")
                    } else {
                        i18n.t("cookieStatus.refresh")
                    }}
                </button>
            </div>

            <div class="sub-tabs">
                {WINDOWS
                    .into_iter()
                    .map(|(name, key)| {
                        view! {
                            <button
                                class=move || {
                                    if window.get() == name { "sub-tab active" } else { "sub-tab" }
                                }
                                on:click=move |_| window.set(name.to_string())
                            >
                                {move || i18n.t(key)}
                            </button>
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>

            <div class="chart">
                {move || {
                    let buckets = stats.get().buckets;
                    // Integer percentages: a bar chart does not need float
                    // division, and this keeps the casts out of the code.
                    let max = buckets.iter().map(|b| b.total()).max().unwrap_or(0).max(1);
                    buckets
                        .into_iter()
                        .map(|bucket| {
                            let total = bucket.total();
                            let height = (total * 100) / max;
                            let error_share = if total == 0 {
                                0
                            } else {
                                (bucket.error * 100) / total
                            };
                            let title = format_timestamp(
                                i64::try_from(bucket.ts).unwrap_or_default(),
                            );
                            view! {
                                <div class="chart-col" title=title>
                                    <div class="chart-stack" style=format!("height:{height}%")>
                                        <div
                                            class="chart-seg error"
                                            style=format!("height:{error_share}%")
                                        ></div>
                                        <div class="chart-seg ok"></div>
                                    </div>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                }}
            </div>

            <p class="section-note">{move || {
                let secs = stats.get().bucket_seconds;
                i18n.tf("traffic.bucketNote", &[("seconds", &secs.to_string())])
            }}</p>

            <div class="section-title-row">
                <h2 class="section-title">{move || i18n.t("traffic.recentTitle")}</h2>
            </div>

            <div class="log-list">
                {move || {
                    let items = requests.get();
                    if items.is_empty() {
                        return view! {
                            <div class="log-empty">{i18n.t("traffic.noRequests")}</div>
                        }
                            .into_any();
                    }
                    items
                        .into_iter()
                        .map(|entry| {
                            let ok = entry.status < 400;
                            let status_class = if ok { "log-status ok" } else { "log-status err" };
                            let when = format_timestamp(i64::try_from(entry.ts).unwrap_or_default());
                            view! {
                                <div class="log-row">
                                    <span class=status_class>{entry.status.to_string()}</span>
                                    <span class="log-path text-mono truncate">{entry.path}</span>
                                    <span class="log-meta">{format!("{}ms", entry.latency_ms)}</span>
                                    <span class="log-meta log-when">{when}</span>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_any()
                }}
            </div>
        </div>
    }
}
