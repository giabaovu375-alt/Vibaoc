# ViBao

**ViBao** là một DSL và compiler ưu tiên tiếng Việt để xây web UI.
ViBao compile trực tiếp ra HTML/CSS/JS cùng một runtime WebAssembly nhỏ
gọn — không cần Node.js, không cần bundler, không cần framework
JavaScript.

```vbao
ung_dung("Ung dung cua toi") {
    trang("/") {
        khoi(dem: 16) {
            text("So dem: ", dam: true)
            button("Tang") {
                khi_nhan {
                    $dem = $dem + 1
                    neu $dem >= 10 {
                        thong_bao("Da dat 10!", kieu: thanh_cong)
                    }
                }
            }
        }
    }
}
```

Chương trình y hệt, viết bằng tiếng Anh:

```vbao
app("My app") {
    page("/") {
        box(padding: 16) {
            text("Count: ", bold: true)
            button("Increment") {
                on_click {
                    $count = $count + 1
                    if $count >= 10 {
                        notify("Reached 10!", type: success)
                    }
                }
            }
        }
    }
}
```

Từ khoá tiếng Việt và tiếng Anh đều resolve về cùng một AST — viết
bằng ngôn ngữ nào tuỳ thích, trộn lẫn thoải mái, đổi qua lại bất cứ
lúc nào.

## Mục tiêu

ViBao tồn tại để thử một ý tưởng mà hầu hết ngôn ngữ khác chưa làm:
**viết code UI bằng từ khoá của chính ngôn ngữ mình, không chỉ tiếng
Anh.** Tiếng Việt và tiếng Anh là những gì đã có hiện tại, nhưng không
phải giới hạn cuối — lớp locale được thiết kế để bất kỳ ngôn ngữ nào
cũng có thể thêm vào mà không đụng vào lõi compiler (xem
[Từ khoá đa ngôn ngữ](#từ-khoá-đa-ngôn-ngữ)).

Ngoài locale, ViBao cũng là nơi để thử nghiệm cú pháp và tính năng mà
các framework lớn chưa làm — những ý tưởng không nhất thiết phải giống
"thêm một bản clone của React". Nếu bạn có ý tưởng lạ về cách code UI
nên hoạt động, một cú pháp khác thường, hay một thư viện built-in nhỏ
mà chưa ai làm — dự án này muốn là sân chơi cho những thứ đó, và rất
hoan nghênh đóng góp theo hướng này, không chỉ dừng ở sửa bug.

Đây cũng là một project cá nhân để học compiler và language design
công khai, nên hãy kỳ vọng nhịp độ và quy mô của một side project, chứ
không phải một toolchain có công ty đứng sau.

## Từ khoá đa ngôn ngữ

ViBao không có ý định chỉ dừng lại ở tiếng Việt/tiếng Anh. Lớp locale
(`vibaoc/src/locale/`) được thiết kế tách biệt hoàn toàn khỏi phần còn
lại của compiler: lexer, parser, codegen, và validator chỉ làm việc với
các định danh ngữ nghĩa không phụ thuộc ngôn ngữ (`Tag`, `PropKey`,
`ActionName`, `FunctionName`, định nghĩa trong `vibao-ast`). Mỗi ngôn
ngữ chỉ đơn giản là một bảng ánh xạ từ ngữ bề mặt sang các định danh đó
— hiện có `vi.rs` và `en.rs`, với cùng cấu trúc sẵn sàng cho `ja.rs`,
`es.rs`, `fr.rs`,... trong tương lai.

Cụ thể, để thêm một locale từ khoá mới cần:
1. Viết file `vibaoc/src/locale/<lang>.rs` mới (kèm `<lang>_action.rs`
   / `<lang>_function.rs` / `<lang>_prop.rs`) ánh xạ từ ngữ của ngôn
   ngữ đó sang các giá trị `Tag`/`PropKey`/`ActionName`/`FunctionName`
   đã có sẵn — không có ngữ nghĩa mới, chỉ là tên gọi mới cho cùng khái
   niệm.
2. Đăng ký trong `vibaoc/src/locale/mod.rs` và
   `vibaoc/src/lexer/tables.rs`, theo đúng pattern đã dùng cho tiếng
   Việt và tiếng Anh.
3. Thêm test resolution xác nhận từ khoá của locale mới map về đúng
   AST như các locale hiện có (xem các module test `*_vi.rs` /
   `*_en.rs` để copy pattern).

Lúc runtime, lexer luôn kiểm tra bảng của locale đang active *cùng
với* tiếng Anh (tiếng Anh là locale fallback toàn cục), nên thêm ngôn
ngữ mới không bao giờ làm hỏng file `.vbao` đã có. Tiếng Việt và tiếng
Anh là 2 ngôn ngữ duy nhất hỗ trợ hiện tại — rất hoan nghênh đóng góp
thêm locale mới, xem [CONTRIBUTING.md](../CONTRIBUTING.md).

## Vì sao chọn ViBao?

- **Compile ra web app tĩnh.** Output là HTML/CSS/JS thuần, mở trực
  tiếp trong trình duyệt hoặc deploy ở bất kỳ đâu.
- **Runtime nhỏ gọn.** State, reactivity, routing, và event được xử lý
  bởi một WebAssembly runtime nhẹ thay vì một framework JS lớn.
- **Báo lỗi lúc compile, không phải lúc chạy production.** Action
  không xác định, function không xác định, và vài lỗi phổ biến khác
  được compiler bắt ngay với thông báo rõ ràng và vị trí trong source.

## Bắt đầu nhanh

Tải bản build sẵn cho hệ điều hành của bạn từ
[Releases](https://github.com/giabaovu375-alt/Vibaoc/releases) — không
cần Rust hay WASM tooling — hoặc cài bằng 1 lệnh:

```bash
curl -fsSL https://raw.githubusercontent.com/giabaovu375-alt/Vibaoc/main/scripts/install.sh | sh
```

Xem chi tiết đầy đủ (kể cả cài trên Android qua Termux) tại
[`docs/INSTALLATION.md`](INSTALLATION.md).

Sau đó compile 1 file `.vbao`:

```bash
vibaoc app.vbao
# hoặc, viết đầy đủ:
vibaoc build app.vbao --out dist
```

Mở `dist/index.html` trong trình duyệt để xem kết quả.

Thêm ví dụ mẫu ở [`examples/`](../examples/): 1 bộ đếm
(`counter.vbao`), 1 app nhiều trang có component tái sử dụng
(`multi_page.vbao`), và 1 app quản lý công việc nhỏ có CRUD trên mảng
(`task_manager.vbao`).

## Cấu trúc repo

```text
.
├── vibao-ast/        # AST dùng chung và định danh ngữ nghĩa
├── vibaoc/           # Compiler CLI (lexer, parser, resolver,
│                      # validator, codegen) + test end-to-end
├── vibao-runtime/    # Runtime chạy trong trình duyệt, compile ra WASM
├── docs/             # Tài liệu ngôn ngữ và dự án
└── scripts/          # Script build, release, install, verify
```

## Build từ source

Yêu cầu:
- Rust và Cargo
- Target `wasm32-unknown-unknown` của Rust (chỉ cần khi build runtime
  cho trình duyệt)
- `wasm-bindgen-cli` — `scripts/build-runtime.sh` tự cài đúng phiên
  bản đã khoá trong `Cargo.lock` nếu chưa có

```bash
cargo test --workspace          # chạy toàn bộ test suite
bash scripts/build-runtime.sh   # build runtime package (WASM)
bash scripts/build-release.sh 0.1.0   # build release archive cho platform hiện tại
```

Xem [`scripts/RELEASE.md`](../scripts/RELEASE.md) để biết quy trình
release đầy đủ.

## Tài liệu ngôn ngữ

| Tài liệu | Mô tả |
|---|---|
| [`SYNTAX_VI.md`](SYNTAX_VI.md) | Cheat-sheet cú pháp tiếng Việt |
| [`SYNTAX_EN.md`](SYNTAX_EN.md) | Cheat-sheet cú pháp tiếng Anh |
| [`VIBAO_SPEC.md`](VIBAO_SPEC.md) | Đặc tả ngôn ngữ đầy đủ |
| [`LIMITATIONS.md`](LIMITATIONS.md) | Phần chưa hoàn thiện/có giới hạn ở bản 0.1.0 |
| [`INSTALLATION.md`](INSTALLATION.md) | Hướng dẫn cài đặt chi tiết (mọi nền tảng) |

## Ví dụ dài hơn một chút

Một app bộ đếm nhỏ với state, điều kiện, và vòng lặp:

```vbao
lang = "vi";

ung_dung("Vi du ViBao") {
    trang("/") {
        state $dem = 0
        state $tasks = [
            {id: 1, tieu_de: "Viet tai lieu", xong: false},
            {id: 2, tieu_de: "Sua loi giao dien", xong: true}
        ]

        khoi(dem: 24, huong: cot, khoang_chu: 12) {
            text("Bo dem: $dem", co: 20, dam: true)
            button("Tang") {
                khi_nhan { $dem = $dem + 1 }
            }

            neu $dem >= 10 {
                text("Da dat 10!", mau: xanh_la)
            } khong_thi {
                text("Chua den 10", mau: xam)
            }

            vong_lap $task trong $tasks {
                text($task.tieu_de)
            }
        }
    }
}
```

## Trạng thái dự án

ViBao 0.1.0 là bản public đầu tiên, còn sớm — dùng được cho app cơ
bản, nhưng vẫn còn nhiều điểm gồ ghề thật sự. Vài điểm đáng chú ý:

- Key option trong action (`kieu:`/`thoi_gian:` trong `thong_bao(...)`)
  hiện chỉ nhận đúng tên tiếng Việt, kể cả trong file toàn tiếng Anh —
  viết `type:`/`duration:` sẽ bị bỏ qua im lặng (không lỗi, không cảnh
  báo).
- 1 số component phức tạp (`modal`, `tabs`, `table`, `chart`,...) hiện
  chỉ là placeholder, render ra `<div>` rỗng. Dùng `@the` để tự xây
  component tương đương.
- `huong`/`gap`/`khoang_chu` không có tác dụng trên `khoi` (box) — chỉ
  có tác dụng trên `flex`/`grid`/`cuon`.

Danh sách đầy đủ, kể cả những gì đang tắt, cần thêm kiểm thử thực tế,
hoặc cố tình để sau 0.1.0 — xem [`LIMITATIONS.md`](LIMITATIONS.md)
(tiếng Anh). Bug report và feedback — về các điểm gồ ghề trên, về thiết
kế locale/từ khoá, hay bất kỳ điều gì khác — đều rất được hoan nghênh;
xem [Đóng góp](#đóng-góp).

## Testing

```bash
cargo test --workspace
```

Test suite gồm 3 lớp: unit test trong từng crate, contract test cho
shape JSON của AST, và end-to-end test chạy binary `vibaoc` thật trên
file `.vbao` thật.

## Đóng góp

Xem [CONTRIBUTING.md](../CONTRIBUTING.md) trước khi mở pull request.

## License

[MIT License](../LICENSE).
