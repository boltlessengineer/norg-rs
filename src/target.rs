use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum NorgLinkTarget {
    Local(NorgLinkLocalTarget),
    App(NorgLinkAppTarget),
}

#[derive(Debug, PartialEq)]
pub enum NorgLinkLocalTarget {
    Raw(String),
    Scope(Vec<NorgLinkScope>),
}

#[derive(Debug, PartialEq)]
pub struct NorgLinkAppTarget {
    pub workspace: Option<String>,
    pub path: PathBuf,
    pub scopes: Vec<NorgLinkScope>,
}

pub type NorgMarkup = String;

#[derive(Debug, PartialEq)]
pub enum NorgLinkScope {
    Heading(u16, NorgMarkup),
    WikiHeading(NorgMarkup),
}

impl Into<janetrs::Janet> for NorgLinkTarget {
    fn into(self) -> janetrs::Janet {
        janetrs::Janet::tuple(self.into())
    }
}

impl Into<janetrs::JanetTuple<'_>> for NorgLinkTarget {
    fn into(self) -> janetrs::JanetTuple<'static> {
        match self {
            Self::Local(local) => janetrs::tuple![
                janetrs::JanetKeyword::new("local"),
                janetrs::Janet::tuple(local.into()),
            ],
            Self::App(app) => janetrs::tuple![
                janetrs::JanetKeyword::new("app"),
                janetrs::Janet::structs(app.into()),
            ],
        }
    }
}

impl TryFrom<janetrs::Janet> for NorgLinkTarget {
    type Error = janetrs::JanetConversionError;

    fn try_from(value: janetrs::Janet) -> Result<Self, Self::Error> {
        Self::try_from(value.try_unwrap::<janetrs::JanetTuple>()?)
    }
}

impl TryFrom<janetrs::JanetTuple<'_>> for NorgLinkTarget {
    type Error = janetrs::JanetConversionError;

    fn try_from(value: janetrs::JanetTuple) -> Result<Self, Self::Error> {
        let [kind, value] = value.as_ref() else {
            todo!("error");
        };
        let kind = kind.try_unwrap::<janetrs::JanetKeyword>()?;
        match kind.as_bytes() {
            b"local" => Ok(Self::Local(NorgLinkLocalTarget::try_from(*value)?)),
            b"app" => Ok(Self::App(NorgLinkAppTarget::try_from(*value)?)),
            _ => todo!("error"),
        }
    }
}

impl Into<janetrs::JanetTuple<'_>> for NorgLinkLocalTarget {
    fn into(self) -> janetrs::JanetTuple<'static> {
        match self {
            Self::Raw(raw) => janetrs::tuple![
                janetrs::JanetKeyword::new("raw"),
                janetrs::Janet::string(raw.into()),
            ],
            Self::Scope(_scopes) => janetrs::tuple![
                janetrs::JanetKeyword::new("scopes"),
                janetrs::tuple![],
            ],
        }
    }
}

impl TryFrom<janetrs::Janet> for NorgLinkLocalTarget {
    type Error = janetrs::JanetConversionError;

    fn try_from(value: janetrs::Janet) -> Result<Self, Self::Error> {
        Self::try_from(value.try_unwrap::<janetrs::JanetTuple>()?)
    }
}

impl TryFrom<janetrs::JanetTuple<'_>> for NorgLinkLocalTarget {
    type Error = janetrs::JanetConversionError;

    fn try_from(value: janetrs::JanetTuple<'_>) -> Result<Self, Self::Error> {
        let [local_kind, value] = value.as_ref() else {
            todo!("error");
        };
        let kind = local_kind.try_unwrap::<janetrs::JanetKeyword>()?;
        match kind.as_bytes() {
            b"raw" => Ok(Self::Raw(
                value
                    .try_unwrap::<janetrs::JanetString>()?
                    .to_str_lossy()
                    .to_string(),
            )),
            b"scopes" => Ok(Self::Scope(
                value
                    .try_unwrap::<janetrs::JanetTuple>()
                    .unwrap()
                    .into_iter()
                    .map(NorgLinkScope::try_from)
                    .collect::<Result<_, _>>()?,
            )),
            _ => todo!("error"),
        }
    }
}

impl From<PathBuf> for NorgLinkAppTarget {
    fn from(path: PathBuf) -> Self {
        Self {
            workspace: None,
            path,
            scopes: vec![],
        }
    }
}

impl Into<janetrs::JanetStruct<'_>> for NorgLinkAppTarget {
    fn into(self) -> janetrs::JanetStruct<'static> {
        janetrs::JanetStruct::builder(3)
            .put(janetrs::JanetKeyword::new("workspace"), match self.workspace {
                Some(name) => janetrs::JanetString::from(name).into(),
                None => janetrs::Janet::nil(),
            })
            .put(
                janetrs::JanetKeyword::new("path"),
                janetrs::JanetString::from(self.path.to_str().unwrap()),
            )
            .put(
                janetrs::JanetKeyword::new("scopes"),
                // TODO: pass scopes
                janetrs::tuple![],
            )
            .finalize()
    }
}

impl TryFrom<janetrs::Janet> for NorgLinkAppTarget {
    type Error = janetrs::JanetConversionError;

    fn try_from(value: janetrs::Janet) -> Result<Self, Self::Error> {
        Self::try_from(value.try_unwrap::<janetrs::JanetStruct>()?)
    }
}

impl TryFrom<janetrs::JanetStruct<'_>> for NorgLinkAppTarget {
    type Error = janetrs::JanetConversionError;

    fn try_from(value: janetrs::JanetStruct<'_>) -> Result<Self, Self::Error> {
        let workspace = value
            .get(janetrs::JanetKeyword::new("workspace"))
            .map(|workspace| {
                workspace
                    .try_unwrap::<janetrs::JanetString>()
                    .unwrap()
                    .to_str_lossy()
                    .to_string()
            });
        let path = value
            .get(janetrs::JanetKeyword::new("path"))
            .unwrap()
            .try_unwrap::<janetrs::JanetString>()?
            .to_path_lossy()
            .to_path_buf();
        let scopes = value
            .get(janetrs::JanetKeyword::new("scopes"))
            .unwrap()
            .try_unwrap::<janetrs::JanetTuple>()?
            .into_iter()
            .map(NorgLinkScope::try_from)
            .collect::<Result<_, _>>()?;
        Ok(Self {
            workspace,
            path,
            scopes,
        })
    }
}

impl TryFrom<janetrs::Janet> for NorgLinkScope {
    type Error = janetrs::JanetConversionError;

    fn try_from(value: janetrs::Janet) -> Result<Self, Self::Error> {
        Self::try_from(value.try_unwrap::<janetrs::JanetTuple>()?)
    }
}

impl TryFrom<janetrs::JanetTuple<'_>> for NorgLinkScope {
    type Error = janetrs::JanetConversionError;

    fn try_from(value: janetrs::JanetTuple<'_>) -> Result<Self, Self::Error> {
        let [kind, ..] = value.as_ref() else {
            todo!("error");
        };
        let values = &value.as_ref()[1..];
        let kind = kind.try_unwrap::<janetrs::JanetKeyword>()?;
        match kind.as_bytes() {
            b"heading" => Ok({
                let level = values[0].try_unwrap::<f64>()? as u16;
                let title = values[1].to_string();
                Self::Heading(level, title)
            }),
            b"wiki" => todo!(),
            _ => todo!("error"),
        }
    }
}
