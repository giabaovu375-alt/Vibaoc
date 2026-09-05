## File language directive

Đặt ở đầu file nếu muốn compiler hiển thị error/warning bằng tiếng Việt:

```vbao
lang = "vi";
```

English vẫn được chấp nhận song song với tiếng Việt. Không khai báo `lang` thì diagnostics mặc định là English.

# ViBao — Syntax tiếng Việt (0.1.0)

Cheat-sheet cho surface syntax tiếng Việt. Nguồn semantic chính là
`vibao-ast` và các bảng locale trong `vibaoc/src/locale/`.

## Khung ứng dụng

```vbao
ung_dung("Ứng dụng") {
    trang("/") {
        khoi(dem: 16) {
            text("Xin chào")
        }
    }
}
```

## Tag/layout phổ biến

`text`, `h1`, `h2`, `h3`, `p`, `nhan`, `image`, `video`, `icon`, `button`,
`input`, `link`, `lien_ket`, `flex`, `grid`, `stack`, `khoi`, `cuon`,
`can_giua`, `lop`, `dinh_dau`, `dinh_man_hinh`, `khoang_cach`, `duong_ke`.

## Props

```vbao
khoi(mau_nen: xam_nhat, dem: 16, radius: 8, rong: 320, cao: 120) {
    text("Nội dung", co: 18, dam: true, mau: xanh)
}

image(nguon: "/images/logo.png", mo_ta_anh: "Logo")
input(loai: "text", chu_tro: "Tên", gia_tri: $ten)
```

`gia_tri: $ten` trên `input`/`textarea` là two-way binding cho biến trực tiếp.

## Dynamic class

```vbao
button(lop: { active: $dang_chon, muted: $im_lang }) {
    on_click { $dang_chon = !$dang_chon }
}
```

## Animation

```vbao
button(hieu_ung_hover: "phong_to", hieu_ung_cuon: "truot_len")
```

Hover: `phong_to`, `lam_sang`. Scroll/load-in: `fade_in`, `truot_len`,
`truot_xuong`, `phong_to`, `rung`.

## Responsive

```vbao
khoi(rong: 800) {
    @di_dong {
        rong: 320
        an: true
    }
}
```

## State / expression / action

```vbao
trang("/") {
    state $dem = 0
    button("Tăng") {
        on_click {
            $dem = $dem + 1
            neu $dem >= 10 {
                thong_bao("Đủ 10!", kieu: thanh_cong)
            }
        }
    }
    text("Số hiện tại: $dem")
}
```

Action chính: `thong_bao`, `canh_bao`, `dieu_huong`, `mo_tab_moi`, `mo_modal`,
`dong_modal`, `cuon_den`, `cuon_len_dau`, `luu_du_lieu`, `tai_du_lieu`,
`goi_api`, `them_vao_mang`, `xoa_theo_id`, `cap_nhat_theo_id`.

Function expression: `gia_tien`, `ngay`, `rut_gon`, `lam_tron`, `phan_tram`,
`hoa_chu`.

## Component tự định nghĩa

```vbao
@the TheCard(tieu_de: chuoi) {
    khoi(dem: 16) {
        text($tieu_de, dam: true)
    }
}
TheCard(tieu_de: "Xin chào")
```

## Màu built-in

`trang`, `den`, `do`, `xanh`, `xanh_la`, `vang`, `hong`, `tim`, `cam`, `xam`,
`xam_nhat`, `xam_dam`, `luc`, `nau`.

Các phần chưa hoàn thiện ở 0.1.0: xem [`LIMITATIONS.md`](LIMITATIONS.md)
(tiếng Anh) hoặc mục "Hạn chế hiện tại" trong
[`docs/README_VI.md`](README_VI.md).


## Template string và range

- Trong chuỗi template, `$ten` là nội suy biến. Dấu `$` đứng trước số hoặc
  không đứng trước tên biến hợp lệ được giữ nguyên như ký tự literal (ví dụ
  `"Giá: $50"`).
- `vong_lap $i tu N1 den N2` là range tăng dần, bao gồm cả hai đầu. Nếu
  `N1 > N2`, compiler báo lỗi; range giảm dần chưa được hỗ trợ trong 0.1.0.
- Tên property ở option `trang(..., ...)` dùng chung locale resolver, nên
  `mau_nen` và `background_color` đều được chấp nhận.
