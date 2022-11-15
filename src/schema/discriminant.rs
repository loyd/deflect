#[derive(Clone)]
pub struct Discriminant<R>
where
    R: crate::gimli::Reader<Offset = usize>,
{
    r#type: super::DiscriminantType,
    align: u64,
    location: crate::gimli::AttributeValue<R>,
}

impl<R> Discriminant<R>
where
    R: crate::gimli::Reader<Offset = usize>,
{
    pub(crate) fn from_dw_tag_enumeration_type<'dwarf>(
        dwarf: &'dwarf crate::gimli::Dwarf<R>,
        unit: &'dwarf crate::gimli::Unit<R, usize>,
        entry: &'dwarf crate::gimli::DebuggingInformationEntry<'dwarf, 'dwarf, R>,
    ) -> Result<Self, crate::Error> {
        assert_eq!(entry.tag(), crate::gimli::DW_TAG_enumeration_type);

        let r#type = crate::get_type(&entry)
            .map(|offset| unit.entry(offset).unwrap())
            .map(|entry| super::DiscriminantType::from_die(dwarf, unit, entry))
            .unwrap();

        let align = crate::get_align(&entry)?.unwrap();
        let location = crate::gimli::AttributeValue::Udata(0);

        Ok(Self {
            r#type,
            align,
            location,
        })
    }

    pub(crate) fn from_dw_tag_variant_part<'dwarf>(
        dwarf: &'dwarf crate::gimli::Dwarf<R>,
        unit: &'dwarf crate::gimli::Unit<R, usize>,
        entry: &'dwarf crate::gimli::DebuggingInformationEntry<'dwarf, 'dwarf, R>,
    ) -> Result<Self, crate::Error> {
        assert_eq!(entry.tag(), crate::gimli::DW_TAG_variant_part);

        let dw_tag_member = crate::get_attr_ref(&entry, crate::gimli::DW_AT_discr)
            .unwrap()
            .and_then(|offset| unit.entry(offset).ok())
            .unwrap();

        let r#type = unit
            .entry(crate::get_type(&dw_tag_member).unwrap())
            .map(|entry| super::DiscriminantType::from_die(dwarf, unit, entry))
            .expect("no entry");

        let align = crate::get_align(&entry).unwrap().unwrap_or(1);

        let location = dw_tag_member
            .attr_value(crate::gimli::DW_AT_data_member_location)
            .unwrap()
            .unwrap();

        Ok(Self {
            r#type,
            align,
            location,
        })
    }

    pub fn r#type(&self) -> &super::DiscriminantType {
        &self.r#type
    }

    pub fn alignment(&self) -> usize {
        self.align as _
    }

    pub fn location(&self) -> &crate::gimli::AttributeValue<R> {
        &self.location
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DiscriminantValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
}

impl<R> From<crate::gimli::AttributeValue<R>> for DiscriminantValue
where
    R: crate::gimli::Reader<Offset = usize>,
{
    fn from(value: crate::gimli::AttributeValue<R>) -> Self {
        match value {
            crate::gimli::AttributeValue::Data1(value) => Self::U8(value),
            crate::gimli::AttributeValue::Data2(value) => Self::U16(value),
            crate::gimli::AttributeValue::Data4(value) => Self::U32(value),
            crate::gimli::AttributeValue::Data8(value) => Self::U64(value),
            crate::gimli::AttributeValue::Udata(value) => Self::U8(value as _),
            _ => todo!(),
        }
    }
}
