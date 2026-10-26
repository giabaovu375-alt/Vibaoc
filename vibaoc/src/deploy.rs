use std::path::Path;

use crate::codegen::css::esc_attr;

const OG_IMAGE_NAMES: [&str; 4] = [
    "og-image.png",
    "og-image.jpg",
    "og-image.jpeg",
    "og-image.webp",
];

pub fn normalize_site_url(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

pub fn find_og_image(public_dir: &Path) -> Option<String> {
    OG_IMAGE_NAMES
        .iter()
        .find(|name| public_dir.join(name).is_file())
        .map(|name| name.to_string())
}

pub fn head_fragment(site_url: &str, og_image: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "\n  <link rel=\"canonical\" href=\"{}/\">",
        esc_attr(site_url)
    ));
    out.push_str(&format!(
        "\n  <meta property=\"og:url\" content=\"{}/\">",
        esc_attr(site_url)
    ));
    match og_image {
        Some(name) => {
            out.push_str(&format!(
                "\n  <meta property=\"og:image\" content=\"{}/{}\">",
                esc_attr(site_url),
                esc_attr(name)
            ));
            out.push_str("\n  <meta name=\"twitter:card\" content=\"summary_large_image\">");
        }
        None => out.push_str("\n  <meta name=\"twitter:card\" content=\"summary\">"),
    }
    out
}

fn is_static_route(route: &str) -> bool {
    !route.contains(':') && !route.contains('*')
}

pub fn sitemap_xml(site_url: &str, routes: &[&String]) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for route in routes.iter().filter(|r| is_static_route(r)) {
        let loc = if route.as_str() == "/" {
            format!("{}/", site_url)
        } else {
            format!("{}{}", site_url, route)
        };
        out.push_str(&format!("  <url><loc>{}</loc></url>\n", xml_escape(&loc)));
    }
    out.push_str("</urlset>\n");
    out
}

pub fn robots_txt(site_url: &str) -> String {
    format!(
        "User-agent: *\nAllow: /\n\nSitemap: {}/sitemap.xml\n",
        site_url
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_bo_dau_gach_cuoi_va_khoang_trang() {
        assert_eq!(normalize_site_url(" https://a.com/x/ "), "https://a.com/x");
        assert_eq!(normalize_site_url("https://a.com"), "https://a.com");
    }

    #[test]
    fn head_co_canonical_og_url_va_the_twitter_theo_anh_og() {
        let co_anh = head_fragment("https://a.com", Some("og-image.png"));
        assert!(
            co_anh.contains("<link rel=\"canonical\" href=\"https://a.com/\">"),
            "{}",
            co_anh
        );
        assert!(
            co_anh.contains("<meta property=\"og:url\" content=\"https://a.com/\">"),
            "{}",
            co_anh
        );
        assert!(
            co_anh.contains("<meta property=\"og:image\" content=\"https://a.com/og-image.png\">"),
            "{}",
            co_anh
        );
        assert!(co_anh.contains("summary_large_image"), "{}", co_anh);

        let khong_anh = head_fragment("https://a.com", None);
        assert!(!khong_anh.contains("og:image"), "{}", khong_anh);
        assert!(khong_anh.contains("content=\"summary\""), "{}", khong_anh);
    }

    #[test]
    fn sitemap_bo_route_dong_va_giu_thu_tu() {
        let a = "/".to_string();
        let b = "/du-an".to_string();
        let c = "/du-an/:id".to_string();
        let xml = sitemap_xml("https://a.com", &[&a, &b, &c]);
        assert!(xml.contains("<loc>https://a.com/</loc>"), "{}", xml);
        assert!(xml.contains("<loc>https://a.com/du-an</loc>"), "{}", xml);
        assert!(!xml.contains(":id"), "{}", xml);
        assert!(xml.starts_with("<?xml"), "{}", xml);
    }

    #[test]
    fn robots_tro_toi_sitemap() {
        assert_eq!(
            robots_txt("https://a.com"),
            "User-agent: *\nAllow: /\n\nSitemap: https://a.com/sitemap.xml\n"
        );
    }
}
