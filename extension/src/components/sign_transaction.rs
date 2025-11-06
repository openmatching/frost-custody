use dioxus::prelude::*;

#[component]
pub fn SignTransaction(on_complete: EventHandler<()>) -> Element {
    let mut to_address = use_signal(|| String::new());
    let mut amount = use_signal(|| String::new());
    let mut signing = use_signal(|| false);

    rsx! {
        div { class: "max-w-md mx-auto mt-10 p-6 bg-white rounded-lg shadow-lg",
            h2 { class: "text-2xl font-bold text-gray-900 mb-6", "Send Transaction" }

            if !signing() {
                div { class: "space-y-4",
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-2",
                            "To Address"
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                            r#type: "text",
                            placeholder: "0x...",
                            value: "{to_address}",
                            oninput: move |e| to_address.set(e.value().clone())
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-2",
                            "Amount (ETH)"
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                            r#type: "text",
                            placeholder: "0.0",
                            value: "{amount}",
                            oninput: move |e| amount.set(e.value().clone())
                        }
                    }

                    div { class: "bg-gray-50 border border-gray-200 rounded-lg p-4",
                        div { class: "flex justify-between text-sm mb-2",
                            span { class: "text-gray-600", "Network Fee" }
                            span { class: "text-gray-900", "~0.001 ETH" }
                        }
                        div { class: "flex justify-between text-sm",
                            span { class: "text-gray-600", "Total" }
                            span { class: "font-medium text-gray-900", "~0.101 ETH" }
                        }
                    }

                    div { class: "flex space-x-4 pt-4",
                        button {
                            class: "flex-1 bg-gray-200 text-gray-700 py-3 px-4 rounded-lg hover:bg-gray-300 transition",
                            onclick: move |_| on_complete.call(()),
                            "Cancel"
                        }
                        button {
                            class: "flex-1 bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 transition",
                            onclick: move |_| signing.set(true),
                            "Sign Transaction"
                        }
                    }
                }
            } else {
                div { class: "space-y-6",
                    div { class: "text-center",
                        div { class: "animate-spin w-12 h-12 border-4 border-blue-600 border-t-transparent rounded-full mx-auto mb-4" }
                        h3 { class: "text-lg font-medium text-gray-900 mb-2",
                            "Coordinating FROST Signature"
                        }
                        p { class: "text-sm text-gray-600",
                            "Connecting to peers via libp2p..."
                        }
                    }

                    div { class: "space-y-2",
                        div { class: "flex items-center text-sm",
                            span { class: "w-2 h-2 bg-green-600 rounded-full mr-2" }
                            "Round 1: Generating commitments"
                        }
                        div { class: "flex items-center text-sm text-gray-400",
                            span { class: "w-2 h-2 bg-gray-300 rounded-full mr-2" }
                            "Round 2: Signature shares"
                        }
                        div { class: "flex items-center text-sm text-gray-400",
                            span { class: "w-2 h-2 bg-gray-300 rounded-full mr-2" }
                            "Broadcasting transaction"
                        }
                    }

                    button {
                        class: "w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 transition",
                        onclick: move |_| on_complete.call(()),
                        "Complete (Test)"
                    }
                }
            }
        }
    }
}
