use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
struct SignedDuration {
    secs: i64,
    nanos: i32,
}

pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    to_signed(*time)
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
    D: Deserializer<'de>,
{
    let signed = SignedDuration::deserialize(deserializer)?;
    from_signed(signed).map_err(serde::de::Error::custom)
}

fn to_signed(time: SystemTime) -> Result<SignedDuration, &'static str> {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => positive(duration),
        Err(error) => negative(error.duration()),
    }
}

fn positive(duration: Duration) -> Result<SignedDuration, &'static str> {
    Ok(SignedDuration {
        secs: i64::try_from(duration.as_secs()).map_err(|_| "время вне диапазона")?,
        nanos: i32::try_from(duration.subsec_nanos()).map_err(|_| "наносекунды вне диапазона")?,
    })
}

fn negative(duration: Duration) -> Result<SignedDuration, &'static str> {
    let secs = i64::try_from(duration.as_secs())
        .map_err(|_| "время вне диапазона")?
        .checked_neg()
        .ok_or("время вне диапазона")?;
    let nanos = i32::try_from(duration.subsec_nanos())
        .map_err(|_| "наносекунды вне диапазона")?
        .checked_neg()
        .ok_or("наносекунды вне диапазона")?;
    if nanos >= 0 {
        return Ok(SignedDuration { secs, nanos });
    }
    Ok(SignedDuration {
        secs: secs.checked_sub(1).ok_or("время вне диапазона")?,
        nanos: nanos
            .checked_add(1_000_000_000)
            .ok_or("наносекунды вне диапазона")?,
    })
}

fn from_signed(signed: SignedDuration) -> Result<SystemTime, &'static str> {
    if !(0..1_000_000_000).contains(&signed.nanos) {
        return Err("наносекунды вне диапазона");
    }
    let nanos = u32::try_from(signed.nanos).map_err(|_| "наносекунды вне диапазона")?;
    if signed.secs >= 0 {
        let secs = u64::try_from(signed.secs).map_err(|_| "время вне диапазона")?;
        return UNIX_EPOCH
            .checked_add(Duration::new(secs, nanos))
            .ok_or("время вне диапазона");
    }
    let magnitude = u64::try_from(signed.secs.checked_neg().ok_or("время вне диапазона")?)
        .map_err(|_| "время вне диапазона")?;
    let duration = if nanos == 0 {
        Duration::from_secs(magnitude)
    } else {
        Duration::new(
            magnitude.checked_sub(1).ok_or("время вне диапазона")?,
            1_000_000_000 - nanos,
        )
    };
    UNIX_EPOCH
        .checked_sub(duration)
        .ok_or("время вне диапазона")
}
