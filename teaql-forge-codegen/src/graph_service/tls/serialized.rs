//! Read the SealedObject stream that JCEKS stores for a secret-key entry.
//! Secret keys provide no trust anchor; preserve the stream boundary and aliases.
use super::Reader;

#[derive(Clone)]
struct Class {
    name: String,
    fields: Vec<(u8, String)>,
    parent: Option<Box<Class>>,
}

struct Stream<'r, 'b> {
    reader: &'r mut Reader<'b>,
    handles: Vec<Option<Class>>,
    last_bytes: Option<Vec<u8>>,
    captured: std::collections::BTreeMap<String, Vec<u8>>,
}

impl Stream<'_, '_> {
    fn byte(&mut self) -> Result<u8, ()> {
        Ok(self.reader.bytes(1)?[0])
    }
    fn reference(&mut self) -> Result<Option<Class>, ()> {
        let handle = self.reader.int()?.checked_sub(0x7e0000).ok_or(())?;
        self.handles.get(handle).cloned().ok_or(())
    }
    fn class(&mut self) -> Result<Option<Class>, ()> {
        match self.byte()? {
            0x70 => Ok(None),
            0x71 => self.reference(),
            0x72 => {
                let name = String::from_utf16(&self.reader.utf()?).map_err(|_| ())?;
                if !matches!(
                    name.as_str(),
                    "com.sun.crypto.provider.SealedObjectForKeyProtector"
                        | "javax.crypto.SealedObject"
                        | "[B"
                ) {
                    return Err(());
                }
                self.reader.bytes(8)?;
                let handle = self.handles.len();
                self.handles.push(None);
                if self.byte()? != 2 {
                    return Err(());
                }
                let count = u16::from_be_bytes(self.reader.bytes(2)?.try_into().unwrap());
                let mut fields = Vec::new();
                for _ in 0..count {
                    let kind = self.byte()?;
                    let name = String::from_utf16(&self.reader.utf()?).map_err(|_| ())?;
                    fields.push((kind, name));
                    if matches!(kind, b'[' | b'L') {
                        self.value()?;
                    }
                }
                if self.byte()? != 0x78 {
                    return Err(());
                }
                let parent = self.class()?.map(Box::new);
                let class = Class {
                    name,
                    fields,
                    parent,
                };
                self.handles[handle] = Some(class.clone());
                Ok(Some(class))
            }
            _ => Err(()),
        }
    }
    fn fields(&mut self, class: &Class) -> Result<(), ()> {
        if let Some(parent) = &class.parent {
            self.fields(parent)?;
        }
        for (field, name) in &class.fields {
            match field {
                b'[' | b'L' => {
                    self.value()?;
                    if class.name == "javax.crypto.SealedObject" {
                        if let Some(bytes) = self.last_bytes.take() {
                            self.captured.insert(name.clone(), bytes);
                        }
                    }
                }
                b'B' | b'Z' => {
                    self.reader.bytes(1)?;
                }
                b'C' | b'S' => {
                    self.reader.bytes(2)?;
                }
                b'I' | b'F' => {
                    self.reader.bytes(4)?;
                }
                b'J' | b'D' => {
                    self.reader.bytes(8)?;
                }
                _ => return Err(()),
            }
        }
        Ok(())
    }
    fn value(&mut self) -> Result<Option<Class>, ()> {
        self.last_bytes = None;
        match self.byte()? {
            0x70 => Ok(None),
            0x71 => self.reference(),
            0x74 => {
                self.reader.utf()?;
                self.handles.push(None);
                Ok(None)
            }
            0x73 => {
                let class = self.class()?.ok_or(())?;
                self.handles.push(None);
                self.fields(&class)?;
                Ok(Some(class))
            }
            0x75 => {
                let class = self.class()?.ok_or(())?;
                if class.name != "[B" {
                    return Err(());
                }
                self.handles.push(None);
                let length = self.reader.int()?;
                self.last_bytes = Some(self.reader.bytes(length)?.to_vec());
                Ok(None)
            }
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub(super) struct Sealed {
    pub parameters: Vec<u8>,
    pub encrypted: Vec<u8>,
}

pub(super) fn read(reader: &mut Reader<'_>) -> Result<Sealed, ()> {
    if reader.bytes(4)? != [0xac, 0xed, 0, 5] {
        return Err(());
    }
    let mut stream = Stream {
        reader,
        handles: Vec::new(),
        last_bytes: None,
        captured: Default::default(),
    };
    let class = stream.value()?.ok_or(())?;
    if class.name != "com.sun.crypto.provider.SealedObjectForKeyProtector" {
        return Err(());
    }
    Ok(Sealed {
        parameters: stream.captured.remove("encodedParams").unwrap_or_default(),
        encrypted: stream
            .captured
            .remove("encryptedContent")
            .unwrap_or_default(),
    })
}

pub(super) fn skip(reader: &mut Reader<'_>) -> Result<(), ()> {
    read(reader).map(|_| ())
}
