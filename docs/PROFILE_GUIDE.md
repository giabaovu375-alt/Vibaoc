# Làm một web profile bằng ViBao

Hướng dẫn ngắn dựa trên `examples/portfolio.vbao`. Mọi mục dưới đây đều
có trong ví dụ đó; xem file này để lấy cú pháp chạy được.

## Cấu trúc thư mục

```
site/
  app.vbao
  public/            # chép nguyên vào dist/ khi build
    favicon.svg      # tự được link vào <head> (cũng nhận .png, .ico)
    avatar.svg
    cv.pdf
```

Build và xem thử:

```
vibaoc serve site/app.vbao
```

Lệnh này build vào `dist/`, chạy server tại `http://127.0.0.1:8000/` và
tự build lại khi bạn sửa file `.vbao` hoặc file trong `public/` (tải lại
trang bằng Ctrl+Shift+R để xem). Tuỳ chọn: `--port 3000`, `--host 0.0.0.0`
(để mở từ điện thoại cùng mạng), `--out thu-muc`, `--no-watch`. Route nội bộ
không có đuôi file (ví dụ `/du-an/3`) được trả về `index.html`.

Chỉ build, không chạy server: `vibaoc build site/app.vbao --out dist`. Khi
đó nên mở qua một server tĩnh; bản build tải WASM bằng `fetch`, và trình
duyệt có thể chặn việc này khi mở thẳng `file://`.

## Font, màu nền và cuộn mượt: `giao_dien`

```vbao
ung_dung("Portfolio") {
    giao_dien {
        phong_chu: "Inter, system-ui, sans-serif"
        tai_phong: "https://fonts.googleapis.com/css2?family=Inter:wght@400;700&display=swap"
        co: 16
        mau: xam_dam
        mau_nen: trang
        hang: 1.6
        cuon_muot: true
        cuon_lui: 72
    }
    ...
}
```

| Key | Tác dụng |
|---|---|
| `phong_chu` | `font-family` của `body` |
| `co` | cỡ chữ gốc (px) |
| `mau`, `mau_nen` | màu chữ và màu nền của trang |
| `hang` | `line-height` |
| `tai_phong` | URL stylesheet (vd Google Fonts) được thêm vào `<head>` |
| `cuon_muot` | `true` để các link `#...` cuộn mượt |
| `cuon_lui` | khoảng chừa phía trên khi cuộn tới `#id`, để menu dính không che tiêu đề (px) |

Tên tiếng Anh: `global_style`, `font_family`, `font_size`, `color`,
`background_color`, `line_height`, `font_url`, `smooth_scroll`,
`scroll_offset`. Giá trị phải là tĩnh. Key lạ là lỗi build. Chỉ khai báo
được một khối `giao_dien`, và chỉ trong file chính.

## Điều hướng trong trang

```vbao
dinh_dau(offset: 0) {
    thanh_dieu_huong(dem: 16, huong: hang, gap: 24, boc: true, mau_nen: trang) {
        link("Giới thiệu", den: "#gioi-thieu")
        link("GitHub", den: "https://github.com", muc_tieu: "_blank", quan_he: "noreferrer")
    }
}

flex {
    @dinh_danh { id: "gioi-thieu" }
    ...
}
```

`den` bắt đầu bằng `#` là cuộn trong trang, bắt đầu bằng `/` là route nội
bộ. Mọi thứ khác (`https://`, `mailto:`, file tương đối như `cv.pdf`)
do trình duyệt xử lý.

## Menu cho điện thoại

Ba breakpoint không chồng nhau (`@di_dong` dưới 640px, `@may_tinh_bang`
640-1023px, `@may_tinh` từ 1024px), nên dùng `an: true` để chọn phần tử
nào hiện ở cỡ nào. Mẫu trong `portfolio.vbao`: danh sách link ngang ẩn trên
điện thoại, nút "Menu" ẩn trên màn hình lớn, và một danh sách dọc chỉ hiện
khi `$menu_mo` bật:

```vbao
state $menu_mo = false

flex {
    @di_dong { an: true }
    link("Dự án", den: "#du-an")
}
button("Menu") {
    @may_tinh_bang { an: true }
    @may_tinh { an: true }
    khi_nhan { $menu_mo = !$menu_mo }
}
neu $menu_mo {
    flex {
        @may_tinh_bang { an: true }
        @may_tinh { an: true }
        link("Dự án", den: "#du-an") { khi_nhan { $menu_mo = false } }
    }
}
```

Lưu ý: menu mở nằm trong header dính, nên nó cao thêm bao nhiêu thì che
bấy nhiêu nội dung khi cuộn. Giữ danh sách gọn (xếp hàng ngang, cho phép
xuống dòng) thay vì xếp dọc từng mục, và đóng menu khi bấm một link như mẫu.
Runtime chưa có sự kiện cuộn của cả trang để tự đóng menu.

## HTML ngữ nghĩa

Dùng `h1`/`h2`/`h3` cho tiêu đề (không phải `text` với cỡ chữ lớn) và
`the_html` để container thành `header`, `main`, `section`, `footer`:

```vbao
flex {
    @dinh_danh { id: "ky-nang", the_html: "section" }
    h2("Kỹ năng", co: 24)
}
```

Việc này giúp công cụ tìm kiếm và trình đọc màn hình hiểu cấu trúc trang.

## Dữ liệu và danh sách

`state` chứa mảng, `vong_lap` sinh phần tử; `den: $bien` bind `href` động:

```vbao
state $du_an = [{ten: "A", url: "https://example.com"}]
vong_lap $d trong $du_an {
    link($d.ten, den: $d.url)
}
```

## Responsive

`@di_dong`, `@may_tinh_bang`, `@may_tinh` nhận: `cot`, `huong`, `co`,
`rong`, `cao`, `dem`, `an`, `gap`, `max_rong`, `le`. `huong` nhận `cot`
hoặc `hang`.

## Hiệu ứng

`hieu_ung_hover`: `phong_to`, `lam_sang`, `nang_len`.
`hieu_ung_cuon`: `fade_in`, `truot_len`, `truot_xuong`, `phong_to`, `rung`.

## Form liên hệ

Site tĩnh không có backend, nên gửi form tới một dịch vụ ngoài (ví dụ
Formspree) bằng `goi_api`. Thay `THAY_BANG_ID_CUA_BAN` trong ví dụ bằng id
form của bạn:

```vbao
khi_gui {
    goi_api("POST", "https://formspree.io/f/XXXX", { name: $ten, email: $email, message: $tin_nhan }) {
        thanh_cong { thong_bao("Đã gửi!", kieu: thanh_cong) }
        that_bai { thong_bao("Chưa gửi được.", kieu: loi) }
    }
}
```

Dịch vụ phải cho phép CORS từ domain của bạn và nhận JSON.

## Chưa có

- Cách trở lại chế độ theo hệ điều hành sau khi đã chọn theme bằng tay.
- Bộ icon; dùng chữ hoặc ký tự Unicode.
- Meta Open Graph riêng cho từng route (chỉ có của trang `/`).
- Chế độ tối, đổi theme và cách triển khai lên host tĩnh: xem [`SYNTAX_VI.md`](SYNTAX_VI.md) (mục Chủ đề)
  và [`TRIEN_KHAI.md`](TRIEN_KHAI.md).
