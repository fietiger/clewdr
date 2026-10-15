use js_sys::Date;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use super::{
    auth::AuthGatekeeper, config_tab::ConfigTab, cookie::CookieVisualization,
    cookie_submit::CookieSubmitForm, traffic::TrafficPage,
};
use crate::{
    api,
    i18n::{self, Locale, use_i18n},
    storage,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Claude,
    Traffic,
    Config,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClaudeSubTab {
    Submit,
    Status,
}

#[component]
pub fn App() -> impl IntoView {
    i18n::provide_i18n();

    let version = RwSignal::new(String::new());
    let is_authenticated = RwSignal::new(false);
    let active_tab = RwSignal::new(Tab::Claude);
    let claude_sub = RwSignal::new(ClaudeSubTab::Submit);
    let toast = RwSignal::new(Option::<(String, bool)>::None);

    provide_context(toast);

    spawn_local(async move {
        if let Ok(v) = api::get_version().await {
            version.set(v);
        }
    });

    spawn_local(async move {
        if let Some(token) = storage::get("authToken")
            && !token.is_empty()
        {
            match api::validate_auth(&token).await {
                Ok(true) => is_authenticated.set(true),
                _ => storage::remove("authToken"),
            }
        }
    });

    // Init theme from localStorage
    {
        let theme = storage::get("theme").unwrap_or_else(|| "dark".into());
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let _ = doc
                .document_element()
                .unwrap()
                .set_attribute("data-theme", &theme);
        }
    }

    let password_changed = web_sys::window()
        .and_then(|w| {
            let search = w.location().search().ok()?;
            let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
            let changed = params.get("passwordChanged").as_deref() == Some("true");
            if changed {
                let _ = w.history().ok()?.replace_state_with_url(
                    &wasm_bindgen::JsValue::NULL,
                    "",
                    Some("/"),
                );
            }
            Some(changed)
        })
        .unwrap_or(false);

    let i18n = use_i18n();

    view! {
        <div>
            <Toast toast />

            <header class="topbar">
                <div class="topbar-inner">
                    <div class="brand">
                        <span class="brand-mark">"C"</span>
                        <span class="brand-name">"ClewdR"</span>
                        <span class="text-xs text-mute" style="white-space:pre-line">
                            {move || version.get()}
                        </span>
                    </div>
                    <div class="row-sm">
                        <ThemeToggle />
                        <LanguageSelector />
                    </div>
                </div>
            </header>

            <Show
                when=move || is_authenticated.get()
                fallback=move || {
                    view! { <LoginPage password_changed is_authenticated /> }
                }
            >
                {move || match active_tab.get() {
                    Tab::Claude => view! {
                        <section class="page page-active" role="tabpanel">
                            <header class="page-title">
                                <h1>"🔑 " {move || i18n.t("tabs.claude")}</h1>
                                <p class="subtitle">{move || i18n.t("page.claudeSubtitle")}</p>
                            </header>
                            <SubTabBar sub_tab=claude_sub />
                            {move || match claude_sub.get() {
                                ClaudeSubTab::Submit => view! { <CookieSubmitForm /> }.into_any(),
                                ClaudeSubTab::Status => view! { <CookieVisualization /> }.into_any(),
                            }}
                        </section>
                    }.into_any(),
                    Tab::Traffic => view! {
                        <section class="page page-active" role="tabpanel">
                            <header class="page-title">
                                <h1>"📈 " {move || i18n.t("tabs.traffic")}</h1>
                                <p class="subtitle">{move || i18n.t("page.trafficSubtitle")}</p>
                            </header>
                            <TrafficPage />
                        </section>
                    }.into_any(),
                    Tab::Config => view! {
                        <section class="page page-active" role="tabpanel">
                            <header class="page-title">
                                <h1>"⚙️ " {move || i18n.t("tabs.config")}</h1>
                                <p class="subtitle">{move || i18n.t("page.configSubtitle")}</p>
                            </header>
                            <ConfigTab />
                            <SessionPanel is_authenticated />
                        </section>
                    }.into_any(),
                }}
            </Show>

            <footer class=move || {
                if is_authenticated.get() { "footer footer-raised" } else { "footer" }
            }>
                {move || {
                    let year = Date::new_0().get_full_year().to_string();
                    use_i18n().tf("app.footer", &[("year", &year)])
                }}
            </footer>

            <Show when=move || is_authenticated.get()>
                <TabBar active_tab />
            </Show>
        </div>
    }
}

#[component]
fn Toast(toast: RwSignal<Option<(String, bool)>>) -> impl IntoView {
    view! {
        <Show when=move || toast.get().is_some()>
            {move || {
                let (msg, ok) = toast.get().unwrap();
                let cls = if ok { "toast toast-success" } else { "toast toast-error" };
                view! {
                    <div class=cls>
                        {msg}
                        <button class="toast-close" on:click=move |_| toast.set(None)>"×"</button>
                    </div>
                }
            }}
        </Show>
    }
}

#[component]
fn LoginPage(password_changed: bool, is_authenticated: RwSignal<bool>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <section class="page page-active">
            <header class="page-title">
                <h1>"🔐 " {move || i18n.t("auth.title")}</h1>
                <p class="subtitle">{move || i18n.t("auth.description")}</p>
            </header>
            <div class="card">
                <Show when=move || password_changed>
                    <div class="alert alert-info" style="margin-bottom:1rem">
                        {move || i18n.t("auth.passwordChanged")}
                    </div>
                </Show>
                <AuthGatekeeper is_authenticated />
            </div>
        </section>
    }
}

#[component]
fn SessionPanel(is_authenticated: RwSignal<bool>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <h2 class="section-title">{move || i18n.t("auth.authTitle")}</h2>
        <div class="group">
            <div class="row row-stack">
                <div class="row-label">{move || i18n.t("auth.loggedInMessage")}</div>
            </div>
        </div>
        <div class="actions page-actions">
            <button
                class="btn btn-danger"
                on:click=move |_| {
                    storage::remove("authToken");
                    is_authenticated.set(false);
                }
            >
                {move || i18n.t("auth.logout")}
            </button>
        </div>
    }
}

#[component]
fn ThemeToggle() -> impl IntoView {
    let is_dark = RwSignal::new(storage::get("theme").as_deref() != Some("light"));

    let toggle = move |_| {
        let dark = !is_dark.get_untracked();
        is_dark.set(dark);
        let theme = if dark { "dark" } else { "light" };
        storage::set("theme", theme);
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let _ = doc
                .document_element()
                .unwrap()
                .set_attribute("data-theme", theme);
        }
    };

    view! {
        <button
            class="lang-btn off"
            title=move || if is_dark.get() { "Switch to light" } else { "Switch to dark" }
            on:click=toggle
        >
            {move || if is_dark.get() { "☾" } else { "☀" }}
        </button>
    }
}

#[component]
fn LanguageSelector() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="row-sm">
            <button
                class=move || if i18n.locale() == Locale::En { "lang-btn on" } else { "lang-btn off" }
                on:click=move |_| i18n.set_locale(Locale::En)
            >"EN"</button>
            <button
                class=move || if i18n.locale() == Locale::Zh { "lang-btn on" } else { "lang-btn off" }
                on:click=move |_| i18n.set_locale(Locale::Zh)
            >"中文"</button>
        </div>
    }
}

#[component]
fn TabBar(active_tab: RwSignal<Tab>) -> impl IntoView {
    let i18n = use_i18n();
    let tabs = [
        (Tab::Claude, "🔑", "tabs.claude"),
        (Tab::Traffic, "📈", "tabs.traffic"),
        (Tab::Config, "⚙️", "tabs.config"),
    ];
    view! {
        <nav class="tab-bar" role="tablist">
            {tabs.into_iter().map(|(tab, icon, key)| {
                view! {
                    <button
                        class=move || {
                            if active_tab.get() == tab { "tab tab-active" } else { "tab" }
                        }
                        role="tab"
                        on:click=move |_| active_tab.set(tab)
                    >
                        <span class="tab-icon">{icon}</span>
                        <span>{move || i18n.t(key)}</span>
                    </button>
                }
            }).collect::<Vec<_>>()}
        </nav>
    }
}

#[component]
fn SubTabBar(sub_tab: RwSignal<ClaudeSubTab>) -> impl IntoView {
    let i18n = use_i18n();
    let tabs = [
        (ClaudeSubTab::Submit, "claudeTab.submit"),
        (ClaudeSubTab::Status, "claudeTab.status"),
    ];
    view! {
        <div class="sub-tabs">
            {tabs.into_iter().map(|(tab, key)| {
                view! {
                    <button
                        class=move || if sub_tab.get() == tab { "sub-tab active" } else { "sub-tab" }
                        on:click=move |_| sub_tab.set(tab)
                    >
                        {move || i18n.t(key)}
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
