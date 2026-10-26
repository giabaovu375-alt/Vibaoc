# ViBao — Cú pháp tiếng Việt (0.1.5)

Tài liệu tra cứu cú pháp. Nguồn ngữ nghĩa là `vibao-ast`; các bảng từ khoá nằm trong `vibaoc/src/locale/`.

## Quy ước viết

- Mọi từ khoá, tên prop, tên khối và tên biến viết **có dấu hoặc không dấu đều được**, kết quả giống hệt nhau:
  `màu_nền` = `mau_nen`, `$kỹ_năng` = `$ky_nang`, `khối` = `khoi`. Tài liệu này viết có dấu.
- Chuỗi trong dấu nháy là dữ liệu, được giữ nguyên: `"lam_sang"`, `"toi"`, `"header"` phải viết đúng như bảng
  giá trị, không bỏ hay thêm dấu.
- Dòng đầu file có thể khai báo ngôn ngữ thông báo lỗi của compiler:

```vbao
lang = "vi";
```

Không khai báo `lang` thì thông báo mặc định là tiếng Anh. Từ khoá tiếng Anh vẫn được chấp nhận song song.

## Khung ứng dụng

```vbao
lang = "vi";

ứng_dụng("Ứng dụng") {
    trang("/", "Tiêu đề trang", mô_tả: "Mô tả ngắn cho công cụ tìm kiếm") {
        khối {
            @khoảng_cách { đệm: 16 }
            text("Xin chào")
        }
    }
}
```

Nhiều `trang(...)` là nhiều route; route có tham số viết dạng `/du-an/:id`.

## Tag thường dùng

`text`, `h1`, `h2`, `h3`, `p`, `nhãn`, `image`, `video`, `icon`, `button`, `input`, `link`, `liên_kết`, `flex`, `grid`,
`stack`, `khối`, `cuộn`, `căn_giữa`, `lớp`, `đính_đầu`, `đính_màn_hình`, `khoảng_cách`, `đường_kẻ`.

- `đính_đầu`: thanh dính ở đầu trang. `đính_màn_hình(vị_trí: dưới)`: thanh cố định ở mép dưới màn hình.
- `căn_giữa(max_rộng: 760)`: khung căn giữa có chiều rộng tối đa.

## Prop và khối `@tên_khối`

Prop có tên chia thành 11 nhóm. Tag **bị ép** dùng khối: `khối`, `flex`, `grid`, `input`, `vùng_văn_bản`, `chọn_một`,
`hộp_kiểm`, `lựa_chọn`. Prop có tên của chúng chỉ viết trong `@tên_khối { ... }` ở thân `{ }`. Mọi tag khác
(`text`, `button`, `image`, `căn_giữa`, ...) viết được cả hai cách, kết quả giống hệt nhau.

```vbao
khối {
    @màu { màu_nền: xám_nhạt }
    @khoảng_cách { đệm: 16, radius: 8 }
    @kích_thước { rộng: 320, cao: 120 }
    text("Nội dung", cỡ: 18, đậm: true, màu: xanh)
}

image(nguồn: "/images/logo.png", mô_tả_ảnh: "Logo")

input {
    @nhập_liệu { loại: "text", chữ_trỏ: "Tên", giá_trị: $tên }
}
```

| Khối | Prop |
|---|---|
| `@màu` | `màu_nền`, `màu` (`màu_chữ`), `màu_viền` |
| `@kích_thước` | `rộng`, `cao`, `max_rộng`, `min_rộng`, `max_cao`, `min_cao` |
| `@khoảng_cách` | `đệm`, `lề`, `radius`, `viền`, `kiểu_viền`, `bóng`, `cuộn_tràn`, `tầng_z` |
| `@chữ` | `cỡ`, `đậm`, `nghiêng`, `gạch_chân`, `giữ_dòng`, `căn`, `hàng`, `khoảng_chữ`, `biến_đổi`, `phông_chữ` |
| `@flex_bố_cục` | `hướng`, `dọc`, `bọc`, `gap`, `gap_dọc`, `gap_ngang` |
| `@grid_bố_cục` | `cột`, `hàng_lưới`, `gap`, `gap_dọc`, `gap_ngang` |
| `@vị_trí` | `tran_x`, `tran_y`, `vị_trí`, `offset` |
| `@media` | `khớp`, `nguồn`, `mô_tả_ảnh`, `tải_chậm` |
| `@nhập_liệu` | `loại`, `chữ_trỏ`, `giá_trị`, `chọn`, `bắt_buộc`, `vô_hiệu` |
| `@liên_kết` | `đến`, `mục_tiêu`, `quan_hệ` |
| `@định_danh` | `id`, `lớp`, `thẻ_html` |

Quy tắc:

- Thứ tự các khối tự do; mỗi khối chỉ khai báo **một lần** trên một tag (`@màu` và `@color` là cùng một khối).
- Prop đặt sai khối là lỗi biên dịch, thông báo chỉ ra khối đúng. `gap`, `gap_dọc`, `gap_ngang` được phép ở cả
  `@flex_bố_cục` lẫn `@grid_bố_cục`.
- `@khoảng_cách` hiện chứa cả viền, bóng, bo góc và `overflow`, không chỉ đệm và lề. Có kế hoạch tách ra (xem `ROADMAP_0.2.0.md`).
- Nội dung đứng đầu `(...)` là positional: `text("Xin chào")`.
- `hiệu_ứng_hover` và `hiệu_ứng_cuộn` luôn viết trong `(...)`, kể cả với tag bị ép.
- Với `flex`, `căn: phải` (hoặc `trái`, `giữa`) nằm trong `@chữ` và là `justify-content`; `dọc: giữa` trong
  `@flex_bố_cục` là `align-items`.
- `thẻ_html: "section"` trong `@định_danh` đổi `<div>` của tag container thành phần tử ngữ nghĩa: `section`,
  `header`, `footer`, `main`, `article`, `aside`, `nav`, `figure`, `ul`, `li`. Không dùng cho lời gọi component.
- `id` và `lớp` chỉ viết trong `@định_danh`, kể cả trên lời gọi component.
- `giá_trị: $tên` trên `input`/`vùng_văn_bản` là two-way binding với state.
- `giữ_dòng: true` giữ ký tự xuống dòng và khoảng trắng của chuỗi, và cho phép ngắt từ quá dài
  (`white-space: pre-wrap`, `overflow-wrap: anywhere`). Chỉ nhận giá trị tĩnh.

## Giao diện chung `giao_diện`

```vbao
giao_diện {
    phông_chữ: "Inter, system-ui, sans-serif"
    tải_phông: "https://fonts.googleapis.com/css2?family=Inter:wght@400;700&display=swap"
    cỡ: 16
    màu: $chữ
    màu_nền: $nền
    hàng: 1.6
    cuộn_mượt: true
    cuộn_lùi: 72
}
```

Chi tiết và cách làm một web profile: [`PROFILE_GUIDE.md`](PROFILE_GUIDE.md).

## Class động, hiệu ứng, responsive

```vbao
button("Bật", lớp: { active: $đang_chọn }, hiệu_ứng_hover: "phong_to", hiệu_ứng_cuộn: "truot_len") {
    khi_nhấn { $đang_chọn = !$đang_chọn }
}
```

- Hover: `phong_to`, `lam_sang`, `nang_len`. Cuộn/xuất hiện: `fade_in`, `truot_len`, `truot_xuong`, `phong_to`, `rung`.
- Responsive viết phẳng, không chia khối. Dùng `@di_động`, `@máy_tính_bảng`, `@máy_tính`:

```vbao
khối {
    @kích_thước { rộng: 800 }
    @di_động {
        rộng: 320
        ẩn: true
    }
}
```

Biến theme không dùng được bên trong các khối responsive.

## State, biểu thức, điều kiện, vòng lặp

```vbao
trang("/") {
    state $đếm = 0
    state $việc = [
        {id: 1, tên: "Viết tài liệu", xong: false},
        {id: 2, tên: "Sửa lỗi giao diện", xong: true}
    ]

    button("Tăng") {
        khi_nhấn {
            $đếm = $đếm + 1
            nếu $đếm >= 10 {
                thông_báo("Đủ 10!", kiểu: thành_công)
            }
        }
    }

    text("Số hiện tại: $đếm")

    nếu $việc.rỗng {
        text("Chưa có việc nào")
    } không_thì {
        vòng_lặp $v trong $việc {
            text($v.tên)
        }
    }

    vòng_lặp $i từ 1 đến 3 {
        text("Mục $i")
    }
}
```

- Trong chuỗi, `$tên` là nội suy biến. `$` không đứng trước tên biến hợp lệ được giữ nguyên (`"Giá: $50"`).
- Truy cập: `$v.tên`, `$ds[0].tên` (chỉ số số nguyên viết thẳng, không nhận biểu thức), `$ds.độ_dài`, `$ds.rỗng`.
- `vòng_lặp $i từ N1 đến N2` tăng dần và gồm cả hai đầu. `N1 > N2` là lỗi.
- Hàm biểu thức: `giá_tiền`, `ngày`, `rút_gọn`, `làm_tròn`, `phần_trăm`, `hoa_chữ`.

## Sự kiện

Sự kiện phần tử: `khi_nhấn`, `khi_gửi` (form). Sự kiện trang: `on_tải`, `on_hủy`.

## Action

`thông_báo`, `cảnh_báo`, `điều_hướng`, `mở_tab_mới`, `cuộn_đến`, `cuộn_lên_đầu`, `lưu_dữ_liệu`, `tải_dữ_liệu`,
`sao_chép`, `đặt_chủ_đề`, `gọi_api`, `thêm_vào_mảng`, `xóa_theo_id`, `cập_nhật_theo_id`. (`mở_modal` và `đóng_modal`
chưa có đích dùng được: xem `LIMITATIONS.md`.)

`thông_báo(nội dung, kiểu: ...)` nhận `thành_công`, `lỗi`, `cảnh_báo`, `info`.

### `gọi_api`

```vbao
$phản_hồi = gọi_api("POST", "https://api.example.com/chat", {model: $mô_hình, messages: $lịch_sử},
        tiêu_đề: {Authorization: "Bearer $khóa_api"}) {
    thành_công {
        thêm_vào_mảng($lịch_sử, {role: "assistant", content: $phản_hồi.choices[0].message.content})
    }
    thất_bại {
        thông_báo("Lỗi: $phản_hồi.error.message", kiểu: lỗi)
    }
}
```

- Tham số: phương thức, địa chỉ, dữ liệu (tuỳ chọn, gửi dạng JSON). Địa chỉ tương đối được nối với `baseURL`.
- `tiêu_đề: { ... }` (tiếng Anh `headers`): thêm hoặc ghi đè header, kể cả `Accept` và `Content-Type` mặc định.
- Kết quả gán vào biến ở vế trái, **cả khi thất bại** (thân lỗi). Thông báo lỗi được lấy từ `message` hoặc `error.message`.
- Khóa API viết trong nguồn sẽ lộ ở trình duyệt: chỉ dùng khi thử cục bộ.

### Tải dữ liệu lúc mở trang, không chặn giao diện

```vbao
state $repo_ds = []
state $repo_tt = "dang_tai"

on_tải {
    $repo_ds = gọi_api("GET", "https://api.github.com/users/ten/repos", chạy_nền: true) {
        thành_công { $repo_tt = "xong" }
        thất_bại { $repo_tt = "loi" }
    }
}
```

`on_tải` chạy **trước** khi trang được gắn sự kiện và hiện ra, nên một `gọi_api` thường trong đó làm cả trang chờ mạng.
`chạy_nền: true` (tiếng Anh `background: true`) cho lời gọi chạy tách rời: `on_tải` xong ngay, `thành_công` và
`thất_bại` chạy khi phản hồi về. Giá trị chỉ nhận `true` viết thẳng. Dùng một state trạng thái cùng `nếu` để hiện đang tải,
lỗi, rỗng hoặc kết quả.

## Chủ đề (theme) sáng/tối

```vbao
ứng_dụng("Ứng dụng") {
    chủ_đề sáng {
        $nền = trang
        $chữ = xám_đậm
    }
    chủ_đề tối {
        $nền = "#1a202c"
        $chữ = "#e2e8f0"
    }

    giao_diện { màu_nền: $nền, màu: $chữ }

    trang("/") {
        khối {
            @màu { màu_nền: $nền }
            text("Xin chào", màu: $chữ)
            button("Giao diện") { khi_nhấn { đặt_chủ_đề() } }
        }
    }
}
```

- Theme khai báo đầu tiên là mặc định. Theme tên `toi` hoặc `dark` tự áp dụng khi hệ điều hành bật chế độ tối.
- Mọi theme phải khai báo **cùng một tập biến**. Giá trị biến là hằng: màu, số (thành px) hoặc chuỗi.
  Chuỗi không được chứa `{ } ; < >`.
- Biến theme chỉ được dùng làm **toàn bộ giá trị** của một prop kiểu dáng (`@màu`, `@kích_thước`, `@khoảng_cách`, `@chữ`,
  `@flex_bố_cục`, `@grid_bố_cục`, `@vị_trí`) hoặc của `giao_diện`. Dùng làm nội dung, điều kiện, tham số, trong khối
  responsive, trong biểu thức hoặc nội suy chuỗi đều là lỗi. Tên biến không được trùng state, biến toàn cục, tham số
  component hay biến vòng lặp.
- `đặt_chủ_đề("toi")` đặt theme theo tên (viết không dấu, đúng như tên theme đã bỏ dấu); `đặt_chủ_đề()` chuyển sang theme
  kế tiếp, bắt đầu từ theme đang hiển thị. Tên không tồn tại, hoặc dùng khi chưa khai báo theme nào, là lỗi build.
- Lựa chọn được lưu vào `localStorage` (khoá `vb-theme`) và khôi phục bằng một script nhỏ trong `<head>` trước khi vẽ
  trang, nên không bị nháy. Script chỉ có khi khai báo từ 2 theme.
- Chưa có cách quay về "theo hệ điều hành" sau khi đã chọn tay.

## Component tự định nghĩa

```vbao
@thẻ ThẻTin(tiêu_đề: chuỗi) {
    khối {
        @khoảng_cách { đệm: 16 }
        text($tiêu_đề, đậm: true)
    }
}

ThẻTin(tiêu_đề: "Xin chào")
ThẻTin(tiêu_đề: "Có id") {
    @định_danh { id: "thẻ-có-id" }
}
```

Tham số component viết trong `(...)` với tên tự do và dùng như biến `$tên_tham_số`.

## Màu có sẵn

`trang`, `đen`, `đỏ`, `xanh`, `xanh_lá`, `vàng`, `hồng`, `tím`, `cam`, `xám`, `xám_nhạt`, `xám_đậm`, `lục`, `nâu`.
Mã hex viết trong chuỗi: `"#1a202c"`.

## Hạn chế

Các phần chưa hoàn thiện: xem [`LIMITATIONS.md`](LIMITATIONS.md) (tiếng Anh) và mục "Hạn chế hiện tại" trong
[`README_VI.md`](README_VI.md).
