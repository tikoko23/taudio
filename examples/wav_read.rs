use std::{
    error::Error,
    fs::File,
    io::{BufReader, BufWriter, Seek, SeekFrom},
};

use taudio::wav::WavFile;

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(not(target_endian = "little"))]
    compile_error!(
        "In order to keep the example simple, proper endianness handling hasn't been implemented"
    );

    let file = File::options()
        .write(true)
        .read(true)
        .open("examples/sine440hz.wav")?;

    let mut reader = BufReader::new(file);
    let mut wav = WavFile::read(&mut reader)?;

    if let Some(data_chk) = wav.get_data_chunk_mut() {
        let bytes = data_chk.data.to_mut();
        let samples: &mut [i16] = bytemuck::cast_slice_mut(bytes);

        for sample in samples {
            *sample *= -1;
        }
    } else {
        eprintln!("Data chunk not found in wave file");
        std::process::exit(1);
    }

    let mut file = reader.into_inner();

    file.seek(SeekFrom::Start(0))?;
    wav.write(&mut BufWriter::new(file))?;

    Ok(())
}
