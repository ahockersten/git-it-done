## Running

```bash
cargo leptos watch
```

## Compiling for Release

```bash
cargo leptos build --release
```

Will generate your server binary in target/server/release and your site package in target/site

## Testing

```bash
cargo leptos end-to-end
```

```bash
cargo leptos end-to-end --release
```

## Building and Running with Docker

```bash
docker build .
```

## Run the Docker container locally

```bash
docker run -p 8080:8080 -e PORT=8080 git-it-done
```
