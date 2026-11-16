# PixelPick

**Color science engine for Orbit: HEX/RGB/HSL conversions and palettes.**

> Orbit companion module — phase 2/4 of the [Orbit ecosystem](https://github.com/Can-5/orbit).
> Standalone crate: usable without Orbit, without other modules, and without network.

## Implemented now

- Rgb/Hsl roundtrip conversions with exact round-trip guarantees tested
- parse_color() accepting #rgb, #rrggbb, rgb() functions; Palette struct + nearest-color search (squared sRGB distance)

## Not implemented yet (explicitly out of scope for this crate)

- No eyedropper, no screen sampling — that requires ScreenCapture permission and lives in the Orbit host
- No color-space management (P3 etc.) yet

## Build & test

```sh
cargo test  # 7 tests
```

Dependencies: serde, serde_json, thiserror. No cross-repo path dependencies, no Git dependencies, no downloads at build time beyond crates.io.

## Privacy

Pure math, no I/O. No network, no storage.

## License

MIT © The Orbit contributors. See [LICENSE](./LICENSE).
