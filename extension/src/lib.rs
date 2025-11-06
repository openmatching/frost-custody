use dioxus::prelude::*;

mod components;
pub mod icons;
mod p2p;
mod peer_context;
mod services;

use components::{CreateWallet, SignTransaction, WalletView};
use peer_context::PeerContext;

#[derive(Clone, PartialEq)]
enum AppState {
    Welcome,
    SetupDevice,
    CreateWallet, // Coordinator mode
    JoinWallet,   // Follower mode
    WalletView,
    SignTransaction,
}

#[derive(Clone, PartialEq)]
pub enum Role {
    Coordinator, // Creates wallet, aggregates signatures
    Follower,    // Joins wallet, follows commands
}

#[component]
fn App() -> Element {
    let mut state = use_signal(|| AppState::Welcome);

    // Global P2P context
    use_context_provider(|| PeerContext::new());

    rsx! {
        div { class: "min-h-screen bg-gray-50 p-4",
            match state() {
                AppState::Welcome => rsx! { WelcomeScreen {
                    on_start: move |_| state.set(AppState::SetupDevice)
                } },
                AppState::SetupDevice => rsx! { SetupDevice {
                    on_create: move |_| state.set(AppState::CreateWallet),
                    on_join: move |_| state.set(AppState::JoinWallet),
                } },
                AppState::CreateWallet => rsx! { CreateWallet {
                    role: Role::Coordinator,
                    on_complete: move |_| state.set(AppState::WalletView)
                } },
                AppState::JoinWallet => rsx! { CreateWallet {
                    role: Role::Follower,
                    on_complete: move |_| state.set(AppState::WalletView)
                } },
                AppState::WalletView => rsx! { WalletView { on_sign: move |_| state.set(AppState::SignTransaction) } },
                AppState::SignTransaction => rsx! { SignTransaction { on_complete: move |_| state.set(AppState::WalletView) } },
            }
        }
    }
}

#[component]
fn WelcomeScreen(on_start: EventHandler<()>) -> Element {
    rsx! {
        div { class: "max-w-md mx-auto mt-20 p-8 bg-white rounded-lg shadow-lg",
            div { class: "text-center mb-8",
                h1 { class: "text-3xl font-bold text-gray-900 mb-2",
                    "FROST Wallet"
                }
                p { class: "text-gray-600",
                    "Peer-to-peer MPC wallet"
                }
            }

            div { class: "space-y-4",
                button {
                    class: "w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 transition",
                    onclick: move |_| on_start.call(()),
                    "Get Started"
                }

                button {
                    class: "w-full bg-gray-200 text-gray-700 py-3 px-4 rounded-lg hover:bg-gray-300 transition",
                    "Recover from Backup"
                }
            }

            div { class: "mt-8 pt-6 border-t border-gray-200",
                p { class: "text-sm text-gray-500 text-center",
                    "Open source • No custody • Self-sovereign"
                }
            }
        }
    }
}

#[component]
fn SetupDevice(on_create: EventHandler<()>, on_join: EventHandler<()>) -> Element {
    let peer_ctx = use_context::<PeerContext>();
    let mut device_name = use_signal(|| String::new());

    rsx! {
        div { class: "max-w-md mx-auto mt-20 p-8 bg-white rounded-lg shadow-lg",
            h2 { class: "text-2xl font-bold text-gray-900 mb-6", "Setup Device" }

            p { class: "text-gray-600 mb-6",
                "First, give this device a name. Then choose to create a new wallet or join an existing one."
            }

            div { class: "mb-6",
                label { class: "block text-sm font-medium text-gray-700 mb-2",
                    "Device Name"
                }
                input {
                    class: "w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                    r#type: "text",
                    placeholder: "My Laptop, Phone, etc.",
                    value: "{device_name}",
                    oninput: move |e| {
                        let name = e.value().clone();
                        device_name.set(name.clone());
                        peer_ctx.set_device_name(name);
                    }
                }
                p { class: "mt-2 text-xs text-gray-500",
                    "This helps identify your device when signing with multiple devices"
                }
            }

            div { class: "space-y-3 pt-6 border-t border-gray-200",
                button {
                    class: "w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 transition disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: device_name().trim().is_empty(),
                    onclick: move |_| on_create.call(()),
                    div { class: "flex items-center justify-center",
                        icons::Crown { class: Some("w-5 h-5 mr-2".to_string()) }
                        span { "Create New Wallet (Coordinator)" }
                    }
                }

                button {
                    class: "w-full bg-green-600 text-white py-3 px-4 rounded-lg hover:bg-green-700 transition disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: device_name().trim().is_empty(),
                    onclick: move |_| on_join.call(()),
                    div { class: "flex items-center justify-center",
                        icons::Users { class: Some("w-5 h-5 mr-2".to_string()) }
                        span { "Join Existing Wallet (Follower)" }
                    }
                }
            }

            div { class: "mt-6 bg-blue-50 border border-blue-200 rounded-lg p-4",
                div { class: "flex items-start",
                    icons::Lightbulb { class: Some("w-4 h-4 text-blue-600 mr-2 mt-0.5".to_string()) }
                    p { class: "text-xs text-blue-800",
                        "Coordinator manages the wallet creation. Followers join and participate in signing."
                    }
                }
            }
        }
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run() {
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("FROST Wallet Extension starting...");
    dioxus::launch(App);
}
