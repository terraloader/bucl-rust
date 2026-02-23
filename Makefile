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

## Build the WebAssembly module and JS glue into demo/pkg/.
##
## Prerequisites:
##   rustup target add wasm32-unknown-unknown
##   cargo install wasm-pack
##
## The resulting demo/pkg/bucl_wasm_bg.wasm + bucl_wasm.js are loaded by
## demo/index.html.  Alternatively, build the raw .wasm without wasm-pack:
##   cargo build --target wasm32-unknown-unknown --profile wasm-release --lib
##   cp target/wasm32-unknown-unknown/wasm-release/bucl_wasm.wasm demo/pkg/bucl_wasm.wasm
wasm:
	wasm-pack build \
	  --target web \
	  --out-dir demo/pkg \
	  --profile wasm-release \
	  -- --no-default-features

## Same as `wasm` but skips wasm-opt (faster iteration).
wasm-dev:
	wasm-pack build \
	  --target web \
	  --out-dir demo/pkg \
	  --dev \
	  -- --no-default-features

## Build raw .wasm without wasm-pack (no JS glue generated; demo uses its own).
wasm-raw:
	cargo build \
	  --target wasm32-unknown-unknown \
	  --profile wasm-release \
	  --lib
	mkdir -p demo/pkg
	cp target/wasm32-unknown-unknown/wasm-release/bucl_wasm.wasm demo/pkg/bucl_wasm.wasm
	@echo "WASM written to demo/pkg/bucl_wasm.wasm"
	@echo "Serve the demo with:  python3 -m http.server --directory demo/"

# ── Demo server ───────────────────────────────────────────────────────────────

## Serve the demo site on http://localhost:8000
## Run `make wasm-raw` (or `make wasm`) first.
demo:
	python3 -m http.server --directory demo/ 8000

# ── Cleanup ───────────────────────────────────────────────────────────────────

clean:
	cargo clean
	rm -rf demo/pkg
