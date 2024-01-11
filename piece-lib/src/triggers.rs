use crate::{
    protogen::{
        self,
        triggers::{Location, TriggerSource},
    },
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct Trigger {
    pub(crate) source: protobuf::EnumOrUnknown<TriggerSource>,
    pub(crate) from: protobuf::EnumOrUnknown<Location>,
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::triggers::Trigger> for Trigger {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::triggers::Trigger) -> Result<Self, Self::Error> {
        Ok(Self {
            source: value.source,
            from: value.from,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}
