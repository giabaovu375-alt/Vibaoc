# Lịch sử thay đổi

## 0.1.5

### Thêm

- **Chủ đề sáng/tối**: khai báo `chủ_đề tên { $biến = giá_trị }`, dùng `$biến` làm giá trị prop kiểu dáng. Theme đầu tiên là
  mặc định; theme tên `toi` hoặc `dark` tự áp dụng khi hệ điều hành bật chế độ tối. Có tầng kiểm tra riêng với thông báo
  lỗi tiếng Việt.
- **`đặt_chủ_đề(tên)`** (`set_theme`): đổi theme bằng tay, không đối số thì chuyển sang theme kế tiếp. Lựa chọn lưu trong
  `localStorage` và khôi phục bằng script nhỏ trong `<head>` trước khi vẽ trang.
- **`gọi_api`**: option `tiêu_đề` (`headers`) và `chạy_nền: true` (`background: true`); thân lỗi dạng `error.message` được đọc.
- **Biểu thức `[n]`**: `$ds[0].trường` (chỉ số số nguyên viết thẳng).
- **Prop `giữ_dòng`** (`preserve_lines`): `white-space: pre-wrap` và ngắt từ dài.
- **Triển khai**: `vibaoc build --site-url <url>` sinh `sitemap.xml`, `robots.txt`, canonical, `og:url`, `og:image`,
  `twitter:card`; sinh `404.html` khi có từ 2 route.
- **`vibaoc serve`**: build, mở server cục bộ, tự build lại khi sửa file.
- **Ví dụ mới**: `portfolio.vbao` (theme, repo lấy từ GitHub) và `chat_groq.vbao`.

### Sửa

- Tên biến `$` viết có dấu (`$tên`, `$kỹ_năng`) giờ được chuẩn hoá như mọi identifier khác; nội suy chuỗi nhận tên biến có dấu.
- `căn: trái`/`phải` trong `flex` sinh đúng `justify-content` (trước đây giá trị tiếng Việt lọt thẳng ra CSS).
- CSS nền không còn mang theo comment nhà phát triển vào `style.css` của trang build ra.
- Bỏ khai báo `[profile.release]` trong `vibao-runtime/Cargo.toml`: Cargo vốn bỏ qua khai báo này ở crate không phải gốc
  workspace và in cảnh báo mỗi lần build.

### Thay đổi

- Tài liệu và ví dụ dùng cú pháp có dấu làm dạng chuẩn.
- Mã nguồn không còn comment; CSS/JS sinh ra không còn comment của compiler.
- Thêm test cho tính năng 0.1.5 (theme, API có header và chạy nền, chỉ số `[n]`, `giữ_dòng`, `--site-url`, phiên bản).
- Thông báo lỗi về range giảm dần không còn nhắc số phiên bản.

Chi tiết từng thay đổi nhỏ: [`BUGS.md`](BUGS.md).

## 0.1.0

Bản public đầu tiên.
