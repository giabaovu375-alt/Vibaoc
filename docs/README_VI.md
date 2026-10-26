# ViBao

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![Release](https://img.shields.io/github/v/release/giabaovu375-alt/Vibaoc)](https://github.com/giabaovu375-alt/Vibaoc/releases)

**ViBao** là ngôn ngữ mô tả giao diện web ưu tiên tiếng Việt, kèm compiler. Mã nguồn `.vbao` được biên dịch thẳng ra
HTML, CSS, JS và một runtime WebAssembly nhỏ. Không cần Node.js, bundler hay framework JavaScript.

> English: [`README.md`](../README.md)

```vbao
lang = "vi";

ứng_dụng("Ứng dụng của tôi") {
    trang("/") {
        state $đếm = 0

        khối {
            @khoảng_cách { đệm: 16 }
            text("Số đếm: $đếm", đậm: true)
            button("Tăng") {
                khi_nhấn {
                    $đếm = $đếm + 1
                    nếu $đếm >= 10 {
                        thông_báo("Đã đạt 10!", kiểu: thành_công)
                    }
                }
            }
        }
    }
}
```

Từ khoá viết có dấu hoặc không dấu đều được, kết quả giống hệt nhau. Từ khoá tiếng Anh cũng được chấp nhận và có thể
trộn trong cùng một file.

## Tính năng

- **Biên dịch ra web tĩnh**: `index.html`, `style.css`, `app.js` và runtime WASM; mở trực tiếp hoặc đưa lên host tĩnh.
- **Reactive nhẹ**: `state`, biểu thức, `nếu`, `vòng_lặp`, two-way binding cho `input`, router nhiều trang.
- **Chủ đề sáng/tối**: khai báo bằng `chủ_đề`, tự theo hệ điều hành, đổi bằng `đặt_chủ_đề()` và nhớ lựa chọn.
- **Gọi API**: `gọi_api` có `tiêu_đề`, đọc phản hồi `$phản_hồi.choices[0].message.content`, chạy nền không chặn trang.
- **Lỗi lúc biên dịch**: action lạ, hàm lạ, prop sai khối, biến theme dùng sai đều báo lỗi kèm vị trí (tiếng Việt hoặc tiếng Anh).
- **Triển khai**: `--site-url` sinh `sitemap.xml`, `robots.txt`, canonical, `og:image`; `404.html` cho SPA.
- **Đa ngôn ngữ từ khoá**: lớp locale tách riêng, thêm ngôn ngữ mới không đụng lõi compiler.

## Bắt đầu nhanh

Tải file chạy cho nền tảng của bạn ở trang [Releases](https://github.com/giabaovu375-alt/Vibaoc/releases)
(Linux, macOS, Windows, Android qua Termux), hoặc cài bằng một lệnh:

```bash
curl -fsSL https://raw.githubusercontent.com/giabaovu375-alt/Vibaoc/main/scripts/install.sh | sh
```

Chi tiết từng nền tảng và Termux: [`INSTALLATION.md`](INSTALLATION.md).

```bash
vibaoc check app.vbao                    # chỉ kiểm tra lỗi
vibaoc build app.vbao --out dist         # biên dịch ra dist/
vibaoc serve app.vbao --port 8000        # build, mở server cục bộ, tự build lại khi sửa file
```

Mở `dist/index.html` hoặc `http://127.0.0.1:8000` để xem kết quả.

## Ví dụ trong repo

Các ví dụ chỉ nằm trên GitHub, không đóng gói vào bản release.

| File | Nội dung |
|---|---|
| [`examples/multi_page.vbao`](../examples/multi_page.vbao) | Nhiều trang và component dùng lại |
| [`examples/task_manager.vbao`](../examples/task_manager.vbao) | Danh sách việc với thêm/xóa/cập nhật |
| [`examples/portfolio.vbao`](../examples/portfolio.vbao) | Web profile: theme sáng/tối, menu di động, form, repo lấy từ GitHub |
| [`examples/chat_groq.vbao`](../examples/chat_groq.vbao) | Chat gọi Groq API (điền khóa API trước khi chạy) |

`chat_groq.vbao` cần khóa API của bạn. Khóa viết trong nguồn sẽ lộ ở trình duyệt, chỉ dùng khi thử cục bộ và không đưa
file đã điền khóa lên repo công khai.

## Cấu trúc repo

```text
.
├── vibao-ast/        # AST và các kiểu ngữ nghĩa dùng chung
├── vibaoc/           # Compiler CLI (lexer, parser, validator, codegen) và test end-to-end
├── vibao-runtime/    # Runtime chạy trong trình duyệt, biên dịch ra WASM
├── examples/         # Ví dụ
├── docs/             # Tài liệu
└── scripts/          # Build, đóng gói, cài đặt, kiểm tra
```

## Build từ source

Cần Rust và Cargo. Cần thêm target `wasm32-unknown-unknown` để build runtime; `scripts/build-runtime.sh` tự cài
`wasm-bindgen-cli` đúng phiên bản nếu thiếu.

```bash
bash scripts/build-runtime.sh       # build runtime (WASM); bắt buộc trước khi chạy test e2e
cargo test --workspace              # chạy toàn bộ test
bash scripts/verify.sh              # fmt, test và build với cảnh báo bị coi là lỗi
bash scripts/build-release.sh 0.1.5 # đóng gói bản release cho nền tảng hiện tại
```

Quy trình phát hành đầy đủ: [`../scripts/RELEASE.md`](../scripts/RELEASE.md).

## Triển khai

```bash
vibaoc build app.vbao --out dist --site-url https://ten-mien.com
```

Xem [`TRIEN_KHAI.md`](TRIEN_KHAI.md) cho các file được sinh ra và hạn chế hiện tại.

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [`SYNTAX_VI.md`](SYNTAX_VI.md) | Cú pháp tiếng Việt: khối, state, action, theme, component |
| [`SYNTAX_EN.md`](SYNTAX_EN.md) | Cú pháp tiếng Anh |
| [`PROFILE_GUIDE.md`](PROFILE_GUIDE.md) | Làm một web profile: tài nguyên, font, neo, form |
| [`TRIEN_KHAI.md`](TRIEN_KHAI.md) | Triển khai lên host tĩnh |
| [`INSTALLATION.md`](INSTALLATION.md) | Cài đặt trên từng nền tảng |
| [`VIBAO_SPEC.md`](VIBAO_SPEC.md) | Đặc tả ngôn ngữ đầy đủ |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Kiến trúc compiler và runtime |
| [`LIMITATIONS.md`](LIMITATIONS.md) | Phần chưa hoàn thiện hoặc không làm gì (tiếng Anh) |
| [`ROADMAP_0.2.0.md`](ROADMAP_0.2.0.md) | Lộ trình tới 0.2.0 |
| [`../CHANGELOG.md`](../CHANGELOG.md) | Lịch sử thay đổi |

## Hạn chế hiện tại

ViBao 0.1.5 còn sớm, dùng được cho web nhỏ nhưng còn những chỗ chưa hoàn thiện:

- Một số tag (`modal`, `tabs`, `table`, `chart`, ...) vẫn là placeholder, chỉ sinh `<div>` rỗng. Dùng `@thẻ` để tự dựng.
- `@khoảng_cách` đang chứa cả viền, bóng, bo góc và `overflow`, không chỉ đệm và lề.
- Mọi route dùng chung một `index.html`, nên Open Graph giống nhau cho cả trang.
- Sau khi chọn theme bằng tay, chưa có cách quay về chế độ theo hệ điều hành.
- Khóa API viết trong nguồn sẽ lộ ở trình duyệt.

Danh sách đầy đủ: [`LIMITATIONS.md`](LIMITATIONS.md). Lỗi và góp ý đều được hoan nghênh.

## Kiểm thử

- **Unit test** trong từng crate: lexer, parser, validator, codegen, runtime.
- **Contract test** (`vibao-ast/tests/serde_contract.rs`): cố định dạng JSON của AST giữa compiler và runtime.
- **End-to-end** (`vibaoc/tests/e2e_*.rs`): chạy binary `vibaoc` thật trên file `.vbao` thật và kiểm tra
  `index.html`, `app.js`, `style.css` sinh ra.

## Đóng góp

Xem [`../CONTRIBUTING.md`](../CONTRIBUTING.md) trước khi mở pull request.

## License

[MIT](../LICENSE).
