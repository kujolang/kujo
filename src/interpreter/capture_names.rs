//! Owned proof names with an allocation-free short representation.
//! Length prefixes preserve arbitrary UTF-8 names without sentinel restrictions.
#[derive(Clone, Debug)]
pub(super) enum CaptureNames {
    Inline { bytes: [u8; 22], used: u8 },
    Heap(Box<[String]>),
}
impl CaptureNames {
    pub(super) fn from_names<'a>(names: impl Iterator<Item = &'a str> + Clone) -> Self {
        let size = names.clone().try_fold(0usize, |used, name| {
            used.checked_add(name.len())?.checked_add(1).filter(|n| *n <= 22)
        });
        if let Some(size) = size {
            let mut bytes = [0; 22];
            let mut offset = 0;
            for name in names {
                bytes[offset] = name.len() as u8;
                offset += 1;
                bytes[offset..offset + name.len()].copy_from_slice(name.as_bytes());
                offset += name.len();
            }
            Self::Inline { bytes, used: size as u8 }
        } else {
            Self::Heap(names.map(str::to_owned).collect())
        }
    }
    pub(super) fn iter(&self) -> Names<'_> {
        match self {
            Self::Inline { bytes, used } => Names::Inline(&bytes[..usize::from(*used)]),
            Self::Heap(names) => Names::Heap(names.iter()),
        }
    }
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Heap(names) => names.len(),
            _ => self.iter().count(),
        }
    }
}
#[derive(Clone)]
pub(super) enum Names<'a> {
    Inline(&'a [u8]),
    Heap(std::slice::Iter<'a, String>),
}
impl<'a> Iterator for Names<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Inline(bytes) => {
                let (&length, tail) = bytes.split_first()?;
                let (name, rest) = tail.split_at(usize::from(length));
                *bytes = rest;
                Some(std::str::from_utf8(name).expect("proof names originate from UTF-8 strings"))
            }
            Self::Heap(names) => names.next().map(String::as_str),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_proof_names_preserve_order_empty_unicode_and_nul() {
        for input in [vec![], vec!["n", "state"], vec!["", "é", "a\0b", "", "z"]] {
            let names = CaptureNames::from_names(input.iter().copied());
            assert!(matches!(names, CaptureNames::Inline { .. }));
            assert_eq!(names.len(), input.len());
            assert_eq!(names.iter().collect::<Vec<_>>(), input);
            let saved = names.clone();
            drop(names);
            assert_eq!(saved.iter().collect::<Vec<_>>(), input);
        }
    }
    #[test]
    fn proof_names_boundary_and_heap_preserve_exact_names() {
        for length in [20, 21, 22, 256, 4096] {
            let name = "x".repeat(length);
            let names = CaptureNames::from_names(std::iter::once(name.as_str()));
            assert_eq!(matches!(names, CaptureNames::Inline { .. }), length <= 21);
            assert_eq!(names.iter().collect::<Vec<_>>(), vec![name.as_str()]);
        }
        let input: Vec<_> = (0..40).map(|i| format!("name_{i}")).collect();
        let names = CaptureNames::from_names(input.iter().map(String::as_str));
        assert_eq!(names.len(), 40);
        assert_eq!(
            names.iter().collect::<Vec<_>>(),
            input.iter().map(String::as_str).collect::<Vec<_>>()
        );
        assert_eq!(names.clone().iter().collect::<Vec<_>>(), names.iter().collect::<Vec<_>>());
    }
    #[test]
    fn proof_names_preserve_previous_vector_layout() {
        assert_eq!(std::mem::size_of::<CaptureNames>(), std::mem::size_of::<Vec<String>>());
        assert_eq!(std::mem::align_of::<CaptureNames>(), std::mem::align_of::<Vec<String>>());
    }
}
