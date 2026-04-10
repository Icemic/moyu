use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::{Config, TS};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Patch<T> {
    #[default]
    Missing,
    Set(T),
    Reset,
}

impl<T> Patch<T> {
    /// Apply patch with direct assignment
    #[inline]
    pub fn apply_to(self, target: &mut T, default: T) {
        match self {
            Patch::Set(v) => *target = v,
            Patch::Reset => *target = default,
            Patch::Missing => {}
        }
    }

    /// Apply patch through a setter function
    #[inline]
    pub fn apply(self, on_set: impl FnOnce(T), default: T) {
        match self {
            Patch::Set(v) => on_set(v),
            Patch::Reset => on_set(default),
            Patch::Missing => {}
        }
    }
}

/// Macro for applying patches with various patterns
#[macro_export]
macro_rules! apply_patch {
    // Method call with closure: apply_patch!(patch => |v| { body }, default)
    // Usage: apply_patch!(props.x => |v| self.set_x(v), 0.0)
    ($patch:expr => |$v:ident| $body:expr, $default:expr) => {
        match $patch {
            $crate::utils::patch::Patch::Set($v) => $body,
            $crate::utils::patch::Patch::Reset => {
                let $v = $default;
                $body
            }
            $crate::utils::patch::Patch::Missing => {}
        }
    };

    // Direct assignment: apply_patch!(patch => target, default)
    ($patch:expr => $target:expr, $default:expr) => {
        match $patch {
            $crate::utils::patch::Patch::Set(v) => $target = v,
            $crate::utils::patch::Patch::Reset => $target = $default,
            $crate::utils::patch::Patch::Missing => {}
        }
    };
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<T>::deserialize(deserializer)?;
        match opt {
            Some(v) => Ok(Patch::Set(v)),
            None => Ok(Patch::Reset),
        }
    }
}

impl<T: Serialize> Serialize for Patch<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Patch::Missing => serializer.serialize_unit(),
            Patch::Set(v) => v.serialize(serializer),
            Patch::Reset => serializer.serialize_none(),
        }
    }
}

impl<T: TS + 'static> TS for Patch<T> {
    type WithoutGenerics = Self;
    type OptionInnerType = T;

    const IS_OPTION: bool = true;

    fn decl(cfg: &Config) -> String {
        <Option<T> as TS>::decl(cfg)
    }

    fn decl_concrete(cfg: &Config) -> String {
        <Option<T> as TS>::decl_concrete(cfg)
    }

    fn name(cfg: &Config) -> String {
        <Option<T> as TS>::name(cfg)
    }

    fn inline(cfg: &Config) -> String {
        <Option<T> as TS>::inline(cfg)
    }

    fn inline_flattened(cfg: &Config) -> String {
        <Option<T> as TS>::inline_flattened(cfg)
    }

    fn dependencies(cfg: &Config) -> Vec<ts_rs::Dependency> {
        <Option<T> as TS>::dependencies(cfg)
    }
}

impl<T> ts_rs::IsOption for Patch<T> {
    type Inner = T;
}
