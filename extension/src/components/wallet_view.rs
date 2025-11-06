use dioxus::prelude::*;
use crate::icons;

#[component]
pub fn WalletView(on_sign: EventHandler<()>) -> Element {
    let balance = use_signal(|| String::from("0.0"));
    let address = use_signal(|| String::from("0x0000...0000"));
    
    rsx! {
        div { class: "max-w-md mx-auto mt-10 p-6 bg-white rounded-lg shadow-lg",
            // Header
            div { class: "flex items-center justify-between mb-6",
                h1 { class: "text-2xl font-bold text-gray-900", "Wallet" }
                button { class: "text-gray-400 hover:text-gray-600",
                    icons::Settings { class: Some("w-5 h-5".to_string()) }
                }
            }
            
            // Balance card
            div { class: "bg-gradient-to-br from-blue-500 to-blue-700 rounded-lg p-6 text-white mb-6",
                p { class: "text-sm opacity-80 mb-2", "Total Balance" }
                h2 { class: "text-4xl font-bold mb-4",
                    "{balance} ETH"
                }
                div { class: "flex items-center space-x-2 text-sm",
                    span { class: "opacity-80", "Address:" }
                    span { class: "font-mono", "{address}" }
                    button { class: "opacity-80 hover:opacity-100",
                        icons::Copy { class: Some("w-3 h-3".to_string()) }
                    }
                }
            }
            
            // Actions
            div { class: "grid grid-cols-2 gap-4 mb-6",
                button {
                    class: "bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 transition",
                    onclick: move |_| on_sign.call(()),
                    "Send"
                }
                button {
                    class: "bg-gray-100 text-gray-700 py-3 px-4 rounded-lg hover:bg-gray-200 transition",
                    "Receive"
                }
            }
            
            // Transaction list
            div { class: "space-y-4",
                h3 { class: "text-sm font-medium text-gray-500 uppercase", "Recent Transactions" }
                
                div { class: "text-center py-8 text-gray-400",
                    p { "No transactions yet" }
                }
            }
            
            // Status
            div { class: "mt-6 pt-6 border-t border-gray-200",
                div { class: "flex items-center justify-between text-sm",
                    span { class: "text-gray-500", "Network" }
                    span { class: "text-green-600 flex items-center",
                        span { class: "w-2 h-2 bg-green-600 rounded-full mr-2" }
                        "Ethereum Mainnet"
                    }
                }
                div { class: "flex items-center justify-between text-sm mt-2",
                    span { class: "text-gray-500", "Threshold" }
                    span { class: "text-gray-900", "2-of-3" }
                }
            }
        }
    }
}

