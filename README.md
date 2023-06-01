# paths

Simple way to semantically require relative or absolute paths.

## tl;dr;

There are three main classes in this crate that have various constraints that are validated by their constructors. See `try_new` on each of these classes.

Each set of constraints has a `*Path` and `*PathBuf` variant, mirroring what is in the standard lib:

- `RelativePath` / `RelativePathBuf`: These paths must not start with `/`
- `AbsolutePath` / `AbsolutePathBuf`: These paths must start with a `/` and be a complete path.
- `CombinedPath` / `CombinedPathBuf`: These paths may be either a relative or absolute one.

## Adding to a project

```toml
# Cargo.toml
[dependencies]
paths = { git = "https://github.com/nataliejameson/paths", tag = "0.1.0" }
```

## Extra features

If the `serde` feature is enabled, a serialization / deserialization impl is made available that also validates path constraints on deserialization.
If the `diesel` feature is enabled, a field type is added that allows serialization and deserialization in Diesel (`ToSql`/`FromSql` impls are provided)

## Random notes

For `CombinedPath`, `CombinedPath::try_into_absolute()` and `CombinedPath::try_into_absolute_in_cwd()` can be used to either return an absolute path, or resolve it against an absolute path if the inner data is a relative path.

Because `FromStr` is implemented, things like `CombinedPathBuf` can be used by `clap`.

Like `Path`, the `*Path` implementations are ~zero size containers over the inner `Path` reference object. It's similar in the case of `PathBuf` variants.

These types all can deref into Path like structs, so pretty much any stdlib functions should work with them.
