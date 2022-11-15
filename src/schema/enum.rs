use std::{borrow::Cow, fmt};

/// A sum type; e.g., a Rust-style enum.
pub struct Enum<'dwarf, R: crate::gimli::Reader<Offset = usize>>
where
    R: crate::gimli::Reader<Offset = usize>,
{
    dwarf: &'dwarf crate::gimli::Dwarf<R>,
    unit: &'dwarf crate::gimli::Unit<R, usize>,
    entry: crate::gimli::DebuggingInformationEntry<'dwarf, 'dwarf, R>,
    name: R,
    discriminant: super::Discriminant<R>,
}

impl<'dwarf, R> fmt::Debug for Enum<'dwarf, R>
where
    R: crate::gimli::Reader<Offset = usize>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_tuple(&self.name().unwrap());
        self.variants(|variant| {
            ds.field(&variant);
        });
        ds.finish()
    }
}

impl<'dwarf, R> Enum<'dwarf, R>
where
    R: crate::gimli::Reader<Offset = usize>,
{
    pub(crate) fn from_dw_tag_enumeration_type(
        dwarf: &'dwarf crate::gimli::Dwarf<R>,
        unit: &'dwarf crate::gimli::Unit<R, usize>,
        entry: crate::gimli::DebuggingInformationEntry<'dwarf, 'dwarf, R>,
    ) -> Result<Self, crate::Error> {
        crate::check_tag(&entry, crate::gimli::DW_TAG_enumeration_type)?;
        let name = crate::get_name(&entry, dwarf, unit)?;
        let discriminant = super::Discriminant::from_dw_tag_enumeration_type(dwarf, unit, &entry)?;
        Ok(Self {
            dwarf,
            unit,
            entry,
            name,
            discriminant,
        })
    }

    pub(crate) fn from_dw_tag_structure_type(
        dwarf: &'dwarf crate::gimli::Dwarf<R>,
        unit: &'dwarf crate::gimli::Unit<R, usize>,
        entry: crate::gimli::DebuggingInformationEntry<'dwarf, 'dwarf, R>,
    ) -> Result<Self, crate::Error> {
        crate::check_tag(&entry, crate::gimli::DW_TAG_structure_type)?;
        let name = crate::get_name(&entry, dwarf, unit)?;
        let discriminant = 'variant: {
            let mut tree = unit.entries_tree(Some(entry.offset()))?;
            let root = tree.root()?;
            let mut children = root.children();
            while let Some(child) = children.next()? {
                let entry = child.entry();
                if child.entry().tag() == crate::gimli::DW_TAG_variant_part {
                    break 'variant super::Discriminant::from_dw_tag_variant_part(
                        dwarf, unit, entry,
                    )?;
                }
            }
            return Err(crate::ErrorKind::MissingChild {
                tag: crate::gimli::DW_TAG_variant_part,
            })?;
        };

        Ok(Self {
            dwarf,
            unit,
            entry,
            name,
            discriminant,
        })
    }

    /// The [DWARF](crate::gimli::Dwarf) sections that this debuginfo entry belongs to.
    pub fn dwarf(&self) -> &'dwarf crate::gimli::Dwarf<R> {
        self.dwarf
    }

    /// The DWARF [unit][gimli::Unit] that this debuginfo entry belongs to.
    pub fn unit(&self) -> &crate::gimli::Unit<R, usize> {
        self.unit
    }

    pub fn name(&self) -> Result<Cow<str>, crate::gimli::Error> {
        self.name.to_string_lossy()
    }

    pub fn variants<F>(&self, mut f: F)
    where
        F: FnMut(super::variant::Variant<'dwarf, R>),
    {
        let mut tree = self.unit.entries_tree(Some(self.entry.offset())).unwrap();
        let root = tree.root().unwrap();
        match self.entry.tag() {
            crate::gimli::DW_TAG_enumeration_type => {
                let discriminant_type = crate::get_type(&self.entry).unwrap();
                let discriminant_type = self.unit.entry(discriminant_type).unwrap();
                let discriminant_type =
                    super::Type::from_die(self.dwarf, self.unit, discriminant_type).unwrap();

                let mut children = root.children();
                while let Some(child) = children.next().unwrap() {
                    let child = child.entry();
                    assert_eq!(child.tag(), crate::gimli::DW_TAG_enumerator);

                    let crate::gimli::AttributeValue::Udata(value) = child.attr_value(crate::gimli::DW_AT_const_value).unwrap().unwrap() else {
                        unimplemented!()
                    };

                    let _discriminant = Some(match discriminant_type {
                        super::Type::Atom(atom) => {

                        },
                        super::Type::U8 => super::DiscriminantValue::U8(value as _),
                        super::Type::U16 => super::DiscriminantValue::U16(value as _),
                        super::Type::U32 => super::DiscriminantValue::U32(value as _),
                        super::Type::U64 => super::DiscriminantValue::U64(value as _),
                        other => panic!("{:?}", other),
                    });

                    let discriminant_value: Option<super::DiscriminantValue> = child
                        .attr_value(crate::gimli::DW_AT_const_value)
                        .unwrap()
                        .map(|value| value.into());
                    f(super::variant::Variant::new(
                        self.dwarf,
                        self.unit,
                        child.clone(),
                        self.discriminant.clone(),
                        discriminant_value,
                    ));
                }
            }
            crate::gimli::DW_TAG_structure_type => {
                let mut variant_part = None;
                {
                    let mut children = root.children();
                    while let Some(child) = children.next().unwrap() {
                        if child.entry().tag() == crate::gimli::DW_TAG_variant_part {
                            variant_part = Some(child.entry().offset());
                            break;
                        }
                    }
                }

                let mut tree = self.unit.entries_tree(variant_part).unwrap();
                let root = tree.root().unwrap();
                let mut variants = root.children();

                while let Some(child) = variants.next().unwrap() {
                    let entry = child.entry();
                    if child.entry().tag() == crate::gimli::DW_TAG_variant {
                        let discriminant_value: Option<super::DiscriminantValue> = entry
                            .attr_value(crate::gimli::DW_AT_discr_value)
                            .unwrap()
                            .map(|value| value.into());

                        let mut variant_children = child.children();
                        let variant_entry = variant_children.next().unwrap().unwrap();
                        let variant_entry = variant_entry.entry();
                        let variant_ty = crate::get_type(variant_entry).unwrap();
                        let entry = self.unit.entry(variant_ty).unwrap();

                        f(super::variant::Variant::new(
                            self.dwarf,
                            self.unit,
                            entry,
                            self.discriminant.clone(),
                            discriminant_value,
                        ));
                    }
                }
            }
            _ => panic!(),
        }
    }

    pub fn size(&self) -> usize {
        self.entry
            .attr_value(crate::gimli::DW_AT_byte_size)
            .unwrap()
            .and_then(|r| r.udata_value())
            .unwrap()
            .try_into()
            .unwrap()
    }

    pub fn align(&self) -> usize {
        self.entry
            .attr_value(crate::gimli::DW_AT_alignment)
            .unwrap()
            .and_then(|r| r.udata_value())
            .unwrap()
            .try_into()
            .unwrap()
    }

    pub fn discriminant(&self) -> &super::Discriminant<R> {
        &self.discriminant
    }
}
