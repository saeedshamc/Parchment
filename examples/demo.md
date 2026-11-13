---
title: نمونه‌ی Parchment
---

# Parchment؛ رندر مارک‌داون

این یک **نمونه‌ی فارسی** است که با `parch` به HTML تبدیل می‌شود. متن راست‌به‌چپ، کد چپ‌به‌راست و اعداد ۱۲۳ یا 123 هر دو درست نمایش داده می‌شوند.

## امکانات

- پشتیبانی کامل از **RTL** و فارسی
- جدول، چک‌لیست، پاورقی و ~~خط‌خورده~~
- رنگی‌سازی کد

### جدول

| قالب | سبک | تاریک |
|:-----|:----|:-----:|
| github | شبیه گیت‌هاب | خودکار |
| minimal | خوانا | خودکار |
| dark | تیره | همیشه |

### چک‌لیست

- [x] هسته‌ی Rust
- [x] ابزار خط فرمان
- [ ] نسخه‌ی اندروید

## کد

```rust
fn main() {
    let name = "Parchment";
    println!("Hello, {name}!"); // سلام
}
```

```python
def greet(name: str) -> str:
    return f"Hello {name}"
```

> نقل‌قول فارسی: «سادگی، نهایت پیچیدگی است.»

## English section

This paragraph is English, so it is left-aligned automatically thanks to `dir="auto"`.[^1]

1. First
2. Second
   - nested item
3. Third

[^1]: Footnotes work as well.

## پیوندها

به [راهنما](guide/intro.md) و [سایت](https://example.com) سر بزنید.
