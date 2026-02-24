.PHONY: all build wasm demo clean

# ── Native binary ────────────────────────────────────────────────────────────

all: build

## Build the native `bucl` binary (debug).
build:
	cargo build

## Build the native `bucl` binary (release).
release:
	cargo build --release

# ── WASM ─────────────────────────────────────────────────────────────────────

## Build the WebAssembly module and JS glue into docs/demo/wasm/pkg/.
##
## Prerequisites:
##   rustup target add wasm32-unknown-unknown
##   cargo install wasm-pack
##
## The resulting bucl_wasm_bg.wasm + bucl_wasm.js are loaded by
## docs/demo/wasm/index.html.  Alternatively, build the raw .wasm without
## wasm-pack:
##   cargo build --target wasm32-unknown-unknown --profile wasm-release --lib
##   cp target/wasm32-unknown-unknown/wasm-release/bucl_wasm.wasm docs/demo/wasm/pkg/bucl_wasm.wasm
wasm:
	wasm-pack build \
	  --target web \
	  --out-dir docs/demo/wasm/pkg \
	  --profile wasm-release \
	  -- --no-default-features

## Same as `wasm` but skips wasm-opt (faster iteration).
wasm-dev:
	wasm-pack build \
	  --target web \
	  --out-dir docs/demo/wasm/pkg \
	  --dev \
	  -- --no-default-features

## Build raw .wasm without wasm-pack (no JS glue generated; demo uses its own).
wasm-raw:
	cargo build \
	  --target wasm32-unknown-unknown \
	  --profile wasm-release \
	  --lib
	mkdir -p docs/demo/wasm/pkg
	cp target/wasm32-unknown-unknown/wasm-release/bucl_wasm.wasm docs/demo/wasm/pkg/bucl_wasm.wasm
	@echo "WASM written to docs/demo/wasm/pkg/bucl_wasm.wasm"
	@echo "Serve the demo with:  python3 -m http.server --directory docs/demo/"

# ── Demo server ───────────────────────────────────────────────────────────────

## Serve the demo site on http://localhost:8000
## The prebuilt .wasm is already checked in; rebuild with `make wasm-raw`.
demo:
	python3 -m http.server --directory docs/demo/ 8000

# ── Cleanup ───────────────────────────────────────────────────────────────────

clean:
	cargo clean
	rm -rf docs/demo/wasm/pkg
