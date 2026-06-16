use taudio::{
    Real,
    automation::{CurveMapping, Mapping},
};

fn main() {
    let mapping = CurveMapping::Exp(220.0, 440.0);
    let note_names = [
        "A3", "A#3", "B3", "C4", "C#4", "D4", "D#4", "E4", "F4", "F#4", "G4", "G#4", "A4",
    ];

    for (semitone, name) in note_names.into_iter().enumerate() {
        let normalized = semitone as Real / 12.0;
        let hz = mapping.map(normalized);

        println!("{name:3} is {hz:.3} Hz");
    }
}
