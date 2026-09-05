//! English surface names for `ActionName`.

use vibao_ast::ActionName;

pub fn action_name_en(name: &str) -> Option<ActionName> {
    Some(match name {
        "notify" => ActionName::Notify,
        "alert" => ActionName::Alert,
        "navigate" => ActionName::Navigate,
        "open_new_tab" => ActionName::OpenNewTab,
        "open_modal" => ActionName::OpenModal,
        "close_modal" => ActionName::CloseModal,
        "scroll_to" => ActionName::ScrollTo,
        "scroll_to_top" => ActionName::ScrollToTop,
        "save_data" => ActionName::SaveData,
        "load_data" => ActionName::LoadData,
        "copy_to_clipboard" => ActionName::CopyToClipboard,
        "api_call" => ActionName::ApiCall,
        "array_push" => ActionName::ArrayPush,
        "array_remove_by_id" => ActionName::ArrayRemoveById,
        "array_update_by_id" => ActionName::ArrayUpdateById,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const ALL: &[&str] = &[
        "notify", "alert", "navigate", "open_new_tab", "open_modal", "close_modal",
        "scroll_to", "scroll_to_top", "save_data", "load_data", "copy_to_clipboard",
        "api_call", "array_push", "array_remove_by_id", "array_update_by_id",
    ];

    #[test]
    fn all_names_resolve() {
        assert_eq!(ALL.len(), 15);
        assert!(ALL.iter().all(|name| action_name_en(name).is_some()));
    }

    #[test]
    fn all_names_resolve_to_distinct_action_names() {
        let resolved: HashSet<ActionName> = ALL.iter().filter_map(|n| action_name_en(n)).collect();
        assert_eq!(resolved.len(), 15);
    }

    #[test]
    fn vietnamese_names_do_not_resolve_here() {
        assert_eq!(action_name_en("thong_bao"), None);
    }
}
