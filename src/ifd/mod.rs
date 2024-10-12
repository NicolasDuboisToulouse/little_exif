// Copyright © 2024 Tobias J. Prisching <tobias.prisching@icloud.com> and CONTRIBUTORS
// See https://github.com/TechnikTobi/little_exif#license for licensing details

use std::io::Cursor;
use std::io::Read;
use std::io::Seek;

use crate::endian::*;
use crate::exif_tag::ExifTag;
use crate::exif_tag::TagType;
use crate::exif_tag_format::ExifTagFormat;
use crate::exif_tag_format::INT16U;
use crate::general_file_io::io_error;
use crate::u8conversion::from_u8_vec_macro;
use crate::u8conversion::U8conversion;

/// Useful constants for dealing with IFDs: The length of a single IFD entry is
/// equal to 12 bytes, as the entry consists of the tags hex value (2 byte), 
/// the format (2 byte), the number of components (4 byte) and the value/offset
/// section (4 byte).
/// The four zeros tell us that this is the last IFD in its sequence and there
/// is no link to another IFD
const IFD_ENTRY_LENGTH: u32     = 12;
const IFD_END_NO_LINK:  [u8; 4] = [0x00, 0x00, 0x00, 0x00];

/// The different types of Image File Directories (IFD). A generic IFD is one
/// without further specialization, like e.g. IFD0. The generic IFDs start
/// with IFD0, which is located via the offset at the start of the TIFF data. 
/// The next IFD (in this case: IFD1) is then located via the link offset at
/// the end of IFD0. 
/// Other IFDs, like e.g. the ExifIFD, are linked via offset tags (in case of 
/// the ExifIFD offset: 0x8769) that are located in the respective generic IFD 
/// (most of them in IFD0).
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
#[allow(non_snake_case, non_camel_case_types)]
pub enum
ExifTagGroup
{
	GENERIC,
	EXIF,
	INTEROP,
	GPS,
	NO_GROUP
}

/*

LEGACY VERSION

#[allow(non_camel_case_types)]
#[derive(Debug, Eq, PartialEq, PartialOrd, Hash, Clone, Copy)]
pub enum
ExifTagGroup
{
	NO_GROUP,
	IFD0,
		ExifIFD,
			InteropIFD,
			MakerNotesIFD,
		GPSIFD,
	IFD1,
	Other,
}

*/


/// The value of `belongs_to_generic_ifd_nr` tells us what generic IFD this
/// specific IFD belongs to, e.g. `0` would indicate that it belongs (or is)
/// IFD0. 
pub struct
ImageFileDirectory
{
	tags:                      Vec<ExifTag>,
	ifd_type:                  ExifTagGroup,
	belongs_to_generic_ifd_nr: u32,
}

impl
ImageFileDirectory
{
	/// If everything goes Ok and there is enough data to unpack, this returns
	/// the offset to the next generic IFD that needs to be processed.
	pub(crate) fn
	decode_ifd
	(
		data_cursor:         &mut Cursor<Vec<u8>>,
		data_begin_position:      usize,                                        // Stays the same for all calls to this function while decoding
		endian:              &    Endian,
		group:               &    ExifTagGroup,
		generic_ifd_nr:           u32,                                          // Reuse value for recursive calls; only gets incremented by caller
		insert_into:         &mut Vec<ImageFileDirectory>,                      // Stays the same for all calls to this function while decoding
	)
	-> Result<Option<Vec<usize>>, std::io::Error>
	{
		// Backup the entry position where this IFD started
		let data_cursor_entry_position = data_cursor.position();

		// Check if there is enough data to decode an IFD
		if (data_cursor.get_ref().len() as i64 - data_cursor_entry_position as i64) < 6i64
		{
			return Ok(None);
		}

		// The first two bytes give us the number of entries in this IFD
		let mut number_of_entries_buffer = vec![0u8; 2];
		data_cursor.read_exact(&mut number_of_entries_buffer)?;
		let number_of_entries = from_u8_vec_macro!(u16, &number_of_entries_buffer.to_vec(), endian);

		// Check that there is enough data to unpack
		if (0
			+ 2
			+ IFD_ENTRY_LENGTH as usize * number_of_entries as usize 
			+ IFD_END_NO_LINK.len()
		) <= (
			data_cursor.get_ref().len() as i64 - data_cursor_entry_position as i64
		) as usize
		{
			return io_error!(Other, "Not enough data to decode IFD!");
		}

		// Temporarily storing specific tags that have been decoded
		// This has to do with data offset tags that are interconnected with
		// other tags.
		// For example, for decoding the StripOffsets we also need the 
		// StripByteCounts to know how many bytes each strip has
		let mut strip_tags: (Option<ExifTag>, Option<ExifTag>) = (None, None);

		// Storing all tags while decoding
		let mut tags = Vec::new();

		// loop through the entries - assumes that the value stored in
		// `number_of_entries` is correct
		for i in 0..number_of_entries
		{
			// Read the entry into a buffer
			let mut entry_buffer = vec![0u8; IFD_ENTRY_LENGTH as usize];
			data_cursor.read_exact(&mut entry_buffer)?;

			// Decode the first 8 bytes with the tag, format and component number
			let hex_tag              = from_u8_vec_macro!(u16, &entry_buffer[0..2].to_vec(), endian);
			let hex_format           = from_u8_vec_macro!(u16, &entry_buffer[2..4].to_vec(), endian);
			let hex_component_number = from_u8_vec_macro!(u32, &entry_buffer[4..8].to_vec(), endian);

			// Decode the format
			// TODO: What to do in case these two differ but the given format
			// can be casted into the expected one, e.g. R64U to R64S?
			let format;
			if let Some(decoded_format) = ExifTagFormat::from_u16(hex_format)
			{
				format = decoded_format;
			}
			else
			{
				return io_error!(Other, format!("Illegal format value: {}", hex_format));
			}

			// Calculating the number of required bytes to determine if next
			// 4 bytes are data or an offset to data
			// Note: It is expected that the format here is "correct" in the
			// sense that it tells us whether or not an offset is used for the
			// data even if the given format in the image file is not the
			// right/default one for the currently processed tag according to 
			// the exif specification. 
			let byte_count = format.bytes_per_component() * hex_component_number;

			let mut value_buffer = vec![0u8; 4];
			data_cursor.read_exact(&mut value_buffer)?;

			let raw_data;
			if byte_count > 4
			{
				// Compute the offset
				let hex_offset = from_u8_vec_macro!(u32, &value_buffer, endian);

				// Backup current position & go to offset position
				let backup_position = data_cursor.position();
				data_cursor.set_position(data_begin_position as u64);
				data_cursor.seek_relative(hex_offset as i64)?;

				// Read the raw data
				let mut raw_data_buffer = vec![0u8; byte_count as usize];
				data_cursor.read_exact(&mut raw_data_buffer)?;
				raw_data = raw_data_buffer.to_vec();
			
				// Rewind the cursor to the start of the next entry
				data_cursor.set_position(backup_position);
			}
			else
			{
				// The 4 bytes are the actual data
				// Note: This may actually be *less* than 4 bytes! This is why
				// The second index isn't just entry_start_index+12
				let mut raw_data_buffer = vec![0u8; byte_count as usize];
				data_cursor.read_exact(&mut raw_data_buffer)?;
				raw_data = raw_data_buffer.to_vec();
			}

			// Try to get the tag via its hex value
			let tag_result = ExifTag::from_u16(hex_tag, group);

			// Start of by checking if this is an unknown tag
			if tag_result.is_err()
			{
				tags.push(ExifTag::from_u16_with_data(
					hex_tag, 
					&format, 
					&raw_data, 
					&endian, 
					group
				).unwrap());
				continue;
			}

			// We can now safely unwrap the result as it can't be an error
			let mut tag = tag_result.unwrap();

			// If this is an IFD offset tag, perform a recursive call
			if let TagType::IFD_OFFSET(subifd_group) = tag.get_tag_type()
			{
				let offset = from_u8_vec_macro!(u32, &raw_data, endian) as usize;

				let subifd_decode_result = Self::decode_ifd(
					data_cursor,
					data_begin_position,
					endian,
					&subifd_group,
					generic_ifd_nr,
					insert_into,
				);

				if let Ok(subifd_result) = subifd_decode_result
				{
					assert_eq!(subifd_result, None);
					continue;
				}
				else
				{
					return io_error!(Other, format!("Could not decode SubIFD:\n  {}", subifd_decode_result.err().unwrap()));
				}
			}

			// At this point we check if the format is actually what we expect
			// it to be.
			let mut decoded_u32_data = None;
			if tag.format().as_u16() != format.as_u16()
			{
				// The expected format and the given format in the file
				// do *not* match. Check special cases (INT16U -> INT32U)
				// If no special cases match, return an error
				if 
					tag.format() == ExifTagFormat::INT32U &&
					format       == ExifTagFormat::INT16U
				{
					let int16u_data = <INT16U as U8conversion<INT16U>>::from_u8_vec(&raw_data, endian);
					let int32u_data = int16u_data.into_iter().map(|x| x as u32).collect::<Vec<u32>>();

					decoded_u32_data = Some(int32u_data);

					tags.push(tag.set_value_to_int32u_vec(int32u_data).unwrap());
					continue;
				}
				// Other special cases
				else
				{
					return io_error!(Other, format!("Illegal format for known tag! Tag: {:?} Expected: {:?} Got: {:?}", tag, tag.format(), format));
				}
			}





			// // If this is a known tag ...
			// if let Ok(tag) = ExifTag::from_u16(hex_tag, group)
			// {
			// 	// ... check its type
			// 	match tag.get_tag_type()
			// 	{
			// 		TagType::VALUE => {
			// 			()
			// 		},

			// 		TagType::IFD_OFFSET(exif_tag_group) => {

			// 		},

			// 		TagType::DATA_OFFSET => {
			// 			// Most difficult case. 
			// 			// Problem: In case of e.g. StripOffsets and
			// 			// StripByteCounts both of them are needed the same
			// 			// time for decoding...
			// 			// Idea: Post pone them after the end of the loop
			// 			match tag
			// 			{
			// 				ExifTag::StripByteCounts(_) => {

			// 				},

			// 				_ => {
			// 					todo!()
			// 				}
			// 			}
			// 		},
			// 	}
			// }

			




			
			

		}
		


		todo!()
	}
}