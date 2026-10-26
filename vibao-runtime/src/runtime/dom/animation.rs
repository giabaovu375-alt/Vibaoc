use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{Element, Event};

pub(super) fn bind_animations(el: &Element) {
    if let Some(raw) = el.get_attribute("data-vb-anim-hover") {
        bind_hover_animation(el, &raw);
    }
    if let Some(raw) = el.get_attribute("data-vb-anim-scroll") {
        bind_scroll_animation(el, &raw);
    }
}

pub(super) fn hover_animation_class_name(anim_name: &str) -> String {
    format!("vb-hover-{}", anim_name)
}

pub(super) fn bind_hover_animation(el: &Element, raw: &str) {
    let mut parts = raw.splitn(2, ':');
    let anim_name = match parts.next() {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => return,
    };
    let class_name = hover_animation_class_name(&anim_name);

    let el_enter = el.clone();
    let class_enter = class_name.clone();
    let enter_closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
        let _ = el_enter.class_list().add_1(&class_enter);
    });
    let target_enter: web_sys::EventTarget = el.clone().into();
    let _ = target_enter
        .add_event_listener_with_callback("mouseenter", enter_closure.as_ref().unchecked_ref());
    enter_closure.forget();

    let el_leave = el.clone();
    let leave_closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
        let _ = el_leave.class_list().remove_1(&class_name);
    });
    let target_leave: web_sys::EventTarget = el.clone().into();
    let _ = target_leave
        .add_event_listener_with_callback("mouseleave", leave_closure.as_ref().unchecked_ref());
    leave_closure.forget();
}

pub(super) fn bind_scroll_animation(el: &Element, raw: &str) {
    let anim_name = match raw.splitn(3, ':').next() {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => return,
    };
    let class_name = format!("vb-anim-{}", anim_name);

    let el_clone = el.clone();
    let observer_cell: std::rc::Rc<std::cell::RefCell<Option<web_sys::IntersectionObserver>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let observer_cell_for_closure = observer_cell.clone();

    let closure = Closure::<dyn FnMut(js_sys::Array)>::new(move |entries: js_sys::Array| {
        for i in 0..entries.length() {
            let entry = entries.get(i);
            let Ok(entry) = entry.dyn_into::<web_sys::IntersectionObserverEntry>() else {
                continue;
            };
            if entry.is_intersecting() {
                let _ = el_clone.class_list().add_1(&class_name);
                if let Some(obs) = observer_cell_for_closure.borrow().as_ref() {
                    obs.disconnect();
                }
            }
        }
    });

    let observer = match web_sys::IntersectionObserver::new(closure.as_ref().unchecked_ref()) {
        Ok(o) => o,
        Err(_) => {
            closure.forget();
            return;
        }
    };
    observer.observe(el);
    *observer_cell.borrow_mut() = Some(observer);
    closure.forget();
}
