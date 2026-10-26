# Triển khai lên host tĩnh

## Lệnh build

```
vibaoc build app.vbao --out dist --site-url https://ten-mien.com
```

Không có `--site-url`, `dist/` chỉ gồm `index.html`, `style.css`, `app.js`, runtime và nội dung `public/`.

## File được sinh

| File | Điều kiện | Nội dung |
| --- | --- | --- |
| `404.html` | có từ 2 route trở lên | bản sao `index.html`, để host tĩnh trả SPA khi tải lại một route như `/about` |
| `sitemap.xml` | có `--site-url` | mọi route tĩnh (bỏ route có `:tham_số`) |
| `robots.txt` | có `--site-url` | cho phép mọi bot và trỏ tới `sitemap.xml` |
| thẻ `canonical`, `og:url`, `twitter:card` | có `--site-url` | trong `<head>` của `index.html` |
| thẻ `og:image` | có `--site-url` và `public/og-image.(png/jpg/jpeg/webp)` | URL tuyệt đối tới ảnh đó |

`robots.txt` và `404.html` do build ghi trước, nên file cùng tên trong `public/` sẽ ghi đè khi chép.

## Hạn chế hiện tại

- Mọi route dùng chung một `index.html`, nên tiêu đề và thẻ Open Graph giống nhau cho cả trang. Open Graph theo từng route chưa làm.
- Router dùng đường dẫn thật (`history.pushState`). Chưa kiểm chứng khi đặt dưới đường dẫn con như `https://user.github.io/ten-repo/`; dùng tên miền riêng hoặc `user.github.io` gốc sẽ an toàn hơn.
- `404.html` chỉ hữu ích với host dùng `404.html` làm trang dự phòng (GitHub Pages, Cloudflare Pages, Netlify). Host khác cần cấu hình "mọi đường dẫn trả về `index.html`".
