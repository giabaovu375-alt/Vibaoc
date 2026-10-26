use super::text_attr::list_attribute_names_with_prefix;
use super::{parse_expr_id, ATTR_EVENT_PREFIX};
use crate::runtime::state::{LoopFrame, SharedState};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{Element, Event};

pub(super) fn event_needs_prevent_default(event_name: &str) -> bool {
    event_name == "submit"
}

pub(super) fn bind_events(
    shared: &SharedState,
    el: &Element,
    loop_frame: Option<&LoopFrame>,
    component_id: Option<&str>,
) {
    let attr_names = list_attribute_names_with_prefix(el, ATTR_EVENT_PREFIX);
    for full_name in attr_names {
        let bound_marker = format!("data-vb-bound-{}", &full_name[ATTR_EVENT_PREFIX.len()..]);
        if el.has_attribute(&bound_marker) {
            continue;
        }

        let event_name = &full_name[ATTR_EVENT_PREFIX.len()..];
        let raw = match el.get_attribute(&full_name) {
            Some(r) => r,
            None => continue,
        };
        let action_id = match parse_expr_id(&raw, &full_name) {
            Some(id) => id,
            None => continue,
        };

        let shared_clone = shared.clone();
        let event_target: web_sys::EventTarget = el.clone().into();

        let actions = match crate::runtime::action_registry::get(action_id) {
            Some(a) => a,
            None => {
                crate::runtime::log::warn(&format!(
                    "[ViBao] action id {} does not exist in the registry",
                    action_id
                ));
                continue;
            }
        };
        let actions = std::rc::Rc::new(actions);
        let loop_frame_owned = loop_frame.cloned();
        let component_id_owned = component_id.map(|s| s.to_string());

        let needs_prevent_default = event_needs_prevent_default(event_name);

        let closure = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
            if needs_prevent_default {
                evt.prevent_default();
            }
            let shared_for_task = shared_clone.clone();
            let actions_for_task = actions.clone();
            let loop_frame_for_task = loop_frame_owned.clone();
            let component_id_for_task = component_id_owned.clone();
            wasm_bindgen_futures::spawn_local(async move {
                crate::runtime::action::dispatch_all(
                    &shared_for_task,
                    &actions_for_task,
                    loop_frame_for_task.as_ref(),
                    component_id_for_task.as_deref(),
                )
                .await;
            });
        });

        let _ = event_target
            .add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref());
        closure.forget();

        let _ = el.set_attribute(&bound_marker, "1");
    }
}
