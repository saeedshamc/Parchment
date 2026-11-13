# Parchment (پارچمنت)

تبدیل فایل‌های Markdown به صفحه‌ی گرافیکی و خروجی **HTML/CSS** با قالب‌های قابل تعویض.
هسته با Rust نوشته شده و روی ترمینال، Termux، دسکتاپ، اندروید و وب یکسان کار می‌کند.
فارسی و راست‌به‌چپ (RTL) به‌صورت پیش‌فرض و خودکار پشتیبانی می‌شود.

## نصب و استفاده‌ی سریع (CLI)

```bash
cargo install --path crates/mdr-cli       # دستور parch نصب می‌شود

parch note.md                             # خروجی: note.html (CSS داخل فایل)
parch note.md -t magazine -o out/         # با قالب دیگر
parch note.md --split                     # note.html + style.css
parch docs/ -o dist/                      # تبدیل کل پوشه به یک سایت استاتیک
parch docs/ -o dist/ --watch              # با هر ذخیره دوباره می‌سازد
parch --list-templates                    # github, minimal, dark, magazine
parch --eject github -o my-theme          # کپی قالب برای ویرایش
```

گزینه‌های پرکاربرد: `--toc` فهرست مطالب، `--fa-digits` اعداد فارسی، `--dir rtl|ltr|auto`، `--lang fa`.

## Termux

```bash
pkg install rust git
cargo install --path crates/mdr-cli
termux-setup-storage
parch ~/storage/shared/note.md
```

## اپ دسکتاپ و اندروید

```bash
cargo install tauri-cli --version "^2"
cd app
cargo tauri dev            # اجرا
cargo tauri build          # ساخت فایل نصبی دسکتاپ
cargo tauri android init   # فقط بار اول (نیاز به Android SDK/NDK)
cargo tauri android build --apk
```

## نسخه‌ی وب

```bash
cargo install wasm-pack
scripts/build-web.sh
python3 -m http.server -d dist-web 8080
```

## نوشتن قالب دلخواه

راهنما در `docs/TEMPLATES.md`. خلاصه: یک پوشه با `template.html` و `style.css` بسازید
(یا با `parch --eject github -o my-theme` از قالب آماده شروع کنید) و با `-t my-theme` استفاده کنید.

مجوز: MIT
