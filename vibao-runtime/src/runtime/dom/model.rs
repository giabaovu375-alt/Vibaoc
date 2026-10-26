use super::ATTR_MODEL;
use crate::runtime::{
    state::{self, LoopFrame, SharedState},
    value::VbValue,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{Element, Event};

pub(super) fn bind_model(
    shared: &SharedState,
    el: &Element,
    _loop_frame: Option<&LoopFrame>,
    _component_id: Option<&str>,
) {
    let key = match el.get_attribute(ATTR_MODEL) {
        Some(k) => k,
        None => return,
    };

    if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
        let input_type = input.type_();
        if input_type == "checkbox" {
            let input_clone = input.clone();
            let key_clone = key.clone();
            state::subscribe(
                shared,
                Box::new(move |sh: &SharedState| {
                    let v = state::get_tracked(sh, &key_clone);
                    input_clone.set_checked(v.is_truthy());
                }),
            );

            let shared_clone = shared.clone();
            let input_for_listener = input.clone();
            let key_for_listener = key.clone();
            let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                let checked = input_for_listener.checked();
                shared_clone
                    .borrow_mut()
                    .set_state(&key_for_listener, VbValue::bool(checked));
                state::flush(&shared_clone);
            });
            let target: web_sys::EventTarget = input.into();
            let _ =
                target.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
            closure.forget();
            return;
        }
        if input_type == "radio" {
            let own_value = input.value();
            let input_clone = input.clone();
            let key_clone = key.clone();
            let own_value_clone = own_value.clone();
            state::subscribe(
                shared,
                Box::new(move |sh: &SharedState| {
                    let v = state::get_tracked(sh, &key_clone);
                    input_clone.set_checked(v.to_string() == own_value_clone);
                }),
            );

            let shared_clone = shared.clone();
            let input_for_listener = input.clone();
            let key_for_listener = key.clone();
            let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                if input_for_listener.checked() {
                    shared_clone
                        .borrow_mut()
                        .set_state(&key_for_listener, VbValue::str(input_for_listener.value()));
                    state::flush(&shared_clone);
                }
            });
            let target: web_sys::EventTarget = input.into();
            let _ =
                target.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
            closure.forget();
            return;
        }

        let input_clone = input.clone();
        let key_clone = key.clone();
        state::subscribe(
            shared,
            Box::new(move |sh: &SharedState| {
                let v = state::get_tracked(sh, &key_clone);
                input_clone.set_value(&v.to_string());
            }),
        );

        let shared_clone = shared.clone();
        let input_for_listener = input.clone();
        let key_for_listener = key.clone();
        let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
            let new_val = input_for_listener.value();
            shared_clone
                .borrow_mut()
                .set_state(&key_for_listener, VbValue::str(new_val));
            state::flush(&shared_clone);
        });
        let target: web_sys::EventTarget = input.into();
        let _ = target.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref());
        closure.forget();
        return;
    }

    if let Ok(textarea) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
        let ta_clone = textarea.clone();
        let key_clone = key.clone();
        state::subscribe(
            shared,
            Box::new(move |sh: &SharedState| {
                let v = state::get_tracked(sh, &key_clone);
                ta_clone.set_value(&v.to_string());
            }),
        );

        let shared_clone = shared.clone();
        let ta_for_listener = textarea.clone();
        let key_for_listener = key.clone();
        let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
            let new_val = ta_for_listener.value();
            shared_clone
                .borrow_mut()
                .set_state(&key_for_listener, VbValue::str(new_val));
            state::flush(&shared_clone);
        });
        let target: web_sys::EventTarget = textarea.into();
        let _ = target.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref());
        closure.forget();
        return;
    }

    if let Ok(select) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
        let select_clone = select.clone();
        let key_clone = key.clone();
        state::subscribe(
            shared,
            Box::new(move |sh: &SharedState| {
                let v = state::get_tracked(sh, &key_clone);
                select_clone.set_value(&v.to_string());
            }),
        );

        let shared_clone = shared.clone();
        let select_for_listener = select.clone();
        let key_for_listener = key.clone();
        let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
            let new_val = select_for_listener.value();
            shared_clone
                .borrow_mut()
                .set_state(&key_for_listener, VbValue::str(new_val));
            state::flush(&shared_clone);
        });
        let target: web_sys::EventTarget = select.into();
        let _ = target.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
        closure.forget();
    }
}
