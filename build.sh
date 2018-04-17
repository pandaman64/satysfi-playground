(cd client && \
    cargo web build --target=wasm32-unknown-unknown --release)
cp client/target/wasm32-unknown-unknown/release/client.js client/target/wasm32-unknown-unknown/release/client.wasm server/assets
sed -i 's/client.wasm/\/client.wasm/' server/assets/client.js
(cd server && \
    cargo build)
