//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/variant/variant.h>

use core::{fmt, mem::size_of};

use bytemuck::{Pod, Zeroable};

use crate::game_engine::godot::SizeInTargetProcess;

/// The type of a [`Variant`].
#[derive(Copy, Clone, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct VariantType(u8);

impl fmt::Debug for VariantType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            Self::NIL => "NIL",
            Self::BOOL => "BOOL",
            Self::INT => "INT",
            Self::FLOAT => "FLOAT",
            Self::STRING => "STRING",
            Self::VECTOR2 => "VECTOR2",
            Self::VECTOR2I => "VECTOR2I",
            Self::RECT2 => "RECT2",
            Self::RECT2I => "RECT2I",
            Self::VECTOR3 => "VECTOR3",
            Self::VECTOR3I => "VECTOR3I",
            Self::TRANSFORM2D => "TRANSFORM2D",
            Self::VECTOR4 => "VECTOR4",
            Self::VECTOR4I => "VECTOR4I",
            Self::PLANE => "PLANE",
            Self::QUATERNION => "QUATERNION",
            Self::AABB => "AABB",
            Self::BASIS => "BASIS",
            Self::TRANSFORM3D => "TRANSFORM3D",
            Self::PROJECTION => "PROJECTION",
            Self::COLOR => "COLOR",
            Self::STRING_NAME => "STRING_NAME",
            Self::NODE_PATH => "NODE_PATH",
            Self::RID => "RID",
            Self::OBJECT => "OBJECT",
            Self::CALLABLE => "CALLABLE",
            Self::SIGNAL => "SIGNAL",
            Self::DICTIONARY => "DICTIONARY",
            Self::ARRAY => "ARRAY",
            Self::PACKED_BYTE_ARRAY => "PACKED_BYTE_ARRAY",
            Self::PACKED_INT32_ARRAY => "PACKED_INT32_ARRAY",
            Self::PACKED_INT64_ARRAY => "PACKED_INT64_ARRAY",
            Self::PACKED_FLOAT32_ARRAY => "PACKED_FLOAT32_ARRAY",
            Self::PACKED_FLOAT64_ARRAY => "PACKED_FLOAT64_ARRAY",
            Self::PACKED_STRING_ARRAY => "PACKED_STRING_ARRAY",
            Self::PACKED_VECTOR2_ARRAY => "PACKED_VECTOR2_ARRAY",
            Self::PACKED_VECTOR3_ARRAY => "PACKED_VECTOR3_ARRAY",
            Self::PACKED_COLOR_ARRAY => "PACKED_COLOR_ARRAY",
            _ => "<Unknown>",
        })
    }
}

#[allow(missing_docs)]
impl VariantType {
    pub const NIL: Self = Self(0);

    // atomic types
    pub const BOOL: Self = Self(1);
    pub const INT: Self = Self(2);
    pub const FLOAT: Self = Self(3);
    pub const STRING: Self = Self(4);

    // math types
    pub const VECTOR2: Self = Self(5);
    pub const VECTOR2I: Self = Self(6);
    pub const RECT2: Self = Self(7);
    pub const RECT2I: Self = Self(8);
    pub const VECTOR3: Self = Self(9);
    pub const VECTOR3I: Self = Self(10);
    pub const TRANSFORM2D: Self = Self(11);
    pub const VECTOR4: Self = Self(12);
    pub const VECTOR4I: Self = Self(13);
    pub const PLANE: Self = Self(14);
    pub const QUATERNION: Self = Self(15);
    pub const AABB: Self = Self(16);
    pub const BASIS: Self = Self(17);
    pub const TRANSFORM3D: Self = Self(18);
    pub const PROJECTION: Self = Self(19);

    // misc types
    pub const COLOR: Self = Self(20);
    pub const STRING_NAME: Self = Self(21);
    pub const NODE_PATH: Self = Self(22);
    pub const RID: Self = Self(23);
    pub const OBJECT: Self = Self(24);
    pub const CALLABLE: Self = Self(25);
    pub const SIGNAL: Self = Self(26);
    pub const DICTIONARY: Self = Self(27);
    pub const ARRAY: Self = Self(28);

    // typed arrays
    pub const PACKED_BYTE_ARRAY: Self = Self(29);
    pub const PACKED_INT32_ARRAY: Self = Self(30);
    pub const PACKED_INT64_ARRAY: Self = Self(31);
    pub const PACKED_FLOAT32_ARRAY: Self = Self(32);
    pub const PACKED_FLOAT64_ARRAY: Self = Self(33);
    pub const PACKED_STRING_ARRAY: Self = Self(34);
    pub const PACKED_VECTOR2_ARRAY: Self = Self(35);
    pub const PACKED_VECTOR3_ARRAY: Self = Self(36);
    pub const PACKED_COLOR_ARRAY: Self = Self(37);
}

/// The most important data type in Godot.
///
/// [`Variant`](https://docs.godotengine.org/en/4.2/classes/class_variant.html)
#[derive(Copy, Clone, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct Variant {
    /// The type of the variant.
    pub ty: VariantType,
    _padding: [u8; 7],
    /// The data of the variant. Use one of the accessors to get the data, based
    /// on the type.
    pub data: [u8; 16],
}

impl Variant {
    /// Assume the variant is a boolean and returns its value. Make sure this is
    /// the correct type beforehand.
    pub fn get_bool(&self) -> bool {
        self.data[0] != 0
    }

    /// Assume the variant is an integer and returns its value. Make sure this
    /// is the correct type beforehand.
    pub fn get_int(&self) -> i32 {
        let [i, _, _, _]: &[i32; 4] = bytemuck::cast_ref(&self.data);
        *i
    }

    /// Assume the variant is a float and returns its value. Make sure this is
    /// the correct type beforehand.
    pub fn get_float(&self) -> f32 {
        let [f, _, _, _]: &[f32; 4] = bytemuck::cast_ref(&self.data);
        *f
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ty {
            VariantType::NIL => write!(f, "Variant::NIL"),
            VariantType::BOOL => write!(f, "Variant::BOOL({})", self.get_bool()),
            VariantType::INT => write!(f, "Variant::INT({})", self.get_int()),
            VariantType::FLOAT => write!(f, "Variant::FLOAT({})", self.get_float()),
            _ => f
                .debug_struct("Variant")
                .field("ty", &self.ty)
                .field("data", &self.data)
                .finish(),
        }
    }
}

impl SizeInTargetProcess for Variant {
    const SIZE: u64 = size_of::<Variant>() as u64;
}
