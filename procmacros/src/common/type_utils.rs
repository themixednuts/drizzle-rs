//! Shared helpers for inspecting `syn::Type` without stringification.

use syn::{GenericArgument, Path, PathArguments, Type};

fn last_path_ident(path: &Path) -> Option<&syn::Ident> {
    path.segments.last().map(|seg| &seg.ident)
}

fn path_contains_ident(path: &Path, ident: &str) -> bool {
    path.segments.iter().any(|seg| seg.ident == ident)
}

fn type_path(ty: &Type) -> Option<&Path> {
    if let Type::Path(type_path) = ty {
        Some(&type_path.path)
    } else {
        None
    }
}

pub(crate) fn is_option_type(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|ident| ident == "Option")
}

pub(crate) fn option_inner_type(ty: &Type) -> Option<&Type> {
    let path = type_path(ty)?;
    let segment = path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    args.args.iter().find_map(|arg| {
        if let GenericArgument::Type(inner) = arg {
            Some(inner)
        } else {
            None
        }
    })
}

pub(crate) fn unwrap_option(ty: &Type) -> &Type {
    option_inner_type(ty).unwrap_or(ty)
}

pub(crate) fn type_is_int(ty: &Type, ident: &str) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == ident)
}

pub(crate) fn type_is_bool(ty: &Type) -> bool {
    type_is_int(ty, "bool")
}

pub(crate) fn type_is_float(ty: &Type, ident: &str) -> bool {
    type_is_int(ty, ident)
}

pub(crate) fn type_is_string_like(ty: &Type) -> bool {
    if type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "String")
    {
        return true;
    }
    if let Type::Reference(reference) = ty
        && let Type::Path(path) = reference.elem.as_ref()
    {
        return path
            .path
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "str");
    }
    false
}

pub(crate) fn type_is_vec_u8(ty: &Type) -> bool {
    let Some(path) = type_path(ty) else {
        return false;
    };
    let Some(segment) = path.segments.last() else {
        return false;
    };
    if segment.ident != "Vec" {
        return false;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    args.args.iter().any(|arg| {
        if let GenericArgument::Type(Type::Path(inner)) = arg {
            inner
                .path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "u8")
        } else {
            false
        }
    })
}

pub(crate) fn type_is_byte_slice(ty: &Type) -> bool {
    match ty {
        Type::Reference(reference) => match reference.elem.as_ref() {
            Type::Slice(slice) => matches!(
                slice.elem.as_ref(),
                Type::Path(path) if path.path.segments.last().is_some_and(|seg| seg.ident == "u8")
            ),
            _ => false,
        },
        Type::Slice(slice) => matches!(
            slice.elem.as_ref(),
            Type::Path(path) if path.path.segments.last().is_some_and(|seg| seg.ident == "u8")
        ),
        _ => false,
    }
}

pub(crate) fn type_is_array_u8(ty: &Type) -> bool {
    match ty {
        Type::Array(array) => matches!(
            array.elem.as_ref(),
            Type::Path(path) if path.path.segments.last().is_some_and(|seg| seg.ident == "u8")
        ),
        _ => false,
    }
}

#[cfg(feature = "postgres")]
pub(crate) fn type_is_array_char(ty: &Type) -> bool {
    match ty {
        Type::Array(array) => matches!(
            array.elem.as_ref(),
            Type::Path(path) if path.path.segments.last().is_some_and(|seg| seg.ident == "char")
        ),
        _ => false,
    }
}

#[cfg(feature = "postgres")]
#[allow(dead_code)]
pub(crate) fn type_is_char_array(ty: &Type) -> bool {
    type_is_array_char(ty)
}

pub(crate) fn type_is_array_string(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "ArrayString")
}

pub(crate) fn type_is_arrayvec_u8(ty: &Type) -> bool {
    let Some(path) = type_path(ty) else {
        return false;
    };
    let Some(segment) = path.segments.last() else {
        return false;
    };
    if segment.ident != "ArrayVec" {
        return false;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    args.args.iter().any(|arg| {
        if let GenericArgument::Type(Type::Path(inner)) = arg {
            inner
                .path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "u8")
        } else {
            false
        }
    })
}

pub(crate) fn type_is_uuid(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "Uuid")
}

pub(crate) fn type_is_json_value(ty: &Type) -> bool {
    let Some(path) = type_path(ty) else {
        return false;
    };
    let Some(last) = last_path_ident(path) else {
        return false;
    };
    if last != "Value" {
        return false;
    }
    // Only match serde_json::Value (qualified path), not bare `Value` which
    // could be any user-defined type
    path_contains_ident(path, "serde_json")
}

pub(crate) fn type_is_naive_date(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "NaiveDate")
}

pub(crate) fn type_is_naive_time(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "NaiveTime")
}

pub(crate) fn type_is_naive_datetime(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "NaiveDateTime")
}

pub(crate) fn type_is_datetime_tz(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "DateTime")
}

pub(crate) fn type_is_chrono_date(ty: &Type) -> bool {
    type_is_naive_date(ty)
}

pub(crate) fn type_is_chrono_time(ty: &Type) -> bool {
    type_is_naive_time(ty)
}

#[allow(dead_code)]
pub(crate) fn type_is_chrono_datetime(ty: &Type) -> bool {
    type_is_naive_datetime(ty) || type_is_datetime_tz(ty)
}

pub(crate) fn type_is_time_date(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "Date")
}

pub(crate) fn type_is_time_time(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "Time")
}

pub(crate) fn type_is_primitive_date_time(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "PrimitiveDateTime")
}

pub(crate) fn type_is_offset_datetime(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "OffsetDateTime")
}

#[cfg(feature = "postgres")]
pub(crate) fn type_is_ip_addr(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "IpAddr" || id == "IpInet")
}

#[cfg(feature = "postgres")]
pub(crate) fn type_is_ip_cidr(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "IpCidr")
}

#[cfg(feature = "postgres")]
pub(crate) fn type_is_mac_addr(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "MacAddress")
}

#[cfg(feature = "postgres")]
pub(crate) fn type_is_geo_point(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "Point")
}

#[cfg(feature = "postgres")]
pub(crate) fn type_is_geo_rect(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "Rect")
}

#[cfg(feature = "postgres")]
pub(crate) fn type_is_geo_linestring(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "LineString")
}

#[cfg(feature = "postgres")]
pub(crate) fn type_is_bit_vec(ty: &Type) -> bool {
    type_path(ty)
        .and_then(last_path_ident)
        .is_some_and(|id| id == "BitVec")
}
