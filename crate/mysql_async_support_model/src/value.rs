use chrono::{Duration, NaiveDate, NaiveDateTime};
use mysql_async::{
    prelude::{ConvIr, FromValue},
    Value as MySqlValue,
};
use serde::{Deserialize, Serialize};

/// Programmer-friendly model of MySQL [`Value`][mysql_async::Value] type.
///
/// # Note
///
/// * You must use prepared statements if you want types to be returned,
///   otherwise it is always returned as `Value::Bytes`
/// * There must be only one statement in the query -- i.e. no multiple selects.
/// * Not sure if nested select statements work.
///
/// However, I haven't managed to get MySQL to return `Value`s with proper
/// return types.
///
/// See:
///
/// * <https://github.com/go-sql-driver/mysql/issues/407#issuecomment-172583652>
/// * <https://dev.mysql.com/doc/refman/8.0/en/sql-prepared-statements.html>
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Value {
    /// Value was `NULL` in the database.
    None,
    Bytes(Vec<u8>),
    Int(i64),
    UInt(u64),
    Float(f32),
    Double(f64),
    /// `DateTime` without a time zone.
    ///
    /// In `mysql_async` stores this as `Date(u16, u8, u8, u8, u8, u8, u32)`,
    /// correspnding to:
    ///
    /// ```text
    /// year, month, day, hour, minutes, seconds, micro seconds
    /// ```
    ///
    /// If you wish to attach timezone information, such as UTC, you may do
    /// something like the following:
    ///
    /// ```rust
    /// use chrono::{NaiveDate, TimeZone, Utc};
    ///
    /// let naive_date_time = NaiveDate::from_ymd(2021, 05, 30).and_hms_micro(12, 06, 53, 445);
    /// let _utc_date_time = Utc.from_utc_datetime(&naive_date_time);
    /// ```
    ///
    /// See:
    ///
    /// * <https://docs.rs/mysql_async/latest/mysql_async/enum.Value.html#variant.Date>
    /// * <https://dev.mysql.com/doc/refman/8.0/en/datetime.html>
    Date(NaiveDateTime),
    /// Time offset or duration.
    ///
    /// In `mysql_async` stores this as `Time(bool, u32, u8, u8, u8, u32)`,
    /// correspnding to:
    ///
    /// ```text
    /// is negative, days, hours, minutes, seconds, micro seconds
    /// ```
    ///
    /// See:
    ///
    /// * <https://docs.rs/mysql_async/latest/mysql_async/enum.Value.html#variant.Time>
    /// * <https://dev.mysql.com/doc/refman/8.0/en/time.html>
    #[serde(with = "value_time_serde")]
    Time(Duration),
}

impl From<MySqlValue> for Value {
    fn from(value: MySqlValue) -> Self {
        match value {
            MySqlValue::NULL => Value::None,
            MySqlValue::Bytes(bytes) => Value::Bytes(bytes),
            MySqlValue::Int(v) => Value::Int(v),
            MySqlValue::UInt(v) => Value::UInt(v),
            MySqlValue::Float(v) => Value::Float(v),
            MySqlValue::Double(v) => Value::Double(v),
            MySqlValue::Date(year, month, day, hour, minutes, seconds, micro_seconds) => {
                Value::Date(
                    NaiveDate::from_ymd(i32::from(year), u32::from(month), u32::from(day))
                        .and_hms_micro(
                            u32::from(hour),
                            u32::from(minutes),
                            u32::from(seconds),
                            micro_seconds,
                        ),
                )
            }
            MySqlValue::Time(is_negative, days, hours, minutes, seconds, micro_seconds) => {
                let mut duration = Duration::days(i64::from(days))
                    + Duration::hours(i64::from(hours))
                    + Duration::minutes(i64::from(minutes))
                    + Duration::seconds(i64::from(seconds))
                    + Duration::microseconds(i64::from(micro_seconds));

                if is_negative {
                    duration = -duration;
                }

                Value::Time(duration)
            }
        }
    }
}

#[derive(Debug)]
pub struct ValueIr(MySqlValue);

impl ConvIr<Value> for ValueIr {
    fn new(value: MySqlValue) -> Result<ValueIr, mysql_async::FromValueError> {
        Ok(Self(value))
    }

    fn commit(self) -> Value {
        Value::from(self.0)
    }

    fn rollback(self) -> MySqlValue {
        self.0
    }
}

impl FromValue for Value {
    type Intermediate = ValueIr;
}

mod value_time_serde {
    use std::fmt;

    use chrono::Duration;
    use serde::{
        de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor},
        ser::{SerializeStruct, Serializer},
    };

    const FIELDS: &'static [&'static str] = &["secs", "nanos"];

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = *duration;
        let is_negative = duration < Duration::zero();
        let mut secs = duration.num_seconds();
        let nanos = (duration - Duration::seconds(secs))
            .num_nanoseconds()
            .expect("Nanos should not overflow as we subtracted seconds.");

        if is_negative && nanos > 0 {
            secs -= 1;
        }

        let mut state = serializer.serialize_struct("Duration", 2)?;
        state.serialize_field("secs", &secs)?;
        state.serialize_field("nanos", &nanos)?;
        state.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Duration", FIELDS, DurationVisitor)
    }

    enum Field {
        Secs,
        Nanos,
    }

    // This part could also be generated independently by:
    //
    //    #[derive(Deserialize)]
    //    #[serde(field_identifier, rename_all = "lowercase")]
    //    enum Field { Secs, Nanos }
    impl<'de> Deserialize<'de> for Field {
        fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct FieldVisitor;

            impl<'de> Visitor<'de> for FieldVisitor {
                type Value = Field;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("`secs` or `nanos`")
                }

                fn visit_str<E>(self, value: &str) -> Result<Field, E>
                where
                    E: de::Error,
                {
                    match value {
                        "secs" => Ok(Field::Secs),
                        "nanos" => Ok(Field::Nanos),
                        _ => Err(de::Error::unknown_field(value, FIELDS)),
                    }
                }
            }

            deserializer.deserialize_identifier(FieldVisitor)
        }
    }

    struct DurationVisitor;

    impl<'de> Visitor<'de> for DurationVisitor {
        type Value = Duration;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("struct Duration")
        }

        fn visit_seq<V>(self, mut seq: V) -> Result<Duration, V::Error>
        where
            V: SeqAccess<'de>,
        {
            let secs = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(0, &self))?;
            let nanos = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(1, &self))?;
            Ok(Duration::seconds(secs) + Duration::nanoseconds(nanos))
        }

        fn visit_map<V>(self, mut map: V) -> Result<Duration, V::Error>
        where
            V: MapAccess<'de>,
        {
            let mut secs = None;
            let mut nanos = None;
            while let Some(key) = map.next_key()? {
                match key {
                    Field::Secs => {
                        if secs.is_some() {
                            return Err(de::Error::duplicate_field("secs"));
                        }
                        secs = Some(map.next_value()?);
                    }
                    Field::Nanos => {
                        if nanos.is_some() {
                            return Err(de::Error::duplicate_field("nanos"));
                        }
                        nanos = Some(map.next_value()?);
                    }
                }
            }
            let secs = secs.ok_or_else(|| de::Error::missing_field("secs"))?;
            let nanos = nanos.ok_or_else(|| de::Error::missing_field("nanos"))?;

            Ok(Duration::seconds(secs) + Duration::nanoseconds(nanos))
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::Value;

    #[test]
    fn serialize_time_positive() {
        let duration = Duration::seconds(123) + Duration::nanoseconds(456);
        let value = Value::Time(duration);

        assert_eq!(
            String::from(r#"{"Time":{"secs":123,"nanos":456}}"#),
            serde_json::to_string(&value).expect("Failed to serialize `Value::Time`.")
        );
    }

    #[test]
    fn serialize_time_negative() {
        let duration = Duration::seconds(123) + Duration::nanoseconds(456);
        let value = Value::Time(-duration);

        assert_eq!(
            String::from(r#"{"Time":{"secs":-123,"nanos":-456}}"#),
            serde_json::to_string(&value).expect("Failed to serialize `Value::Time`.")
        );
    }

    #[test]
    fn deserialize_time_positive() {
        let duration = Duration::seconds(123) + Duration::nanoseconds(456);
        let value = Value::Time(duration);

        assert_eq!(
            value,
            serde_json::from_str(r#"{"Time":{"secs":123,"nanos":456}}"#)
                .expect("Failed to deserialize `Value::Time`")
        );
    }

    #[test]
    fn deserialize_time_negative() {
        let duration = Duration::seconds(123) + Duration::nanoseconds(456);
        let value = Value::Time(-duration);
        assert_eq!(
            value,
            serde_json::from_str(r#"{"Time":{"secs":-123,"nanos":-456}}"#)
                .expect("Failed to deserialize `Value::Time`")
        );
    }
}
