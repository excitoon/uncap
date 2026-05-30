use anyhow::{Context, Result};
use regex::Regex;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use url::Url;

/// A discovered captcha on the web.
#[derive(Debug, Clone)]
pub struct CaptchaFound {
    /// URL of the page containing the captcha.
    pub page_url: String,
    /// Direct URL to the captcha resource (image/audio).
    pub captcha_url: String,
    /// How it was detected.
    pub detection: Detection,
}

#[derive(Debug, Clone)]
pub enum Detection {
    /// img tag with captcha-related attributes.
    ImgTag,
    /// URL path contains captcha-related keywords.
    UrlPattern,
    /// Form with known captcha structure.
    FormPattern,
}

/// Crawl starting from seed URLs, return found captchas.
pub fn crawl(seeds: &[&str], max_pages: usize) -> Result<Vec<CaptchaFound>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (compatible; uncap-crawler/0.1)")
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = seeds.iter().map(|s| s.to_string()).collect();
    let mut found: Vec<CaptchaFound> = Vec::new();

    while let Some(url) = queue.pop() {
        if visited.len() >= max_pages {
            break;
        }
        if visited.contains(&url) {
            continue;
        }
        visited.insert(url.clone());

        eprintln!("[crawl] {url}");

        let html = match fetch_page(&client, &url) {
            Ok(h) => h,
            Err(_) => continue,
        };

        // Detect captchas on this page
        let captchas = detect_captchas(&url, &html);
        found.extend(captchas);

        // Extract links for further crawling (same domain only)
        if let Ok(base) = Url::parse(&url) {
            let links = extract_links(&base, &html);
            for link in links {
                if !visited.contains(&link) {
                    queue.push(link);
                }
            }
        }
    }

    Ok(found)
}

fn fetch_page(client: &Client, url: &str) -> Result<String> {
    let resp = client.get(url).send().context("fetch")?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !ct.contains("text/html") {
        anyhow::bail!("not HTML");
    }
    Ok(resp.text()?)
}

fn detect_captchas(page_url: &str, html: &str) -> Vec<CaptchaFound> {
    let mut results = Vec::new();
    let document = Html::parse_document(html);

    // Pattern 1: <img> tags with captcha in src, class, id, alt, or name
    let img_sel = Selector::parse("img").unwrap();
    let captcha_re =
        Regex::new(r"(?i)captcha|capcha|verify|security.?code|vcode|checkcode").unwrap();

    for el in document.select(&img_sel) {
        let attrs = ["src", "data-src", "class", "id", "alt", "name"];
        let mut is_captcha = false;
        let mut src = None;

        for attr in &attrs {
            if let Some(val) = el.value().attr(attr) {
                if *attr == "src" || *attr == "data-src" {
                    src = Some(val.to_string());
                }
                if captcha_re.is_match(val) {
                    is_captcha = true;
                }
            }
        }

        // Also check if src URL contains captcha keywords
        if let Some(ref s) = src {
            if captcha_re.is_match(s) {
                is_captcha = true;
            }
        }

        if is_captcha {
            if let Some(s) = src {
                let captcha_url = resolve_url(page_url, &s);
                results.push(CaptchaFound {
                    page_url: page_url.to_string(),
                    captcha_url,
                    detection: Detection::ImgTag,
                });
            }
        }
    }

    // Pattern 2: any URL in the page matching captcha patterns
    let url_re = Regex::new(r#"(?i)(https?://[^\s"'<>]+captcha[^\s"'<>]*)"#).unwrap();
    for caps in url_re.captures_iter(html) {
        let url = caps[1].to_string();
        if !results.iter().any(|r| r.captcha_url == url) {
            results.push(CaptchaFound {
                page_url: page_url.to_string(),
                captcha_url: url,
                detection: Detection::UrlPattern,
            });
        }
    }

    // Pattern 3: <input> with captcha-related name inside a <form>
    let input_sel = Selector::parse("form input").unwrap();
    for el in document.select(&input_sel) {
        let name = el.value().attr("name").unwrap_or("");
        let id = el.value().attr("id").unwrap_or("");
        if captcha_re.is_match(name) || captcha_re.is_match(id) {
            // Look for a sibling/nearby img
            // For now, just note the form has a captcha field
            // (the img detection above likely already caught the image)
            if results.is_empty() {
                results.push(CaptchaFound {
                    page_url: page_url.to_string(),
                    captcha_url: page_url.to_string(),
                    detection: Detection::FormPattern,
                });
            }
            break;
        }
    }

    results
}

fn extract_links(base: &Url, html: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let a_sel = Selector::parse("a[href]").unwrap();
    let mut links = Vec::new();

    for el in document.select(&a_sel) {
        if let Some(href) = el.value().attr("href") {
            if let Ok(resolved) = base.join(href) {
                // Same domain only
                if resolved.host_str() == base.host_str() {
                    let mut url = resolved.to_string();
                    // Strip fragment
                    if let Some(pos) = url.find('#') {
                        url.truncate(pos);
                    }
                    links.push(url);
                }
            }
        }
    }

    links
}

fn resolve_url(page_url: &str, src: &str) -> String {
    if src.starts_with("http://") || src.starts_with("https://") {
        return src.to_string();
    }
    if let Ok(base) = Url::parse(page_url) {
        if let Ok(resolved) = base.join(src) {
            return resolved.to_string();
        }
    }
    src.to_string()
}

/// Download a captcha image/audio from the given URL.
pub fn download(url: &str) -> Result<Vec<u8>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (compatible; uncap/0.1)")
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let resp = client.get(url).send().context("download captcha")?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }
    Ok(resp.bytes()?.to_vec())
}
