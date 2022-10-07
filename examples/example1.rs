use std::fs::copy;
use std::fs::remove_file;
use std::path::Path;

extern crate little_exif;
use little_exif::metadata::Metadata;
use little_exif::exif_tag::ExifTag;
use little_exif::exif_tag::ExifTagGroup;

fn
main()
-> Result<(), std::io::Error>
{

	// Remove old copy and create new one for writing EXIF data to
	remove_file("examples/copy.png")?;
	remove_file("examples/copy.jpg")?;
	copy("examples/image.png", "examples/copy.png")?;
	copy("examples/image.jpg", "examples/copy.jpg")?;

	// Create metadata structs & fill them
	let mut png_data = Metadata::new();
	let mut jpg_data = Metadata::new_from_path(Path::new("examples/copy.jpg"));
	fill_metadata(&mut png_data);
	fill_metadata(&mut jpg_data);

	// Write the metadata to the copies
	png_data.write_to_file(Path::new("examples/copy.png"))?;
	jpg_data.write_to_file(Path::new("examples/copy.jpg"))?;
	
	// Read in the metadata again & print it
	println!("PNG read result:");
	for tag in Metadata::new_from_path(Path::new("examples/copy.png")).data()
	{
		println!("{:?}", tag);
	}

	println!("JPG read result:");
	for tag in Metadata::new_from_path(Path::new("examples/copy.jpg")).data()
	{
		println!("{:?}", tag);
	}

	Ok(())
}

fn
fill_metadata
(
	metadata: &mut Metadata
)
{
	// Set the ImageDescription (IFD0) an ISO (ExifIFD) tag as examples
	// as well as two (to little_exif) unknown tags
	metadata.set_tag(
		ExifTag::UnknownSTRING("test".to_string(), 0x010d, ExifTagGroup::IFD0)
	);

	metadata.set_tag(
		ExifTag::ImageDescription("-w 1000 -h 1000 --x_mid=0 --y_mid=0 -z 0.5 -i 1000 -c 8".to_string())
	);

	metadata.set_tag(
		ExifTag::ISO(vec![2022])
	);

	metadata.set_tag(
		ExifTag::UnknownSTRING("test".to_string(), 0x010c, ExifTagGroup::IFD0)
	);
}