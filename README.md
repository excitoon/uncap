# uncap

un-cap(tcha) — like uncorking a bottle.

Solve image captchas locally. No paid services, no API keys — just OCR.

## How it works

1. Load image
2. Preprocess — grayscale → scale 2× → binary threshold → auto-invert
3. OCR via [Tesseract](https://github.com/tesseract-ocr/tesseract)

## Prerequisites

```sh
# macOS
brew install tesseract

# Ubuntu/Debian
apt install tesseract-ocr

# Windows
choco install tesseract
```

## Usage

```sh
# Solve a captcha image
uncap captcha.png

# Pipe from stdin
curl -s https://example.com/captcha.png | uncap -

# Custom page segmentation mode
uncap --psm 8 captcha.png
```

## Build

```sh
cargo build --release
```

## License

MIT
