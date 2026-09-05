// ============================================================
// VIBAO RUNTIME (Rust/WASM) — runtime/state.rs
// A direct port of 17-runtime-state.ts to pure Rust.
//
// The core difference from the original JS version:
//   - JS version: a "subscriber" is an arbitrary JS closure
//     (run: () => {...}), freely reading __state.x/__state.y inside;
//     deps are inferred since __getState() records which field was
//     read while the closure runs, via a global __currentTracking
//     variable visible to every function in the same module scope.
//   - Rust version: there is NO implicit "module scope" like JS, so
//     state must be shared explicitly via `SharedState = Rc<RefCell<State>>`.
//     A subscriber holds a `Box<dyn Fn(&SharedState)>` - a pre-compiled
//     pure Rust function (not a string eval) - receiving `&SharedState`
//     (not `&State`/`&mut State` directly) so it can `.borrow_mut()`
//     itself when it needs to call `get_tracked()` (reads + tracks a
//     dependency). Re-running a subscriber (`run_subscriber`), batching
//     (`flush`), and registering/unregistering (`subscribe`/`unsubscribe`)
//     are all FREE FUNCTIONS (not methods on `State`) taking
//     `&SharedState` as a parameter - see the detailed reasoning in the
//     doc-comment right above that group of functions, in the
//     "SUBSCRIBER LIFECYCLE" section.
//
// WASM runs single-threaded in a tab, so RefCell/Rc is used instead of
// Mutex/Arc - no multi-thread synchronization needed, only interior
// mutability so multiple closures (DOM event handlers) can share access
// to one State.
// ============================================================

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::runtime::value::VbValue;
use vibao_ast::Expr;

/// The identifying ID for a subscriber (a registered binding),
/// monotonically increasing - equivalent to a `sub` object's identity in
/// the Set() used by the JS version (in JS, an object reference serves
/// as the identity; in Rust an explicit key is needed).
pub type SubId = u64;

/// A registered binding, equivalent to `{ run, deps }` in the JS
/// version.
///
/// `run` receives `&SharedState` (Rc<RefCell<State>>) - not `&State`/
/// `&mut State` directly - since a binding typically needs to call back
/// into track-aware functions (`get_tracked`), and tracking itself
/// needs to borrow `&mut State` at the exact moment of reading.
/// Receiving `&SharedState` lets the closure `.borrow()`/`.borrow_mut()`
/// itself when needed, with no circular-ownership concern since
/// `SharedState` is passed in from OUTSIDE (by the free functions
/// `subscribe()`/`run_subscriber()`/`flush()` below), not stored inside
/// `State` itself.
struct Subscriber {
    run: Box<dyn Fn(&SharedState)>,
    deps: HashSet<String>,
}

/// The store for a page's entire reactive state - equivalent to merging
/// __state + __vars + __subscribers + __keyIndex + __currentTracking
/// from the old JS version into a single struct.
///
/// Wrapped in `Rc<RefCell<..>>` at its usage sites (see `SharedState`
/// below) since multiple DOM callback closures (onclick, oninput...)
/// need to access and mutate the same State - Rust doesn't allow
/// multiple `&mut` at once, so RefCell is needed to move the borrow
/// check to runtime, for the same reason JS doesn't need to worry about
/// this (JS is always single-threaded, mutable-by-default).
pub struct State {
    state: HashMap<String, VbValue>,
    vars: HashMap<String, VbValue>,

    subscribers: HashMap<SubId, Subscriber>,
    next_sub_id: SubId,
    key_index: HashMap<String, HashSet<SubId>>,

    /// Equivalent to __currentTracking - the subscriber currently mid-run,
    /// so get() knows which subscriber's deps to record a just-read
    /// field into.
    current_tracking: Option<SubId>,

    /// Equivalent to __pendingKeys - batches changes within 1 "tick"
    /// before notifying. The JS version uses queueMicrotask; here the
    /// caller (runtime/dom.rs) explicitly calls `flush()` after finishing
    /// each event/callback, since WASM has no convenient microtask queue
    /// like JS - but the final effect is the same: several consecutive
    /// set_state() calls within the same event handler only trigger 1
    /// re-render when flush() is called.
    pending_keys: HashSet<String>,

    // ── Scope stacks (component props / vong_lap) ─────────────────────
    loop_scope_stack: Vec<LoopFrame>,
    component_scope_stack: Vec<String>,
    /// id -> (prop name -> getter). A getter is a Rust closure, not a JS
    /// function, so props stay "live" (re-reading the parent's state on
    /// every call) for the exact same reason the JS version uses
    /// `() => value` instead of a static value.
    component_props: HashMap<String, HashMap<String, Box<dyn Fn(&State) -> VbValue>>>,

    /// The base URL for goi_api() - equivalent to __api.baseURL in the
    /// old JS version, loaded from optsJson during __vb.boot() (see
    /// dom.rs::VbRuntime::new).
    base_url: String,
    /// Reactive state initialized per route, passed in by the compiler
    /// so the router can correctly reset a page's state during SPA
    /// navigation.
    page_initial_states: Vec<(String, Vec<(String, Expr)>)>,
}

/// One loop's scope "frame" - equivalent to `{ [itemVar]: item, ... }`
/// in the JS version. Keeps item_var/index_var explicit instead of a
/// dynamic object since Rust needs to know the shape ahead of time and
/// can't "add arbitrary fields" like JS.
#[derive(Clone)]
pub struct LoopFrame {
    pub item_var: String,
    pub item_value: VbValue,
    pub index_var: Option<String>,
    pub index_value: Option<f64>,
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
            pending_keys: HashSet::new(),
            loop_scope_stack: Vec::new(),
            component_scope_stack: Vec::new(),
            component_props: HashMap::new(),
            base_url: String::new(),
            page_initial_states: Vec::new(),
        }
    }

    // ════════════════════════════════════════════════════════════
    // STATE STORE — equivalent to section 1 of the JS version
    // ════════════════════════════════════════════════════════════

    /// Equivalent to __setState. Compares by value (VbValue: PartialEq)
    /// instead of JS reference identity (`old === value`) - for
    /// scalar/small array/object data this gives an equivalent practical
    /// result; the only difference is that 2 objects that are *different
    /// references but the same value* would trigger a re-render in JS
    /// but not here - this is an improvement, not a regression (the JS
    /// side treated that as a "latent bug" to avoid by always creating a
    /// new object on change, see the original comment).
    pub fn set_state(&mut self, key: &str, value: VbValue) {
        if let Some(old) = self.state.get(key) {
            if old == &value {
                return;
            }
        }
        self.state.insert(key.to_string(), value);
        self.pending_keys.insert(key.to_string());
    }

    /// Equivalent to __getState - reads the value AND records a
    /// dependency if a subscriber is currently mid-run. This method
    /// still lives on `State` (taking `&mut self`) since tracking only
    /// needs to mutate this struct internally (current_tracking +
    /// subscribers[id].deps), with no need to call back into any other
    /// subscriber - unlike `subscribe`/`flush` (below, outside the impl
    /// block), which need to call `(sub.run)(&SharedState)`.
    pub fn get_state(&mut self, key: &str) -> VbValue {
        if let Some(sub_id) = self.current_tracking {
            if let Some(sub) = self.subscribers.get_mut(&sub_id) {
                sub.deps.insert(key.to_string());
            }
        }
        self.state.get(key).cloned().unwrap_or(VbValue::Null)
    }

    /// Reads directly with no tracking - used internally where
    /// reactivity isn't needed (e.g. debug display, or reading outside
    /// a subscriber's run).
    pub fn peek_state(&self, key: &str) -> VbValue {
        self.state.get(key).cloned().unwrap_or(VbValue::Null)
    }

    pub fn get_var(&self, key: &str) -> VbValue {
        self.vars.get(key).cloned().unwrap_or(VbValue::Null)
    }

    pub fn set_var(&mut self, key: &str, value: VbValue) {
        self.vars.insert(key.to_string(), value);
    }

    // ── Batch notify ───────────────────────────────────────────────
    // The JS version uses queueMicrotask to automatically batch multiple
    // consecutive set_state() calls. Rust/WASM has no equivalent safe
    // "self-triggering" mechanism without adding an external dependency
    // (setTimeout(0) via web-sys is an option but adds a real 1-tick
    // delay). Instead: every entry point calling into Rust from JS (an
    // event handler, __setState called by an action...) MUST end with
    // flush(&shared_state) - a free function below this impl block (see
    // the reasoning for splitting it out of the impl in its
    // doc-comment).

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

    // ════════════════════════════════════════════════════════════
    // MUTATION HELPERS — equivalent to section 2 of the JS version
    // ════════════════════════════════════════════════════════════

    /// $ds.them(item)
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

    /// $ds.xoa(index) - only supports removal by numeric index,
    /// equivalent to the `typeof indexOrItem === 'number'` branch in the
    /// JS version. Removal-by-value (the JS version's other branch) is
    /// split into `state_remove_matching` below since Rust needs an
    /// explicit parameter type instead of "a number or an object".
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

    /// $ds.xoa(item) - removes by matching: if the item has an "id"
    /// field, compares by id; otherwise compares by full value
    /// (strict_eq).
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

    /// $ds.xoa_het()
    pub fn state_clear(&mut self, key: &str) {
        self.set_state(key, VbValue::Array(Vec::new()));
    }

    /// $ds.cap_nhat(index, newValue)
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

    /// $obj.field = value - a shallow copy preserving immutability,
    /// equivalent to __stateSetField. The same design constraint as the
    /// JS version: mutating deep inside an object retrieved from
    /// get_state() will NOT trigger a re-render; every change must go
    /// through these state_* functions.
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

    // ════════════════════════════════════════════════════════════
    // SCOPE RESOLUTION — equivalent to sections 3, 4, 5 of the JS version
    // (member access resolver, component props, loop scope)
    // ════════════════════════════════════════════════════════════

    pub fn push_loop_scope(&mut self, frame: LoopFrame) {
        self.loop_scope_stack.push(frame);
    }

    pub fn pop_loop_scope(&mut self) {
        self.loop_scope_stack.pop();
    }

    pub fn push_component_scope(&mut self, id: &str) {
        self.component_scope_stack.push(id.to_string());
    }

    /// A guarded pop, equivalent to __popComponentScope - logs an
    /// error instead of popping the wrong thing if the top doesn't
    /// match the expected id.
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

    /// Equivalent to __propScope: searches the component scope stack
    /// (innermost to outermost), falling back to global state/vars.
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

    /// Equivalent to __resolveRoot: priority order is loop > @the props >
    /// global. This is the root resolver for __get()-style path access.
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

    /// Equivalent to __scopeResolve: used by the expr evaluator when
    /// resolving a Variable("ten") - the nearest loop scope takes
    /// priority (including a nested path like "item.ten"), then
    /// prop_scope.
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

    /// The TRACKING variant of `scope_resolve` - used by the expr
    /// evaluator when running inside a subscriber (an if/loop/style
    /// dynamic binding needing to auto-re-render when state changes).
    /// Differs from `scope_resolve` (&self, no tracking) in exactly 1
    /// spot: when a variable name matches no loop scope and no
    /// component prop, it falls back to `get_state()` (WHICH records a
    /// dependency) instead of reading `self.state.get()` directly
    /// (recording nothing).
    ///
    /// A variable in loop scope / component props does NOT need
    /// tracking through this path - since a loop itself re-renders its
    /// entire body whenever the list changes (see the control.rs
    /// codegen), and component props are getters that already package
    /// their own tracking logic (if that getter calls get_tracked
    /// internally).
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
        // Component props: the getter decides for itself whether to
        // track (it receives &State, not &mut, so it can't call
        // get_state() itself - if a prop needs to be reactive to its
        // parent's state, the getter should be written to read through
        // a different mechanism; this is a limitation already inherent
        // to register_props()'s design, not something the evaluator can
        // fix).
        for scope_id in self.component_scope_stack.clone().iter().rev() {
            if let Some(getters) = self.component_props.get(scope_id) {
                if let Some(getter) = getters.get(name) {
                    return getter(self);
                }
            }
        }
        // Global: this is the ONLY branch that differs from
        // `scope_resolve` - uses get_state() to record a dependency
        // into the currently running subscriber.
        self.get_state(name)
    }

    /// __get(path) - resolves a full path starting from the root per
    /// the correct scope priority order, then digs into the remaining
    /// parts.
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

    // ════════════════════════════════════════════════════════════
    // INIT / RESET — equivalent to section 7 of the JS version
    // ════════════════════════════════════════════════════════════

    /// Equivalent to __initPageState: resets every subscriber + all
    /// state when navigating pages within the SPA.
    pub fn init_page_state(&mut self, initial: HashMap<String, VbValue>) {
        self.subscribers.clear();
        self.key_index.clear();
        self.pending_keys.clear();
        self.state.clear();
        self.state.extend(initial);
    }

    /// Equivalent to __initGlobalVars: runs once at app boot, never
    /// reset when navigating pages.
    pub fn init_global_vars(&mut self, initial: HashMap<String, VbValue>) {
        self.vars.extend(initial);
    }

    // ── Devtools ─────────────────────────────────────────────────────

    pub fn inspect_state(&self) -> VbValue {
        VbValue::Object(self.state.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    // ── Base URL (used by goi_api) ─────────────────────────────────

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

    pub fn set_base_url(&mut self, url: String) {
        self.base_url = url;
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// The State shared throughout the runtime - every DOM callback closure
/// (event handler, binding) clones this `Rc` to share access to one
/// State. Equivalent to every function in the JS version closing over
/// the same IIFE scope and implicitly sharing __state/__subscribers via
/// closure - in Rust this has to be made explicit via
/// Rc<RefCell<..>> since there's no implicit "module scope" like JS.
pub type SharedState = Rc<RefCell<State>>;

pub fn new_shared_state() -> SharedState {
    Rc::new(RefCell::new(State::new()))
}

// ════════════════════════════════════════════════════════════
// SUBSCRIBER LIFECYCLE — FREE FUNCTIONS (not methods on State)
// ════════════════════════════════════════════════════════════
//
// Why these are split out of `impl State`: `Subscriber::run` needs to
// be called with `&SharedState` so the binding itself can
// `.borrow_mut()` and call back into `get_state()` (tracking a
// dependency) right while it runs. If these functions were `&mut self`
// methods on `State`, calling `(sub.run)(&shared)` would require
// creating a `&SharedState` pointing back to `self` itself - meaning
// `State` would need to know about the `Rc<RefCell<Self>>` wrapping it,
// an unnecessary reference cycle. Instead, the functions below receive
// `&SharedState` from OUTSIDE (provided by the caller - runtime/dom.rs
// - which already holds a copy of this `Rc`), so there's no reference
// cycle at all.

/// Equivalent to __subscribe. Returns a SubId to later call
/// `unsubscribe()` with (replacing the unsubscribe closure in the JS
/// version - Rust needs an explicit key since there's no implicit
/// object-reference identity).
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
    run_subscriber(shared, id); // runs immediately the first time, like the JS version
    id
}

pub fn unsubscribe(shared: &SharedState, id: SubId) {
    let mut state = shared.borrow_mut();
    if let Some(sub) = state.subscribers.remove(&id) {
        state.unindex_sub(id, &sub.deps);
    }
}

fn run_subscriber(shared: &SharedState, id: SubId) {
    // Removes the Subscriber from the map BEFORE calling run - otherwise,
    // when the closure inside `run` calls back into `shared.borrow_mut()`
    // (e.g. so get_state() can track a dependency), it would collide
    // with the borrow held here and panic "already borrowed" at
    // runtime. Removing it from the map first ensures the "take sub
    // out" step's borrow has already ended before run is called.
    let mut sub = {
        let mut state = shared.borrow_mut();
        match state.subscribers.remove(&id) {
            Some(s) => {
                state.unindex_sub(id, &s.deps);
                s
            }
            None => return, // already unsubscribed while pending
        }
    };

    let prev_tracking = {
        let mut state = shared.borrow_mut();
        let prev = state.current_tracking;
        state.current_tracking = Some(id);
        prev
    };

    // Deps are rebuilt from scratch on every re-run - DELIBERATE (see
    // the original comment in the JS version): the dependency branch can
    // change based on runtime conditions.
    sub.deps.clear();

    (sub.run)(shared);

    let mut state = shared.borrow_mut();
    state.current_tracking = prev_tracking;
    state.index_sub(id, &sub.deps);
    state.subscribers.insert(id, sub);
}

/// Equivalent to the body of __scheduleNotify's queueMicrotask
/// callback, but called explicitly instead of automatically (see the
/// reasoning in the `pending_keys` field's doc-comment). Idempotent if
/// nothing is pending.
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

/// Reads state AND tracks a dependency, used by the expr evaluator (the
/// next module) when running inside a subscriber. Equivalent to calling
/// `__getState()` directly in the JS version. Receives `&SharedState` to
/// match the signature that binding closures receive.
pub fn get_tracked(shared: &SharedState, key: &str) -> VbValue {
    shared.borrow_mut().get_state(key)
}

/// Reads the current base_url - used by action.rs when executing goi_api().
pub fn get_base_url(shared: &SharedState) -> String {
    shared.borrow().base_url().to_string()
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
        state.pending_keys.clear(); // simulates already having flushed
        state.set_state("dem", VbValue::num(1.0)); // unchanged value
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
                let _ = get_tracked(sh, "n"); // reads + tracks "n" like a real binding
            }),
        );
        // The first run (subscribe runs itself once)
        assert_eq!(*run_count.borrow(), 1);

        // Changes "n" then flushes -> the subscriber must re-run since it tracked "n"
        shared.borrow_mut().set_state("n", VbValue::num(2.0));
        flush(&shared);
        assert_eq!(*run_count.borrow(), 2);

        unsubscribe(&shared, sub_id);

        // After unsubscribing, changing state no longer triggers a run
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
                let v = get_tracked(sh, "n"); // reads + tracks "n"
                *seen_clone.borrow_mut() = v.as_num().unwrap_or(0.0);
            }),
        );

        assert_eq!(*seen.borrow(), 1.0);

        shared.borrow_mut().set_state("n", VbValue::num(2.0));
        flush(&shared);
        assert_eq!(*seen.borrow(), 2.0); // auto-rerun since "n" was tracked
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
        // Popping with the wrong id - must be rejected, stack stays [a, b]
        state.pop_component_scope("a");
        assert_eq!(state.component_scope_stack.last().map(|s| s.as_str()), Some("b"));
        state.pop_component_scope("b");
        assert_eq!(state.component_scope_stack.last().map(|s| s.as_str()), Some("a"));
    }

    // ── init_global_vars() — BUG-19 already fixed ──────────────────────────
    // (see RUNTIME_BOUNDARY_BUGS.md). This function
    // used to exist but had NO caller - the tests below confirm it
    // works correctly AT THE STATE LAYER (no DOM access, runs under
    // plain `cargo test`). Calling it from VbRuntime::new() with data
    // ALREADY EVALUATED FROM AN EXPR (literal/expression/variable
    // reference) is tested separately in dom.rs (only functions that
    // DON'T touch a real DOM can be tested there - the BootOpts
    // deserialization/init_global_vars call sits INSIDE the constructor
    // with #[wasm_bindgen(constructor)], which can't be split out for
    // independent testing under plain cargo test; a manual build+run
    // check in a real browser is NEEDED to fully confirm the entire
    // "bien -> compiler -> generated JS -> runtime state -> actually
    // usable" chain).

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
    fn test_init_global_vars_basic() {
        let mut state = State::new();
        let mut initial = HashMap::new();
        initial.insert("gia_co_dinh".to_string(), VbValue::num(1000.0));
        state.init_global_vars(initial);
        assert_eq!(state.scope_resolve("gia_co_dinh").as_num(), Some(1000.0));
    }

    #[test]
    fn test_init_global_vars_multiple_variables() {
        // Requirement: test with MULTIPLE variables at once, not just 1.
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
        // `vars` (global vars) and `state` (reactive) MUST stay separate
        // - this is exactly why the BUG-19 fix used a separate
        // field/function instead of merging into
        // set_state()/initial_state. Confirms: after init_global_vars(),
        // state.state (the reactive map) is STILL EMPTY - a `bien`
        // variable doesn't accidentally become reactive.
        let mut state = State::new();
        let mut initial = HashMap::new();
        initial.insert("hang_so".to_string(), VbValue::num(42.0));
        state.init_global_vars(initial);
        assert!(state.state.is_empty(), "global vars must not leak into reactive state");
        // But still readable via scope_resolve (a shared read interface)
        assert_eq!(state.scope_resolve("hang_so").as_num(), Some(42.0));
    }

    #[test]
    fn test_init_global_vars_not_reset_by_init_page_state() {
        // The `vars` field does NOT reset when navigating pages (SPA) -
        // unlike `state` (reactive, reset by init_page_state when the
        // router activates a different page). Global vars mean
        // "app-wide constants", existing throughout the app's entire
        // lifetime, not per page.
        let mut state = State::new();
        let mut initial = HashMap::new();
        initial.insert("hang_so".to_string(), VbValue::num(42.0));
        state.init_global_vars(initial);

        // Simulates navigating pages: init_page_state with a different page's new state
        let mut new_page_state = HashMap::new();
        new_page_state.insert("dem".to_string(), VbValue::num(0.0));
        state.init_page_state(new_page_state);

        // "hang_so" (a global var) is still intact after navigating pages
        assert_eq!(state.scope_resolve("hang_so").as_num(), Some(42.0));
    }

    #[test]
    fn test_state_takes_priority_over_global_vars_on_name_collision() {
        // If (rare, shouldn't happen in valid code, but the behavior
        // still needs to be well-defined) a variable name is BOTH a
        // `state` AND a `bien` (same name), scope_resolve/prop_scope
        // gives `state` priority (see prop_scope():
        // "self.state.get(name).or_else(|| self.vars.get(name))") - this
        // test LOCKS IN that behavior, so if anyone changes the priority
        // order in the future, the test will fail instead of silently
        // changing the language's meaning.
        let mut state = State::new();
        state.set_state("x", VbValue::num(1.0));
        let mut initial = HashMap::new();
        initial.insert("x".to_string(), VbValue::num(999.0));
        state.init_global_vars(initial);
        assert_eq!(state.scope_resolve("x").as_num(), Some(1.0));
    }
}
