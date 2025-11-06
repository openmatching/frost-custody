# Vendor Dependencies

Third-party libraries bundled with the extension.

## Files

### tailwind.min.css
- **Version**: 3.4.1
- **Purpose**: CSS utility framework
- **Source**: https://cdn.jsdelivr.net/npm/tailwindcss@3.4.1/dist/tailwind.min.css
- **License**: MIT

### lucide.min.js
- **Version**: Latest
- **Purpose**: SVG icon library
- **Source**: https://unpkg.com/lucide@latest/dist/umd/lucide.min.js
- **License**: ISC

## Why Bundled?

1. **CSP Compliance**: No external script sources
2. **Privacy**: No external requests
3. **Offline**: Works without internet
4. **Performance**: No CDN latency

## Updating

Download updates via `build.sh` (automatic) or manually:

```bash
# Tailwind
curl -sL https://cdn.jsdelivr.net/npm/tailwindcss@3.4.1/dist/tailwind.min.css -o vendor/tailwind.min.css

# Lucide
curl -sL https://unpkg.com/lucide@latest/dist/umd/lucide.min.js -o vendor/lucide.min.js
```

## Note

**Pure Rust stack**: All logic (UI, P2P, crypto) is Rust/WASM. No JavaScript runtime dependencies.
