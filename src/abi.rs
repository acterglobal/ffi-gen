use crate::parser::{Interface, Type};
use std::collections::HashSet;

pub mod export;
pub mod import;

#[derive(Clone, Copy, Debug)]
pub enum NumType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    IPtr,
    UPtr,
}

#[derive(Clone, Debug)]
pub enum AbiType {
    Num(NumType),
    Usize,
    Isize,
    Bool,
    RefStr,
    String,
    RefSlice(NumType),
    Vec(NumType),
    RefObject(String),
    Object(String),
    Option(Box<AbiType>),
    Result(Box<AbiType>),
    RefIter(Box<AbiType>),
    Iter(Box<AbiType>),
    RefFuture(Box<AbiType>),
    Future(Box<AbiType>),
    RefStream(Box<AbiType>),
    Stream(Box<AbiType>),
    Tuple(Vec<AbiType>),
    Buffer(NumType),
    List(String),
    RefEnum(String),
}

impl AbiType {
    pub fn num(&self) -> NumType {
        match self {
            Self::Num(num) => *num,
            _ => todo!("{self:?} still missing"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum FunctionType {
    Constructor(String),
    Method(String),
    Function,
    NextIter(String, AbiType),
    PollFuture(String, AbiType),
    PollStream(String, AbiType),
}

#[derive(Clone, Debug)]
pub struct AbiFunction {
    pub doc: Vec<String>,
    pub ty: FunctionType,
    pub name: String,
    pub args: Vec<(String, AbiType)>,
    pub ret: Option<AbiType>,
}

impl AbiFunction {
    pub fn symbol(&self) -> String {
        match &self.ty {
            FunctionType::Constructor(object) | FunctionType::Method(object) => {
                format!("__{}_{}", object, &self.name)
            }
            FunctionType::Function => format!("__{}", &self.name),
            FunctionType::NextIter(symbol, _) => format!("{}_iter_{}", symbol, &self.name),
            FunctionType::PollFuture(symbol, _) => format!("{}_future_{}", symbol, &self.name),
            FunctionType::PollStream(symbol, _) => format!("{}_stream_{}", symbol, &self.name),
        }
    }

    pub fn ret(&self, rets: Vec<Var>) -> Return {
        match rets.len() {
            0 => Return::Void,
            1 => Return::Num(rets[0].clone()),
            _ => Return::Struct(rets, format!("{}Return", self.symbol())),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AbiObject {
    pub doc: Vec<String>,
    pub name: String,
    pub methods: Vec<AbiFunction>,
    #[allow(dead_code)]
    pub destructor: String,
}

#[derive(Clone, Debug)]
pub struct AbiIter {
    pub ty: AbiType,
    pub symbol: String,
}

impl AbiIter {
    pub fn next(&self) -> AbiFunction {
        AbiFunction {
            ty: FunctionType::NextIter(self.symbol.clone(), self.ty.clone()),
            doc: vec![],
            name: "next".to_string(),
            args: vec![],
            ret: Some(AbiType::Option(Box::new(self.ty.clone()))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AbiFuture {
    pub ty: AbiType,
    pub symbol: String,
}

impl AbiFuture {
    pub fn poll(&self) -> AbiFunction {
        AbiFunction {
            ty: FunctionType::PollFuture(self.symbol.clone(), self.ty.clone()),
            doc: vec![],
            name: "poll".to_string(),
            args: vec![
                ("post_cobject".to_string(), AbiType::Isize),
                ("port".to_string(), AbiType::Num(NumType::I64)),
            ],
            ret: Some(AbiType::Option(Box::new(self.ty.clone()))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AbiStream {
    pub ty: AbiType,
    pub symbol: String,
}

impl AbiStream {
    pub fn poll(&self) -> AbiFunction {
        AbiFunction {
            ty: FunctionType::PollStream(self.symbol.clone(), self.ty.clone()),
            doc: vec![],
            name: "poll".to_string(),
            args: vec![
                ("post_cobject".to_string(), AbiType::Isize),
                ("port".to_string(), AbiType::Num(NumType::I64)),
                ("done".to_string(), AbiType::Num(NumType::I64)),
            ],
            ret: Some(AbiType::Option(Box::new(self.ty.clone()))),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Return {
    Void,
    Num(Var),
    Struct(Vec<Var>, String),
}

#[derive(Default)]
struct VarGen {
    counter: u32,
}

impl VarGen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gen_num(&mut self, num: NumType) -> Var {
        self.gen(AbiType::Num(num))
    }

    pub fn gen(&mut self, ty: AbiType) -> Var {
        let binding = self.counter;
        self.counter += 1;
        Var { binding, ty }
    }
}

#[derive(Clone, Debug)]
pub struct Var {
    pub binding: u32,
    pub ty: AbiType,
}

/// Abi type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Abi {
    /// Native 32bit
    Native32,
    /// Native 64bit
    Native64,
    /// Wasm 32bit
    Wasm32,
    /// Wasm 64bit
    Wasm64,
}

impl Abi {
    pub(crate) fn native() -> Self {
        #[cfg(target_pointer_width = "32")]
        return Abi::Native32;
        #[cfg(target_pointer_width = "64")]
        return Abi::Native64;
    }

    /// Returns the size and alignment of a primitive type.
    pub(crate) fn layout(self, ty: NumType) -> (usize, usize) {
        let size = match ty {
            NumType::U8 | NumType::I8 => 1,
            NumType::U16 | NumType::I16 => 2,
            NumType::U32 | NumType::I32 | NumType::F32 => 4,
            NumType::U64 | NumType::I64 | NumType::F64 => 8,
            NumType::IPtr => todo!(),
            NumType::UPtr => todo!(),
        };
        let size = match self {
            Self::Native32 | Self::Native64 => size,
            Self::Wasm32 | Self::Wasm64 => core::cmp::max(4, size),
        };
        (size, size)
    }
}

impl Interface {
    pub fn objects(&self) -> Vec<AbiObject> {
        let mut objs = vec![];
        for object in &self.objects {
            let mut methods = vec![];
            for method in &object.methods {
                let obj = object.ident.clone();
                let func = AbiFunction {
                    doc: method.doc.clone(),
                    name: method.ident.clone(),
                    ty: if method.is_static {
                        FunctionType::Constructor(obj)
                    } else {
                        FunctionType::Method(obj)
                    },
                    args: method
                        .args
                        .iter()
                        .map(|(n, ty)| (n.clone(), self.to_type(ty)))
                        .collect(),
                    ret: method.ret.as_ref().map(|ty| self.to_type(ty)),
                };
                methods.push(func);
            }
            objs.push(AbiObject {
                doc: object.doc.clone(),
                name: object.ident.clone(),
                methods,
                destructor: format!("drop_box_{}", &object.ident),
            });
        }
        objs
    }

    pub fn functions(&self) -> Vec<AbiFunction> {
        let mut funcs = vec![];
        for func in &self.functions {
            assert!(!func.is_static);
            let args = func
                .args
                .iter()
                .map(|(n, ty)| (n.clone(), self.to_type(ty)))
                .collect();
            let ret = func.ret.as_ref().map(|ty| self.to_type(ty));
            let func = AbiFunction {
                doc: func.doc.clone(),
                name: func.ident.clone(),
                ty: FunctionType::Function,
                args,
                ret,
            };
            funcs.push(func);
        }
        funcs
    }

    pub fn iterators(&self) -> Vec<AbiIter> {
        let mut iterators = vec![];
        let mut functions = self.functions();
        for obj in self.objects() {
            functions.extend(obj.methods);
        }
        for func in functions {
            if let Some(ty) = func.ret.as_ref() {
                let mut p = ty;
                let mut symbol = func.symbol();
                loop {
                    match p {
                        AbiType::Option(ty) | AbiType::Result(ty) => p = &**ty,
                        AbiType::Future(ty) => {
                            symbol.push_str("_future_poll");
                            p = &**ty
                        }
                        AbiType::Stream(ty) => {
                            symbol.push_str("_stream_poll");
                            p = &**ty
                        }
                        AbiType::Iter(ty) => {
                            iterators.push(AbiIter {
                                ty: (**ty).clone(),
                                symbol,
                            });
                            break;
                        }
                        _ => break,
                    }
                }
            }
        }
        iterators
    }

    pub fn futures(&self) -> Vec<AbiFuture> {
        let mut futures = vec![];
        let mut functions = self.functions();
        for obj in self.objects() {
            functions.extend(obj.methods);
        }
        for func in functions {
            if let Some(ty) = func.ret.as_ref() {
                let mut p = ty;
                loop {
                    match p {
                        AbiType::Option(ty) | AbiType::Result(ty) => p = &**ty,
                        AbiType::Future(ty) => {
                            let symbol = func.symbol();
                            futures.push(AbiFuture {
                                ty: (**ty).clone(),
                                symbol,
                            });
                            break;
                        }
                        _ => break,
                    }
                }
            }
        }
        futures
    }

    pub fn streams(&self) -> Vec<AbiStream> {
        let mut streams = vec![];
        let mut functions = self.functions();
        for obj in self.objects() {
            functions.extend(obj.methods);
        }
        for func in functions {
            if let Some(ty) = func.ret.as_ref() {
                let mut p = ty;
                loop {
                    match p {
                        AbiType::Option(ty) | AbiType::Result(ty) => p = &**ty,
                        AbiType::Stream(ty) => {
                            let symbol = func.symbol();
                            streams.push(AbiStream {
                                ty: (**ty).clone(),
                                symbol,
                            });
                            break;
                        }
                        _ => break,
                    }
                }
            }
        }
        streams
    }

    pub fn listed_types(&self) -> Vec<String> {
        fn find_inner_listed_types<F: FnMut(String)>(ty: &AbiType, cb: &mut F) {
            use AbiType::*;
            match ty {
                List(name) => cb(name.clone()),
                Option(ty) | Result(ty) | Iter(ty) | Future(ty) | Stream(ty) | RefIter(ty)
                | RefFuture(ty) | RefStream(ty) => find_inner_listed_types(ty.as_ref(), cb),
                Tuple(tys) => tys.iter().for_each(|ty| find_inner_listed_types(ty, cb)),
                _ => {}
            }
        }

        let mut res = HashSet::new();
        let mut res_adder = |ty| {
            res.insert(ty);
        };
        let mut func_processor = |f: AbiFunction| {
            if let Some(ty) = &f.ret {
                find_inner_listed_types(ty, &mut res_adder);
            }
            for (_, ty) in f.args.iter() {
                find_inner_listed_types(ty, &mut res_adder);
            }
        };

        for func in self.functions() {
            func_processor(func);
        }
        for obj in self.objects() {
            for func in obj.methods {
                func_processor(func);
            }
        }

        let mut fin: Vec<String> = res.into_iter().collect();
        fin.sort();
        fin
    }

    pub fn imports(&self, abi: &Abi) -> Vec<import::Import> {
        let mut imports = vec![];
        for function in self.functions() {
            imports.push(abi.import(&function));
        }
        for obj in self.objects() {
            for method in &obj.methods {
                imports.push(abi.import(method));
            }
        }
        for iter in self.iterators() {
            imports.push(abi.import(&iter.next()));
        }
        for fut in self.futures() {
            imports.push(abi.import(&fut.poll()));
        }
        for stream in self.streams() {
            imports.push(abi.import(&stream.poll()));
        }
        imports
    }

    pub fn to_type(&self, ty: &Type) -> AbiType {
        match ty {
            Type::U8 => AbiType::Num(NumType::U8),
            Type::U16 => AbiType::Num(NumType::U16),
            Type::U32 => AbiType::Num(NumType::U32),
            Type::U64 => AbiType::Num(NumType::U64),
            Type::Usize => AbiType::Usize,
            Type::I8 => AbiType::Num(NumType::I8),
            Type::I16 => AbiType::Num(NumType::I16),
            Type::I32 => AbiType::Num(NumType::I32),
            Type::I64 => AbiType::Num(NumType::I64),
            Type::Isize => AbiType::Isize,
            Type::F32 => AbiType::Num(NumType::F32),
            Type::F64 => AbiType::Num(NumType::F64),
            Type::Bool => AbiType::Bool,
            Type::Buffer(inner) => match self.to_type(inner) {
                AbiType::Num(ty) => AbiType::Buffer(ty),
                ty => unimplemented!("Vec<{:?}>", ty),
            },
            Type::Ref(inner) => match &**inner {
                Type::String => AbiType::RefStr,
                Type::Slice(inner) => match self.to_type(inner) {
                    AbiType::Num(ty) => AbiType::RefSlice(ty),
                    ty => unimplemented!("&{:?}", ty),
                },
                Type::Ident(ident) => {
                    if self.is_object(ident) {
                        AbiType::RefObject(ident.clone())
                    } else if self.is_enum(ident) {
                        AbiType::RefEnum(ident.clone())
                    } else {
                        panic!("unknown identifier {}", ident)
                    }
                }
                ty => unimplemented!("&{:?}", ty),
            },
            Type::String => AbiType::String,
            Type::Slice(_) => panic!("slice needs to be passed by reference"),
            Type::Vec(inner) => match self.to_type(inner) {
                AbiType::Num(ty) => AbiType::Vec(ty),
                AbiType::Object(ty) => AbiType::List(ty),
                AbiType::RefEnum(ty) => AbiType::List(ty),
                AbiType::String => AbiType::List("FfiString".to_string()),
                ty => unimplemented!("Vec<{:?}>", ty),
            },
            Type::Ident(ident) => {
                if self.is_object(ident) {
                    AbiType::Object(ident.clone())
                } else if self.is_enum(ident) {
                    AbiType::RefEnum(ident.clone())
                } else {
                    panic!("unknown identifier {}", ident)
                }
            }
            Type::Option(ty) => {
                let inner = self.to_type(ty);
                if let AbiType::Option(_) = inner {
                    panic!("nested options are not supported");
                }
                AbiType::Option(Box::new(inner))
            }
            Type::Result(ty) => AbiType::Result(Box::new(self.to_type(ty))),
            Type::Iter(ty) => AbiType::Iter(Box::new(self.to_type(ty))),
            Type::Future(ty) => AbiType::Future(Box::new(self.to_type(ty))),
            Type::Stream(ty) => AbiType::Stream(Box::new(self.to_type(ty))),
            Type::Tuple(ty) => AbiType::Tuple(ty.iter().map(|ty| self.to_type(ty)).collect()),
        }
    }
}
