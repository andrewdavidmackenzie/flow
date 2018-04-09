use std::fs::File;

pub struct WriteBitmap;

impl Implementation for WriteBitmap {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        let filename = inputs.remove(0).remove(0);
        let bytes = inputs.remove(0).remove(0);
        let bounds = inputs.remove(0).remove(0);

        // bounds: (usize, usize),
        let width = point["width"].as_u64().unwrap() as usize;
        let height = point["height"].as_u6464().unwrap() as usize;

        write_bitmap(Path::from(filename), bytes, (width, height));

        true
    }
}


/// Write the buffer 'pixels', whose dimensions are given by 'bounds', to the file named 'filename'
fn write_bitmap(filename: &PathBuf, pixels: &[u8], bounds: (usize, usize)) {
    let output = File::create(filename).unwrap();

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32, ColorType::Gray(8))?;
}