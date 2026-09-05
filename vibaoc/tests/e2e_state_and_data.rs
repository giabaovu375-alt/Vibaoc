// VIBAO COMPILER — end-to-end: state, variables, and data actions.

mod common;
use common::build_source;

#[test]
fn page_level_state_is_present_for_every_page() {
    let (_dir, result) = build_source(
        "page-state",
        r#"
ung_dung("App") {
    trang("/") {
        state $dem_trang_chu = 1
        text($dem_trang_chu)
    }
    trang("/khac") {
        state $dem_khac = 2
        text($dem_khac)
    }
}
"#,
    );
    result.assert_ok();
    let js = result.js();
    assert!(js.contains("dem_trang_chu"), "state from the first page must be present");
    assert!(js.contains("dem_khac"), "state from a later page must also be present");
}

#[test]
fn non_state_variable_is_exposed_as_a_global_var() {
    let (_dir, result) = build_source(
        "global-var",
        r#"
ung_dung("App") {
    trang("/") {
        $gia_co_dinh = 42
        text($gia_co_dinh)
    }
}
"#,
    );
    result.assert_ok();
    let js = result.js();
    assert!(
        js.contains("globalVars") && js.contains("gia_co_dinh"),
        "a bare (non-`state`) variable should be serialized into globalVars"
    );
}

#[test]
fn app_level_state_is_shared_across_pages() {
    let (_dir, result) = build_source(
        "app-level-state",
        r#"
ung_dung("App") {
    state $dem = 0

    trang("/") {
        text($dem)
    }
    trang("/khac") {
        text($dem)
    }
}
"#,
    );
    result.assert_ok();
    assert!(result.js().contains("dem"));
}

#[test]
fn array_push_action_requires_a_bare_state_variable_as_first_argument() {
    let (_dir, result) = build_source(
        "array-push-good",
        r#"
ung_dung("App") {
    trang("/") {
        state $tasks = []
        button("Them") {
            khi_nhan {
                them_vao_mang($tasks, "viec moi")
            }
        }
    }
}
"#,
    );
    result.assert_ok();
}

#[test]
fn array_push_action_rejects_a_string_literal_as_first_argument() {
    let (_dir, result) = build_source(
        "array-push-bad",
        r#"
ung_dung("App") {
    trang("/") {
        state $tasks = []
        button("Them") {
            khi_nhan {
                them_vao_mang("tasks", "viec moi")
            }
        }
    }
}
"#,
    );
    result.assert_err();
    assert!(
        result.stderr.contains("tasks") || result.stderr.to_lowercase().contains("state variable"),
        "the error should point at the malformed first argument, stderr:\n{}",
        result.stderr
    );
}

#[test]
fn array_update_and_remove_by_id_compile_successfully() {
    let (_dir, result) = build_source(
        "array-crud",
        r#"
ung_dung("App") {
    trang("/") {
        state $tasks = [{id: 1, xong: false}]
        button("Xong") {
            khi_nhan {
                cap_nhat_theo_id($tasks, "id", 1, {id: 1, xong: true})
            }
        }
        button("Xoa") {
            khi_nhan {
                xoa_theo_id($tasks, "id", 1)
            }
        }
    }
}
"#,
    );
    result.assert_ok();
}

#[test]
fn two_way_model_binding_wires_input_to_a_state_variable() {
    let (_dir, result) = build_source(
        "model-binding",
        r#"
ung_dung("App") {
    trang("/") {
        state $ten = "Khach"
        input(gia_tri: $ten, loai: "text")
        text("Xin chao, $ten")
    }
}
"#,
    );
    result.assert_ok();
    let html = result.html();
    assert!(html.contains("ten"), "the bound state variable name should appear in the emitted markup");
}

#[test]
fn member_access_on_an_object_in_an_array_resolves_at_build_time() {
    let (_dir, result) = build_source(
        "member-access",
        r#"
ung_dung("App") {
    trang("/") {
        state $tasks = [{id: 1, tieu_de: "Viet tai lieu"}]
        vong_lap $task trong $tasks {
            text($task.tieu_de)
        }
    }
}
"#,
    );
    result.assert_ok();
}

#[test]
fn numeric_addition_on_state_does_not_lose_its_numeric_type_at_build_time() {
    // Regression-style coverage at the compiler boundary: incrementing a
    // numeric state variable must still be recognized as a numeric
    // expression by the compiler (actual runtime arithmetic semantics
    // are covered by vibao-runtime's own unit tests).
    let (_dir, result) = build_source(
        "numeric-counter",
        r#"
ung_dung("App") {
    trang("/") {
        state $dem = 0
        button("Tang") {
            khi_nhan {
                $dem = $dem + 1
            }
        }
        text("Dem: $dem")
    }
}
"#,
    );
    result.assert_ok();
}
