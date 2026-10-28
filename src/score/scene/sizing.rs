use super::*;
pub fn scene(m: &mut Music, id: u64, meters: Vec<Meter>) -> Result<(), String> {
    crate::score::pattern::body::meter_music(&meters, m.division)?;
    let mut c = read(m)?;
    c.scenes
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or("Missing Scene")?
        .meters = meters;
    write(m, &c)
}
pub fn song(m: &mut Music, measures: u64) -> Result<(), String> {
    let mut c = read(m)?;
    c.song_length = if measures == 0 {
        0
    } else {
        let time = MusicalTime::new(m);
        same_measure::bounds(time.map(0).ok_or("Missing Song meter")?, measures.into())?.1
    };
    write(m, &c)
}
