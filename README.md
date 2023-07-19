Substrate API Explorer
======================

A small app for exploring substrate-based API's.




## Deployment


To trigger CD and deploy a new version on Github releases:

```
$ git tag v{version}
$ git push origin {version}
```




## Bundling Locally

To bundle the distributive locally:

```
$ cargo build --release
$ cargo run --release -p bundler
```

The artifacts will be placed in `target/release/bundle` directory.
