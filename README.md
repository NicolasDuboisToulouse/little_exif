# little\_exif

A little library for reading and writing EXIF data in pure Rust.

[![version-badge][]][version]&nbsp;
[![license-badge][]][license]&nbsp;

[version-badge]: https://img.shields.io/crates/v/little_exif.svg
[version]: https://crates.io/crates/little_exif
[license-badge]: https://img.shields.io/crates/l/little_exif.svg
[license]: https://github.com/TechnikTobi/little_exif#license

## Supported Formats
- JPEG
- JXL
- PNG
- TIFF
- WebP (only lossless and extended)

Your required format is not listed here or you've run into a problem with a file that should be supported? Open up a new issue (ideally with an example image for reproduction in case of a problem) and I'll take a look!

## Example

```rust
use little_exif::metadata::Metadata;
use little_exif::exif_tag::ExifTag;

// Image stored as a file
let image_path = std::path::Path::new("image.png");
let mut metadata = Metadata::new_from_path(&image_path);

// Alternatively, the image is stored in a Vec<u8> variable
// let file_type = FileExtension::JPEG;
// let mut metadata = Metadata::new_from_vec(&image_vector, file_type);

metadata.set_tag(
    ExifTag::ImageDescription("Hello World!".to_string())
);

metadata.write_to_file(&image_path)?;

// Or, in case of a Vec<u8>:
// metadata.write_to_vec(&mut image_vector, file_type)?;
```

## FAQ

### I tried writing the ImageDescription tag on a JPEG file, but it does not show up. Why?

This could be due to the such called APP12 or APP13 segment stored in the JPEG, likely caused by editing the file using e.g. Photoshop. These segments may store data that image viewers also interpret as an ImageDescription, overriding the EXIF tag. Right now, ```little_exif``` can't edit these segments. As a workaround, the functions ```clear_app12_segment``` and ```clear_app13_segment``` can remove these areas from the JPEG:

```rust
// File in a Vec<u8>
Metadata::clear_app13_segment(&mut file_content, file_extension)?;

// File at a given path
Metadata::file_clear_app13_segment(&given_path)?;
```


## License

Licensed under either

- Apache License, Version 2.0 (See [LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0) or
- MIT License (See [LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
