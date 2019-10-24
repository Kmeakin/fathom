use std::io;
use std::io::prelude::*;

use crate::rust::{Alias, Const, Function, Item, Module, RtType, StructType, Term, Type};

pub fn emit_module(writer: &mut impl Write, module: &Module) -> io::Result<()> {
    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");

    writeln!(
        writer,
        "// This file is automatically @generated by {} {}",
        pkg_name, pkg_version,
    )?;
    writeln!(writer, "// It is not intended for manual editing.")?;

    if !module.doc.is_empty() {
        writeln!(writer)?;
        for doc_line in module.doc.iter() {
            writeln!(writer, "//!{}", doc_line)?;
        }
    }

    for item in &module.items {
        emit_item(writer, &item)?;
    }

    Ok(())
}

fn emit_item(writer: &mut impl Write, item: &Item) -> io::Result<()> {
    match item {
        Item::Const(const_) => emit_const(writer, const_),
        Item::Function(function) => emit_function(writer, function),
        Item::Alias(ty_alias) => emit_alias(writer, ty_alias),
        Item::Struct(struct_ty) => emit_struct_ty(writer, struct_ty),
    }
}

fn emit_const(writer: &mut impl Write, const_: &Const) -> io::Result<()> {
    writeln!(writer)?;

    for doc_line in const_.doc.iter() {
        writeln!(writer, "///{}", doc_line)?;
    }

    write!(writer, "pub const {}: ", const_.name)?;
    emit_ty(writer, &const_.ty)?;
    write!(writer, " = ")?;
    emit_term(writer, &const_.term)?;
    writeln!(writer, ";")?;

    Ok(())
}

fn emit_function(writer: &mut impl Write, function: &Function) -> io::Result<()> {
    writeln!(writer)?;

    for doc_line in function.doc.iter() {
        writeln!(writer, "///{}", doc_line)?;
    }

    write!(writer, "pub ")?;
    if function.is_const {
        write!(writer, "const ")?;
    }
    write!(writer, "fn {}() -> ", function.name)?;
    emit_ty(writer, &function.ty)?;
    writeln!(writer, " {{")?;
    write!(writer, "    ")?;
    emit_term(writer, &function.term)?;
    writeln!(writer)?;
    writeln!(writer, "}}")?;

    Ok(())
}

fn emit_alias(writer: &mut impl Write, ty_alias: &Alias) -> io::Result<()> {
    writeln!(writer)?;

    for doc_line in ty_alias.doc.iter() {
        writeln!(writer, "///{}", doc_line)?;
    }

    write!(writer, "pub type {} = ", ty_alias.name)?;
    emit_ty(writer, &ty_alias.ty)?;
    writeln!(writer, ";")?;

    Ok(())
}

fn emit_struct_ty(writer: &mut impl Write, struct_ty: &StructType) -> io::Result<()> {
    use itertools::Itertools;

    writeln!(writer)?;

    for doc_line in struct_ty.doc.iter() {
        writeln!(writer, "///{}", doc_line)?;
    }

    if !struct_ty.derives.is_empty() {
        writeln!(
            writer,
            "#[derive({})]",
            struct_ty.derives.iter().format(", "),
        )?;
    }
    if struct_ty.fields.is_empty() {
        writeln!(writer, "pub struct {} {{}}", struct_ty.name)?;
    } else {
        writeln!(writer, "pub struct {} {{", struct_ty.name)?;
        for field in &struct_ty.fields {
            for doc_line in field.doc.iter() {
                writeln!(writer, "    ///{}", doc_line)?;
            }

            write!(writer, "    pub {}: ", field.name)?;
            emit_ty(writer, &field.host_ty)?;
            write!(writer, ",")?;
            writeln!(writer)?;
        }
        writeln!(writer, "}}")?;
    }
    writeln!(writer)?;

    // Binary impl

    writeln!(writer, "impl ddl_rt::Binary for {} {{", struct_ty.name,)?;
    writeln!(writer, "    type Host = {};", struct_ty.name)?;
    writeln!(writer, "}}")?;
    writeln!(writer)?;

    // ReadBinary impl

    writeln!(
        writer,
        "impl<'data> ddl_rt::ReadBinary<'data> for {} {{",
        struct_ty.name,
    )?;
    if struct_ty.fields.is_empty() {
        writeln!(
            writer,
            "    fn read(_: &mut ddl_rt::ReadCtxt<'data>) -> Result<{}, ddl_rt::ReadError> {{",
            struct_ty.name,
        )?;
        writeln!(writer, "        Ok({} {{}})", struct_ty.name)?;
        writeln!(writer, "    }}")?;
    } else {
        writeln!(
            writer,
            "    fn read(ctxt: &mut ddl_rt::ReadCtxt<'data>) -> Result<{}, ddl_rt::ReadError> {{",
            struct_ty.name,
        )?;
        for field in &struct_ty.fields {
            write!(writer, "        let {} = ", field.name)?;
            emit_ty_read(writer, &field.format_ty)?;
            write!(writer, ";")?;
            writeln!(writer)?;
        }
        writeln!(writer)?;
        writeln!(writer, "        Ok({} {{", struct_ty.name)?;
        for field in &struct_ty.fields {
            writeln!(writer, "            {},", field.name)?;
        }
        writeln!(writer, "        }})")?;
        writeln!(writer, "    }}")?;
    }
    writeln!(writer, "}}")?;

    Ok(())
}

fn emit_ty(writer: &mut impl Write, ty: &Type) -> io::Result<()> {
    match ty {
        Type::Var(name) => write!(writer, "{}", name),
        Type::U8 => write!(writer, "u8"),
        Type::U16 => write!(writer, "u16"),
        Type::U32 => write!(writer, "u32"),
        Type::U64 => write!(writer, "u64"),
        Type::I8 => write!(writer, "i8"),
        Type::I16 => write!(writer, "i16"),
        Type::I32 => write!(writer, "i32"),
        Type::I64 => write!(writer, "i64"),
        Type::F32 => write!(writer, "f32"),
        Type::F64 => write!(writer, "f64"),
        Type::Bool => write!(writer, "bool"),
        Type::Rt(rt_ty) => {
            // TODO: Make this path configurable
            write!(writer, "ddl_rt::")?;
            emit_rt_ty(writer, rt_ty)
        }
        Type::If(_, lhs, rhs) => {
            // TODO: Make this path configurable
            write!(writer, "ddl_rt::If<")?;
            emit_ty(writer, lhs)?;
            write!(writer, ", ")?;
            emit_ty(writer, rhs)?;
            write!(writer, ">")
        }
    }
}

fn emit_ty_read(writer: &mut impl Write, ty: &Type) -> io::Result<()> {
    match ty {
        Type::Var(name) => write!(writer, "ctxt.read::<{}>()?", name),
        Type::Rt(rt_ty) => {
            // TODO: Make this path configurable
            write!(writer, "ctxt.read::<ddl_rt::")?;
            emit_rt_ty(writer, rt_ty)?;
            write!(writer, ">()?")
        }
        Type::If(cond, lhs, rhs) => {
            write!(writer, "if ")?;
            emit_term(writer, cond)?;
            write!(writer, " {{")?;
            emit_ty_read(writer, lhs)?;
            write!(writer, "}} else {{")?;
            emit_ty_read(writer, rhs)?;
            write!(writer, "}}")
        }
        _ => unimplemented!("unexpected host type"),
    }
}

fn emit_rt_ty(writer: &mut impl Write, ty: &RtType) -> io::Result<()> {
    match ty {
        RtType::U8 => write!(writer, "U8"),
        RtType::U16Le => write!(writer, "U16Le"),
        RtType::U16Be => write!(writer, "U16Be"),
        RtType::U32Le => write!(writer, "U32Le"),
        RtType::U32Be => write!(writer, "U32Be"),
        RtType::U64Le => write!(writer, "U64Le"),
        RtType::U64Be => write!(writer, "U64Be"),
        RtType::I8 => write!(writer, "I8"),
        RtType::I16Le => write!(writer, "I16Le"),
        RtType::I16Be => write!(writer, "I16Be"),
        RtType::I32Le => write!(writer, "I32Le"),
        RtType::I32Be => write!(writer, "I32Be"),
        RtType::I64Le => write!(writer, "I64Le"),
        RtType::I64Be => write!(writer, "I64Be"),
        RtType::F32Le => write!(writer, "F32Le"),
        RtType::F32Be => write!(writer, "F32Be"),
        RtType::F64Le => write!(writer, "F64Le"),
        RtType::F64Be => write!(writer, "F64Be"),
        RtType::InvalidDataDescription => write!(writer, "InvalidDataDescription"),
    }
}

fn emit_term(writer: &mut impl Write, term: &Term) -> io::Result<()> {
    match term {
        Term::Var(name) => write!(writer, "{}", name),
        Term::Bool(value) => write!(writer, "{}", value),
        Term::U8(value) => write!(writer, "{}u8", value),
        Term::U16(value) => write!(writer, "{}u16", value),
        Term::U32(value) => write!(writer, "{}u32", value),
        Term::U64(value) => write!(writer, "{}u64", value),
        Term::I8(value) => write!(writer, "{}i8", value),
        Term::I16(value) => write!(writer, "{}i16", value),
        Term::I32(value) => write!(writer, "{}i32", value),
        Term::I64(value) => write!(writer, "{}i64", value),
        Term::F32(value) => write!(writer, "{}f32", value),
        Term::F64(value) => write!(writer, "{}f64", value),
        Term::Call(term) => {
            emit_term(writer, term)?;
            write!(writer, "()")
        }
        Term::If(term0, term1, term2) => {
            write!(writer, "if ")?;
            emit_term(writer, term0)?;
            write!(writer, " {{ ")?;
            emit_term(writer, term1)?;
            write!(writer, " }} else {{ ")?;
            emit_term(writer, term2)?;
            write!(writer, " }}")
        }
    }
}
