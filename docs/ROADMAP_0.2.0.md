# Lộ trình tới 0.2.0

Mục tiêu: một web profile cá nhân tương đối tốt viết hoàn toàn bằng ViBao, đủ tốt để công bố 0.2.0.
Trạng thái dưới đây dựa trên code, không dựa trên các file `.md` cũ.

## Đã có (tới 0.1.5)

- Cú pháp khối `@màu`, `@chữ`, ... với thông báo lỗi rõ ràng (tiếng Việt và tiếng Anh); từ khoá viết có dấu hoặc không dấu.
- `public/` được chép vào `dist/`; favicon tự link; meta description, Open Graph, `og:image`, canonical.
- `giao_diện { ... }`: font, màu nền, cuộn mượt, stylesheet ngoài.
- HTML ngữ nghĩa (`h1`-`h3`, `thẻ_html`), responsive (`@di_động`, `@máy_tính_bảng`, `@máy_tính`).
- Chủ đề sáng/tối: `chủ_đề`, tự theo hệ điều hành, `đặt_chủ_đề()` nhớ lựa chọn và không nháy khi tải lại.
- `gọi_api` có `tiêu_đề`, `chạy_nền`; truy cập `$ds[0].trường`, `.rỗng`, `.độ_dài`.
- `giữ_dòng` cho chuỗi nhiều dòng.
- `vibaoc serve` để thử cục bộ; `--site-url` sinh `sitemap.xml`, `robots.txt`, `404.html`.
- Ví dụ: portfolio (theme, repo lấy từ GitHub) và chat gọi Groq.

## Cần làm tới 0.2.0, theo thứ tự đề xuất

1. **Tách `@khoảng_cách`.** Đưa viền, bo góc, bóng, `overflow`, `tầng_z` sang `@viền` và `@hiệu_ứng`; `@khoảng_cách`
   chỉ còn đệm và lề. Đổi cú pháp nên làm trước khi công bố.
2. **Scroll spy cho menu** và bộ icon (hiện dùng `image` với SVG trong `public/`).
3. **Open Graph theo từng route** và kiểm chứng host dưới đường dẫn con (`user.github.io/repo/`).
4. **Quay về chế độ theo hệ điều hành** sau khi đã chọn theme bằng tay.
5. **`activate_page` idempotent.** Kích hoạt lại cùng một trang không được render lặp.
6. **Menu mobile tốt hơn.** Tự đóng khi cuộn trang, hoặc menu nổi.
7. **Từ khoá chỉ có tiếng Việt** (`trong`, `từ`, `đến`, `@di_động`, `.rỗng`, ...) cho file viết bằng tiếng Anh.
8. **Tag còn là placeholder** (`modal`, `tabs`, `table`, ...): chọn tag nào thật sự cần.
9. **Phát hành 0.2.0:** nâng version, cập nhật `CHANGELOG.md`, kiểm tra toàn bộ ví dụ trên trình duyệt.

## Chưa quyết định

- Huỷ lời gọi `gọi_api` chạy nền khi người dùng rời trang.
- Sự kiện "sau khi hiện" riêng thay cho việc dùng `chạy_nền`.
