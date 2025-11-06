// Lucide icon components for Dioxus
// SVG icons inlined for better performance

use dioxus::prelude::*;

#[component]
pub fn Crown(class: Option<String>) -> Element {
    rsx! {
        svg {
            class: "{class.unwrap_or_default()}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M11.562 3.266a.5.5 0 0 1 .876 0L15.39 8.87a1 1 0 0 0 1.516.294L21.183 5.5a.5.5 0 0 1 .798.519l-2.834 10.246a1 1 0 0 1-.956.734H5.81a1 1 0 0 1-.957-.734L2.02 6.02a.5.5 0 0 1 .798-.519l4.276 3.664a1 1 0 0 0 1.516-.294z" }
            path { d: "M5 21h14" }
        }
    }
}

#[component]
pub fn Users(class: Option<String>) -> Element {
    rsx! {
        svg {
            class: "{class.unwrap_or_default()}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" }
            circle { cx: "9", cy: "7", r: "4" }
            path { d: "M22 21v-2a4 4 0 0 0-3-3.87" }
            path { d: "M16 3.13a4 4 0 0 1 0 7.75" }
        }
    }
}

#[component]
pub fn Smartphone(class: Option<String>) -> Element {
    rsx! {
        svg {
            class: "{class.unwrap_or_default()}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            rect { width: "14", height: "20", x: "5", y: "2", rx: "2", ry: "2" }
            path { d: "M12 18h.01" }
        }
    }
}

#[component]
pub fn Radio(class: Option<String>) -> Element {
    rsx! {
        svg {
            class: "{class.unwrap_or_default()}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M4.9 19.1C1 15.2 1 8.8 4.9 4.9" }
            path { d: "M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5" }
            circle { cx: "12", cy: "12", r: "2" }
            path { d: "M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5" }
            path { d: "M19.1 4.9C23 8.8 23 15.1 19.1 19" }
        }
    }
}

#[component]
pub fn CheckCircle(class: Option<String>) -> Element {
    rsx! {
        svg {
            class: "{class.unwrap_or_default()}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "12", r: "10" }
            path { d: "m9 12 2 2 4-4" }
        }
    }
}

#[component]
pub fn Copy(class: Option<String>) -> Element {
    rsx! {
        svg {
            class: "{class.unwrap_or_default()}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            rect { width: "14", height: "14", x: "8", y: "8", rx: "2", ry: "2" }
            path { d: "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" }
        }
    }
}

#[component]
pub fn AlertCircle(class: Option<String>) -> Element {
    rsx! {
        svg {
            class: "{class.unwrap_or_default()}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "12", r: "10" }
            line { x1: "12", x2: "12", y1: "8", y2: "12" }
            line { x1: "12", x2: "12.01", y1: "16", y2: "16" }
        }
    }
}

#[component]
pub fn Lightbulb(class: Option<String>) -> Element {
    rsx! {
        svg {
            class: "{class.unwrap_or_default()}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5" }
            path { d: "M9 18h6" }
            path { d: "M10 22h4" }
        }
    }
}

#[component]
pub fn Settings(class: Option<String>) -> Element {
    rsx! {
        svg {
            class: "{class.unwrap_or_default()}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" }
            circle { cx: "12", cy: "12", r: "3" }
        }
    }
}

