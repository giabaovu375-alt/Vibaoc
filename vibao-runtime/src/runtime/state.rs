use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::runtime::value::VbValue;
use vibao_ast::Expr;

pub type SubId = u64;

struct Subscriber {
    run: Box<dyn Fn(&SharedState)>,
    deps: HashSet<String>,
}

pub struct State {
    state: HashMap<String, VbValue>,
    vars: HashMap<String, VbValue>,

    subscribers: HashMap<SubId, Subscriber>,
    next_sub_id: SubId,
    key_index: HashMap<String, HashSet<SubId>>,

    current_tracking: Option<SubId>,

    current_tracking_deps: HashSet<String>,

    unsubscribed_while_running: HashSet<SubId>,

    pending_keys: HashSet<String>,

    loop_scope_stack: Vec<LoopFrame>,
    component_scope_stack: Vec<String>,
    component_props: HashMap<String, HashMap<String, Box<dyn Fn(&State) -> VbValue>>>,

    base_url: String,
    lang: String,
    page_initial_states: Vec<(String, Vec<(String, Expr)>)>,
    page_metadata: Vec<(String, Option<String>, Option<String>)>,
}

#[derive(Clone)]
pub struct LoopFrame {
    pub item_var: String,
    pub item_value: VbValue,
    pub index_var: Option<String>,
    pub index_value: Option<f64>,
}

pub(crate) struct ScopeGuard<'a> {
    shared: &'a SharedState,
    component_id: Option<String>,
    has_loop_frame: bool,
}

impl<'a> ScopeGuard<'a> {
    pub(crate) fn new(
        shared: &'a SharedState,
        loop_frame: Option<&LoopFrame>,
        component_id: Option<&str>,
    ) -> Self {
        if let Some(frame) = loop_frame {
            shared.borrow_mut().push_loop_scope(frame.clone());
        }
        if let Some(cid) = component_id {
            shared.borrow_mut().push_component_scope(cid);
        }
        ScopeGuard {
            shared,
            component_id: component_id.map(|s| s.to_string()),
            has_loop_frame: loop_frame.is_some(),
        }
    }
}

impl<'a> Drop for ScopeGuard<'a> {
    fn drop(&mut self) {
        if let Some(cid) = &self.component_id {
            self.shared.borrow_mut().pop_component_scope(cid);
        }
        if self.has_loop_frame {
            self.shared.borrow_mut().pop_loop_scope();
        }
    }
}

impl State {
    pub fn new() -> Self {
        State {
            state: HashMap::new(),
            vars: HashMap::new(),
            subscribers: HashMap::new(),
            next_sub_id: 0,
            key_index: HashMap::new(),
            current_tracking: None,
            current_tracking_deps: HashSet::new(),
            unsubscribed_while_running: HashSet::new(),
            pending_keys: HashSet::new(),
            loop_scope_stack: Vec::new(),
            component_scope_stack: Vec::new(),
            component_props: HashMap::new(),
            base_url: String::new(),
            lang: "vi".to_string(),
            page_initial_states: Vec::new(),
            page_metadata: Vec::new(),
        }
    }

    pub fn set_state(&mut self, key: &str, value: VbValue) {
        if let Some(old) = self.state.get(key) {
            if old == &value {
                return;
            }
        }
        self.state.insert(key.to_string(), value);
        self.pending_keys.insert(key.to_string());
    }

    pub fn get_state(&mut self, key: &str) -> VbValue {
        if self.current_tracking.is_some() {
            self.current_tracking_deps.insert(key.to_string());
        }
        self.state.get(key).cloned().unwrap_or(VbValue::Null)
    }

    pub fn peek_state(&self, key: &str) -> VbValue {
        self.state.get(key).cloned().unwrap_or(VbValue::Null)
    }

    pub fn get_var(&self, key: &str) -> VbValue {
        self.vars.get(key).cloned().unwrap_or(VbValue::Null)
    }

    pub fn set_var(&mut self, key: &str, value: VbValue) {
        self.vars.insert(key.to_string(), value);
    }

    fn index_sub(&mut self, id: SubId, deps: &HashSet<String>) {
        for key in deps {
            self.key_index.entry(key.clone()).or_default().insert(id);
        }
    }

    fn unindex_sub(&mut self, id: SubId, deps: &HashSet<String>) {
        for key in deps {
            if let Some(set) = self.key_index.get_mut(key) {
                set.remove(&id);
            }
        }
    }

    pub fn state_push(&mut self, key: &str, item: VbValue) {
        let arr = match self.state.get(key) {
            Some(VbValue::Array(a)) => a.clone(),
            _ => {
                crate::runtime::log::warn(&format!(
                    "[ViBao] \"${}.them()\" was called on a value that is not an array",
                    key
                ));
                return;
            }
        };
        let mut next = arr;
        next.push(item);
        self.set_state(key, VbValue::Array(next));
    }

    pub fn state_remove_by_index(&mut self, key: &str, index: usize) {
        let arr = match self.state.get(key) {
            Some(VbValue::Array(a)) => a.clone(),
            _ => return,
        };
        let next: Vec<VbValue> = arr
            .into_iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, v)| v)
            .collect();
        self.set_state(key, VbValue::Array(next));
    }

    pub fn state_remove_matching(&mut self, key: &str, target: &VbValue) {
        let arr = match self.state.get(key) {
            Some(VbValue::Array(a)) => a.clone(),
            _ => return,
        };
        let target_id = target.as_object().and_then(|o| o.get("id"));
        let next: Vec<VbValue> = arr
            .into_iter()
            .filter(|it| {
                if let (Some(tid), Some(it_obj)) = (target_id, it.as_object()) {
                    if let Some(iid) = it_obj.get("id") {
                        return iid != tid;
                    }
                }
                it != target
            })
            .collect();
        self.set_state(key, VbValue::Array(next));
    }

    pub fn state_clear(&mut self, key: &str) {
        self.set_state(key, VbValue::Array(Vec::new()));
    }

    pub fn state_update(&mut self, key: &str, index: usize, new_value: VbValue) {
        let arr = match self.state.get(key) {
            Some(VbValue::Array(a)) => a.clone(),
            _ => return,
        };
        let next: Vec<VbValue> = arr
            .into_iter()
            .enumerate()
            .map(|(i, v)| if i == index { new_value.clone() } else { v })
            .collect();
        self.set_state(key, VbValue::Array(next));
    }

    pub fn state_set_field(&mut self, key: &str, field: &str, value: VbValue) {
        let obj = match self.state.get(key) {
            Some(VbValue::Object(o)) => o.clone(),
            _ => {
                crate::runtime::log::warn(&format!(
                    "[ViBao] \"${}.{} = ...\" was called on a value that is not an object",
                    key, field
                ));
                return;
            }
        };
        let mut next = obj;
        next.insert(field.to_string(), value);
        self.set_state(key, VbValue::Object(next));
    }

    pub fn push_loop_scope(&mut self, frame: LoopFrame) {
        self.loop_scope_stack.push(frame);
    }

    pub fn pop_loop_scope(&mut self) {
        self.loop_scope_stack.pop();
    }

    pub fn push_component_scope(&mut self, id: &str) {
        self.component_scope_stack.push(id.to_string());
    }

    pub fn pop_component_scope(&mut self, expected_id: &str) {
        match self.component_scope_stack.last() {
            Some(top) if top == expected_id => {
                self.component_scope_stack.pop();
            }
            Some(top) => {
                crate::runtime::log::error(&format!(
                    "[ViBao] Component scope stack mismatch: expected \"{}\" but the top is \"{}\" - skipping the pop to avoid corrupting the stack.",
                    expected_id, top
                ));
            }
            None => {}
        }
    }

    pub fn register_props(&mut self, id: &str, getters: HashMap<String, Box<dyn Fn(&State) -> VbValue>>) {
        self.component_props.insert(id.to_string(), getters);
    }

    pub fn unregister_props(&mut self, id: &str) {
        self.component_props.remove(id);
    }

    #[cfg(test)]
    pub(crate) fn component_props_count(&self) -> usize {
        self.component_props.len()
    }

    fn prop_scope(&self, name: &str) -> VbValue {
        for scope_id in self.component_scope_stack.iter().rev() {
            if let Some(getters) = self.component_props.get(scope_id) {
                if let Some(getter) = getters.get(name) {
                    return getter(self);
                }
            }
        }
        self.state
            .get(name)
            .or_else(|| self.vars.get(name))
            .cloned()
            .unwrap_or(VbValue::Null)
    }

    pub fn resolve_root(&self, name: &str) -> VbValue {
        for frame in self.loop_scope_stack.iter().rev() {
            if frame.item_var == name {
                return frame.item_value.clone();
            }
            if let Some(idx_var) = &frame.index_var {
                if idx_var == name {
                    return frame.index_value.map(VbValue::Num).unwrap_or(VbValue::Null);
                }
            }
        }
        for scope_id in self.component_scope_stack.iter().rev() {
            if let Some(getters) = self.component_props.get(scope_id) {
                if let Some(getter) = getters.get(name) {
                    return getter(self);
                }
            }
        }
        self.state
            .get(name)
            .or_else(|| self.vars.get(name))
            .cloned()
            .unwrap_or(VbValue::Null)
    }

    pub fn scope_resolve(&self, name: &str) -> VbValue {
        for frame in self.loop_scope_stack.iter().rev() {
            if frame.item_var == name {
                return frame.item_value.clone();
            }
            if let Some(idx_var) = &frame.index_var {
                if idx_var == name {
                    return frame.index_value.map(VbValue::Num).unwrap_or(VbValue::Null);
                }
            }
            let prefix = format!("{}.", frame.item_var);
            if let Some(sub_path) = name.strip_prefix(&prefix) {
                return frame.item_value.dig_path(sub_path);
            }
        }
        self.prop_scope(name)
    }

    pub fn scope_resolve_tracked(&mut self, name: &str) -> VbValue {
        for frame in self.loop_scope_stack.clone().iter().rev() {
            if frame.item_var == name {
                return frame.item_value.clone();
            }
            if let Some(idx_var) = &frame.index_var {
                if idx_var == name {
                    return frame.index_value.map(VbValue::Num).unwrap_or(VbValue::Null);
                }
            }
            let prefix = format!("{}.", frame.item_var);
            if let Some(sub_path) = name.strip_prefix(&prefix) {
                return frame.item_value.dig_path(sub_path);
            }
        }
        for scope_id in self.component_scope_stack.clone().iter().rev() {
            if let Some(getters) = self.component_props.get(scope_id) {
                if let Some(getter) = getters.get(name) {
                    return getter(self);
                }
            }
        }
        self.get_state(name)
    }

    pub fn get_path(&self, path: &str) -> VbValue {
        let mut parts = path.split('.');
        let root_name = match parts.next() {
            Some(p) => p,
            None => return VbValue::Null,
        };
        let mut cur = self.resolve_root(root_name);
        for part in parts {
            if cur.is_null() {
                return VbValue::Null;
            }
            cur = cur.get_field(part);
        }
        cur
    }

    pub fn init_page_state(&mut self, initial: HashMap<String, VbValue>) {
        self.subscribers.clear();
        self.key_index.clear();
        self.pending_keys.clear();
        self.state.clear();
        self.state.extend(initial);
    }

    pub fn init_global_vars(&mut self, initial: HashMap<String, VbValue>) {
        self.vars.extend(initial);
    }

    pub fn inspect_state(&self) -> VbValue {
        VbValue::Object(self.state.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    pub fn set_page_initial_states(&mut self, states: Vec<(String, Vec<(String, Expr)>)>) {
        self.page_initial_states = states;
    }

    pub fn page_initial_states_for_route(&self, route: &str) -> Vec<(String, Expr)> {
        self.page_initial_states
            .iter()
            .find(|(pattern, _)| pattern == route)
            .map(|(_, exprs)| exprs.clone())
            .unwrap_or_default()
    }

    pub fn set_page_metadata(&mut self, metadata: Vec<(String, Option<String>, Option<String>)>) {
        self.page_metadata = metadata;
    }

    pub fn page_metadata_for_route(&self, route: &str) -> Option<(Option<String>, Option<String>)> {
        self.page_metadata
            .iter()
            .find(|(pattern, _, _)| pattern == route)
            .map(|(_, name, description)| (name.clone(), description.clone()))
    }

    pub fn set_base_url(&mut self, url: String) {
        self.base_url = url;
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn set_lang(&mut self, lang: String) {
        self.lang = lang;
    }

    pub fn lang(&self) -> &str {
        &self.lang
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedState = Rc<RefCell<State>>;

pub fn new_shared_state() -> SharedState {
    Rc::new(RefCell::new(State::new()))
}

pub fn subscribe(shared: &SharedState, run: Box<dyn Fn(&SharedState)>) -> SubId {
    let id = {
        let mut state = shared.borrow_mut();
        let id = state.next_sub_id;
        state.next_sub_id += 1;
        state.subscribers.insert(
            id,
            Subscriber {
                run,
                deps: HashSet::new(),
            },
        );
        id
    };
    run_subscriber(shared, id);
    id
}

pub fn unsubscribe(shared: &SharedState, id: SubId) {
    let mut state = shared.borrow_mut();
    if let Some(sub) = state.subscribers.remove(&id) {
        state.unindex_sub(id, &sub.deps);
    } else if state.current_tracking == Some(id) {
        state.unsubscribed_while_running.insert(id);
    }
}

fn run_subscriber(shared: &SharedState, id: SubId) {
    let sub = {
        let mut state = shared.borrow_mut();
        match state.subscribers.remove(&id) {
            Some(s) => {
                state.unindex_sub(id, &s.deps);
                s
            }
            None => return,
        }
    };

    let (prev_tracking, prev_tracking_deps) = {
        let mut state = shared.borrow_mut();
        let prev_tracking = state.current_tracking;
        let prev_deps = std::mem::take(&mut state.current_tracking_deps);
        state.current_tracking = Some(id);
        (prev_tracking, prev_deps)
    };

    (sub.run)(shared);

    let mut state = shared.borrow_mut();
    state.current_tracking = prev_tracking;
    let new_deps = std::mem::replace(&mut state.current_tracking_deps, prev_tracking_deps);
    let mut sub = sub;
    sub.deps = new_deps;
    if state.unsubscribed_while_running.remove(&id) {
        return;
    }
    state.index_sub(id, &sub.deps);
    state.subscribers.insert(id, sub);
}

pub fn flush(shared: &SharedState) {
    let keys: Vec<String> = {
        let mut state = shared.borrow_mut();
        if state.pending_keys.is_empty() {
            return;
        }
        state.pending_keys.drain().collect()
    };

    let to_run: HashSet<SubId> = {
        let state = shared.borrow();
        let mut set = HashSet::new();
        for key in &keys {
            if let Some(subs) = state.key_index.get(key) {
                set.extend(subs.iter().copied());
            }
        }
        for (id, sub) in &state.subscribers {
            if sub.deps.is_empty() {
                set.insert(*id);
            }
        }
        set
    };

    for id in to_run {
        run_subscriber(shared, id);
    }
}

pub fn get_tracked(shared: &SharedState, key: &str) -> VbValue {
    shared.borrow_mut().get_state(key)
}

pub fn get_base_url(shared: &SharedState) -> String {
    shared.borrow().base_url().to_string()
}

pub fn get_lang(shared: &SharedState) -> String {
    shared.borrow().lang().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_get_state_roundtrip() {
        let mut state = State::new();
        state.set_state("dem", VbValue::num(1.0));
        assert_eq!(state.peek_state("dem").as_num(), Some(1.0));
    }

    #[test]
    fn test_set_state_same_value_does_not_mark_pending() {
        let mut state = State::new();
        state.set_state("dem", VbValue::num(1.0));
        state.pending_keys.clear();
        state.set_state("dem", VbValue::num(1.0));
        assert!(state.pending_keys.is_empty());
    }

    #[test]
    fn test_subscriber_reruns_on_dependency_change() {
        let shared = new_shared_state();
        let run_count = Rc::new(RefCell::new(0));

        shared.borrow_mut().set_state("n", VbValue::num(1.0));

        let run_count_clone = run_count.clone();
        let sub_id = subscribe(
            &shared,
            Box::new(move |sh: &SharedState| {
                *run_count_clone.borrow_mut() += 1;
                let _ = get_tracked(sh, "n");
            }),
        );
        assert_eq!(*run_count.borrow(), 1);

        shared.borrow_mut().set_state("n", VbValue::num(2.0));
        flush(&shared);
        assert_eq!(*run_count.borrow(), 2);

        unsubscribe(&shared, sub_id);

        shared.borrow_mut().set_state("n", VbValue::num(3.0));
        flush(&shared);
        assert_eq!(*run_count.borrow(), 2);
    }

    #[test]
    fn test_self_unsubscribe_during_run_is_not_revived() {
        let shared = new_shared_state();
        let run_count = Rc::new(RefCell::new(0));
        let self_id: Rc<RefCell<Option<SubId>>> = Rc::new(RefCell::new(None));

        shared.borrow_mut().set_state("n", VbValue::num(1.0));

        let run_count_clone = run_count.clone();
        let self_id_clone = self_id.clone();
        let id = subscribe(
            &shared,
            Box::new(move |sh: &SharedState| {
                *run_count_clone.borrow_mut() += 1;
                let _ = get_tracked(sh, "n");
                if *run_count_clone.borrow() == 2 {
                    if let Some(my_id) = *self_id_clone.borrow() {
                        unsubscribe(sh, my_id);
                    }
                }
            }),
        );
        *self_id.borrow_mut() = Some(id);
        assert_eq!(*run_count.borrow(), 1);

        shared.borrow_mut().set_state("n", VbValue::num(2.0));
        flush(&shared);
        assert_eq!(*run_count.borrow(), 2);

        shared.borrow_mut().set_state("n", VbValue::num(3.0));
        flush(&shared);
        assert_eq!(*run_count.borrow(), 2);
    }

    #[test]
    fn test_auto_tracking_via_get_state() {
        let shared = new_shared_state();
        shared.borrow_mut().set_state("n", VbValue::num(1.0));

        let seen = Rc::new(RefCell::new(0.0));
        let seen_clone = seen.clone();

        subscribe(
            &shared,
            Box::new(move |sh: &SharedState| {
                let v = get_tracked(sh, "n");
                *seen_clone.borrow_mut() = v.as_num().unwrap_or(0.0);
            }),
        );

        assert_eq!(*seen.borrow(), 1.0);

        shared.borrow_mut().set_state("n", VbValue::num(2.0));
        flush(&shared);
        assert_eq!(*seen.borrow(), 2.0);
    }

    #[test]
    fn test_state_push_and_remove() {
        let mut state = State::new();
        state.set_state("ds", VbValue::Array(vec![VbValue::num(1.0)]));
        state.state_push("ds", VbValue::num(2.0));
        assert_eq!(
            state.peek_state("ds").as_array().map(|a| a.len()),
            Some(2)
        );

        state.state_remove_by_index("ds", 0);
        let arr = state.peek_state("ds");
        assert_eq!(arr.as_array().unwrap()[0].as_num(), Some(2.0));
    }

    #[test]
    fn test_state_set_field_shallow_copy() {
        let mut state = State::new();
        let mut obj = std::collections::BTreeMap::new();
        obj.insert("ten".to_string(), VbValue::str("An"));
        state.set_state("nguoi_dung", VbValue::Object(obj));

        state.state_set_field("nguoi_dung", "ten", VbValue::str("Binh"));
        let updated = state.peek_state("nguoi_dung");
        assert_eq!(
            updated.as_object().unwrap().get("ten").unwrap().as_str(),
            Some("Binh")
        );
    }

    #[test]
    fn test_loop_scope_resolve_priority_over_global() {
        let mut state = State::new();
        state.set_state("item", VbValue::str("global"));
        state.push_loop_scope(LoopFrame {
            item_var: "item".to_string(),
            item_value: VbValue::str("local"),
            index_var: None,
            index_value: None,
        });

        assert_eq!(state.scope_resolve("item").as_str(), Some("local"));
        state.pop_loop_scope();
        assert_eq!(state.scope_resolve("item").as_str(), Some("global"));
    }

    #[test]
    fn test_get_path_nested_with_special_field() {
        let mut state = State::new();
        state.set_state(
            "ds",
            VbValue::Array(vec![VbValue::num(1.0), VbValue::num(2.0)]),
        );
        assert_eq!(state.get_path("ds.do_dai").as_num(), Some(2.0));
    }

    #[test]
    fn test_component_scope_mismatch_does_not_corrupt_stack() {
        let mut state = State::new();
        state.push_component_scope("a");
        state.push_component_scope("b");
        state.pop_component_scope("a");
        assert_eq!(state.component_scope_stack.last().map(|s| s.as_str()), Some("b"));
        state.pop_component_scope("b");
        assert_eq!(state.component_scope_stack.last().map(|s| s.as_str()), Some("a"));
    }

    #[test]
    fn test_component_props_can_be_unregistered() {
        let mut state = State::new();
        state.register_props("component-a", HashMap::new());
        state.register_props("component-b", HashMap::new());
        assert_eq!(state.component_props_count(), 2);
        state.unregister_props("component-a");
        assert_eq!(state.component_props_count(), 1);
        state.unregister_props("component-b");
        assert_eq!(state.component_props_count(), 0);
    }

    #[test]
    fn test_page_initial_states_are_stored_per_route() {
        let mut state = State::new();
        state.set_page_initial_states(vec![
            (
                "/".to_string(),
                vec![("dem_home".to_string(), vibao_ast::Expr::literal_num(1.0, vibao_ast::Pos { line: 1, column: 1 }))],
            ),
            (
                "/about".to_string(),
                vec![("dem_about".to_string(), vibao_ast::Expr::literal_num(2.0, vibao_ast::Pos { line: 1, column: 1 }))],
            ),
        ]);

        let home = state.page_initial_states_for_route("/");
        let about = state.page_initial_states_for_route("/about");
        assert_eq!(home.len(), 1);
        assert_eq!(about.len(), 1);
        assert_eq!(home[0].0, "dem_home");
        assert_eq!(about[0].0, "dem_about");
    }

    #[test]
    fn test_page_metadata_for_route() {
        let mut state = State::new();
        state.set_page_metadata(vec![
            ("/".to_string(), Some("Trang chủ".to_string()), Some("Giới thiệu bản thân".to_string())),
            ("/lien-he".to_string(), Some("Liên hệ".to_string()), None),
        ]);

        assert_eq!(
            state.page_metadata_for_route("/"),
            Some((Some("Trang chủ".to_string()), Some("Giới thiệu bản thân".to_string())))
        );
        assert_eq!(
            state.page_metadata_for_route("/lien-he"),
            Some((Some("Liên hệ".to_string()), None))
        );
        assert_eq!(state.page_metadata_for_route("/khong-ton-tai"), None);
    }

    #[test]
    fn test_init_global_vars_basic() {
        let mut state = State::new();
        let mut initial = HashMap::new();
        initial.insert("gia_co_dinh".to_string(), VbValue::num(1000.0));
        state.init_global_vars(initial);
        assert_eq!(state.scope_resolve("gia_co_dinh").as_num(), Some(1000.0));
    }

    #[test]
    fn test_init_global_vars_multiple_variables() {
        let mut state = State::new();
        let mut initial = HashMap::new();
        initial.insert("a".to_string(), VbValue::num(1.0));
        initial.insert("b".to_string(), VbValue::Str("xin chao".to_string()));
        initial.insert("c".to_string(), VbValue::Bool(true));
        state.init_global_vars(initial);
        assert_eq!(state.scope_resolve("a").as_num(), Some(1.0));
        assert_eq!(state.scope_resolve("b"), VbValue::Str("xin chao".to_string()));
        assert_eq!(state.scope_resolve("c"), VbValue::Bool(true));
    }

    #[test]
    fn test_init_global_vars_does_not_pollute_reactive_state() {
        let mut state = State::new();
        let mut initial = HashMap::new();
        initial.insert("hang_so".to_string(), VbValue::num(42.0));
        state.init_global_vars(initial);
        assert!(state.state.is_empty(), "global vars must not leak into reactive state");
        assert_eq!(state.scope_resolve("hang_so").as_num(), Some(42.0));
    }

    #[test]
    fn test_init_global_vars_not_reset_by_init_page_state() {
        let mut state = State::new();
        let mut initial = HashMap::new();
        initial.insert("hang_so".to_string(), VbValue::num(42.0));
        state.init_global_vars(initial);

        let mut new_page_state = HashMap::new();
        new_page_state.insert("dem".to_string(), VbValue::num(0.0));
        state.init_page_state(new_page_state);

        assert_eq!(state.scope_resolve("hang_so").as_num(), Some(42.0));
    }

    #[test]
    fn test_state_takes_priority_over_global_vars_on_name_collision() {
        let mut state = State::new();
        state.set_state("x", VbValue::num(1.0));
        let mut initial = HashMap::new();
        initial.insert("x".to_string(), VbValue::num(999.0));
        state.init_global_vars(initial);
        assert_eq!(state.scope_resolve("x").as_num(), Some(1.0));
    }
}
