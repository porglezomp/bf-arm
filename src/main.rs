#![feature(io)]
#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(slice_patterns)]

use std::env;
use std::fs::File;
use std::io::{Read, BufReader};
use std::collections::VecDeque;
use std::iter::Iterator;

#[derive(Debug, PartialEq, Eq)]
enum AST {
    Move(i32),
    Inc(i32),
    Block(i32, Vec<AST>),
    Read,
    Write
}

#[derive(Debug, PartialEq, Eq)]
enum HIR {
    Move(i32),
    Inc(i32),
    Open(i32),
    Close(i32),
    Read,
    Write,
}

#[derive(Debug, PartialEq, Eq)]
enum MIR {
    Store(Location, Location),
    Load(Location, Location),
    Move(i32, Location),
    Inc(i32, Location),
    BranchZero(i32, i32),
    BranchNonZero(i32, i32),
    Label(Pair),
    Svc,
}

#[derive(Debug, PartialEq, Eq)]
enum Pair {
    Start(i32),
    End(i32),
}

#[derive(Debug, PartialEq, Eq)]
enum Location {
    Reg(i32),
    Imm(i32),
    Addr(Box<Location>),
}
use ::Location::*;

struct WindowMap<I, F> where
    I: Iterator,
{
    window_size: usize,
    window: VecDeque<I::Item>,
    iter: I,
    f: F,
}

impl<I, F> Iterator for WindowMap<I, F> where
    I: Iterator, F: FnMut(&mut VecDeque<I::Item>),
{
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        while self.window.len() < self.window_size {
            if let Some(i) = self.iter.next() {
                self.window.push_back(i);
            } else {
                break;
            }
            (self.f)(&mut self.window);
        }

        let item = self.window.pop_front();
        if let Some(i) = self.iter.next() {
            self.window.push_back(i);
        }
        (self.f)(&mut self.window);
        item
    }
}

trait WindowMapper<I: Iterator>: Iterator {
    fn map_window<F>(self, window_size: usize, f: F) -> WindowMap<Self, F> where
        Self: Sized, F: FnMut(&mut VecDeque<I::Item>)
    {
        WindowMap {
            window_size: window_size,
            window: VecDeque::new(),
            iter: self,
            f: f
        }
    }
}

impl<T> WindowMapper<T> for T where T: Iterator { }

fn ast_to_ir(ast: &Vec<AST>) -> Vec<HIR> {
    let mut ir = vec![];
    for node in ast.iter() {
        match node {
            &AST::Block(id, ref v) => {
                ir.push(HIR::Open(id));
                ir.extend(ast_to_ir(&v));
                ir.push(HIR::Close(id));
            }
            &AST::Move(i) => ir.push(HIR::Move(i)),
            &AST::Inc(i) => ir.push(HIR::Inc(i)),
            &AST::Read => ir.push(HIR::Read),
            &AST::Write => ir.push(HIR::Write),
        }
    }
    ir
}

fn is_bf_char(x: &char) -> bool {
    let &x = x;
    x == '<' || x == '>' || x == '+' || x == '-' ||
    x == '[' || x == ']' || x == ',' || x == '.'
}

fn read(fname: &String) -> std::io::Result<Box<Iterator<Item=char>>> {
    let f = try!(File::open(fname));
    let chars = BufReader::new(f).chars();
    Ok(Box::new(chars.map(|x| x.ok())
                .filter_map(|x| x)
                .filter(is_bf_char)))
}

fn prelude() {
    println!("        .text");
    println!("        .global main");
    println!("        .syntax unified");
    println!("main:   ldr r5, =tape");
}

fn postlude() {
    println!("        .data");
    println!("tape:   .space 30000");    
}

fn map_collect<I, F, T>(items: I, f: F) -> Vec<T> where
    I: Iterator, F: Fn(I::Item) -> Vec<T>
{
    let mut collect = vec![];
    for item in items {
        let x = f(item);
        collect.extend(x);
    }
    collect
}

fn compile(fname: &String) -> std::io::Result<()> {
    let mut text = try!(read(fname));

    fn parse_block<I>(text: &mut I, id: &mut i32) -> Vec<AST>
        where I: Iterator<Item=char>
    {
        let mut items = vec![];
        while let Some(letter) = text.next() {
            let item = match letter {
                '<' => AST::Move(-1),
                '>' => AST::Move(1),
                '-' => AST::Inc(-1),
                '+' => AST::Inc(1),
                '[' => {
                    let last_id = *id;
                    *id += 1;
                    let code = parse_block(text, id);
                    AST::Block(last_id, code)
                }
                ']' => return items,
                ',' => AST::Read,
                '.' => AST::Write,
                _   => panic!("NO")
            };
            items.push(item);
        }
        items
    }

    let mut id = 0;
    let code = parse_block(&mut text, &mut id);
    let code = ast_to_ir(&code);

    fn constant_fold(window: &mut VecDeque<HIR>) {
        if window.len() < 2 { return; }

        match (&window[0], &window[1]) {
            (&HIR::Inc(a), &HIR::Inc(b)) => {
                window[1] = HIR::Inc(a + b);
                window.pop_front();
            }
            (&HIR::Move(a), &HIR::Move(b)) => {
                window[1] = HIR::Move(a + b);
                window.pop_front();
            }
            _ => ()
        }
    }

    fn hir_to_mir(x: HIR) -> Vec<MIR> {
        match x {
            HIR::Move(i) => vec![MIR::Inc(5, Imm(i))],
            HIR::Inc(i)  => vec![MIR::Load(Reg(1), Addr(box Reg(5))),
                                 MIR::Inc(1, Imm(i)),
                                 MIR::Store(Reg(1), Addr(box Reg(5)))],
            HIR::Open(i) => vec![MIR::Load(Reg(1), Addr(box Reg(5))),
                                 MIR::BranchZero(1, i),
                                 MIR::Label(Pair::Start(i))],
            HIR::Close(i) => vec![MIR::Load(Reg(1), Addr(box Reg(5))),
                                  MIR::BranchNonZero(1, i),
                                  MIR::Label(Pair::End(i))],
            HIR::Write => vec![MIR::Move(7, Imm(4)),
                               MIR::Move(0, Imm(1)),
                               MIR::Move(1, Reg(5)),
                               MIR::Move(2, Imm(1)),
                               MIR::Svc],
            HIR::Read => vec![MIR::Move(7, Imm(3)),
                              MIR::Move(0, Imm(0)),
                              MIR::Move(1, Reg(5)),
                              MIR::Move(2, Imm(1)),
                              MIR::Svc],
        }
    }

    fn mir_to_asm(x: MIR) -> Vec<String> {
        match x {
            MIR::Store(Reg(src), Addr(box Reg(dst))) =>
                vec![format!("        strb r{}, [r{}]", src, dst)],
            MIR::Load(Reg(dst), Addr(box Reg(src))) =>
                vec![format!("        ldrb r{}, [r{}]", dst, src)],
            MIR::Move(dst, Imm(src)) =>
                vec![format!("        mov  r{}, {}", dst, src)],
            MIR::Move(dst, Reg(src)) =>
                vec![format!("        mov  r{}, r{}", dst, src)],
            MIR::Inc(dst, Imm(src)) =>
                vec![format!("        add  r{}, {}", dst, src)],
            MIR::BranchZero(reg, label) =>
                vec![format!("        cmp  r{}, 0", reg),
                     format!("        beq  BF_End_{}", label)],
            MIR::BranchNonZero(reg, label) =>
                vec![format!("        cmp  r{}, 0", reg),
                     format!("        bne  BF_Start_{}", label)],
            MIR::Label(Pair::Start(i)) => vec![format!("BF_Start_{}:", i)],
            MIR::Label(Pair::End(i)) => vec![format!("BF_End_{}:", i)],
            MIR::Svc => vec!["        svc 0".into()],
            x => panic!("ICE: Unsupported MIR `{:?}`", x),
        }
    }

    fn remove_redundant_load(window: &mut VecDeque<MIR>) {
        if window.len() < 2 { return; }

        let fst = window.pop_front().unwrap();
        let snd = window.pop_front().unwrap();
        match (fst, snd) {
            (MIR::Store(Reg(ra), addr_a), MIR::Load(Reg(rb), addr_b)) => {
                if ra != rb || addr_a != addr_b {
                    window.push_front(MIR::Load(Reg(rb), addr_b));
                }
                window.push_front(MIR::Store(Reg(ra), addr_a));
            }
            (fst, snd) => {
                window.push_front(snd);
                window.push_front(fst);
            }
        }
    }

    prelude();
    let code = code.into_iter().map_window(2, constant_fold);
    let code = map_collect(code, hir_to_mir);
    let code = code.into_iter().map_window(2, remove_redundant_load);
    let code = map_collect(code, mir_to_asm);
    for item in code {
        println!("{}", item);
//        emit_asm(&item);
    }
    postlude();
    Ok(())
}

fn main() {
    for fname in env::args().skip(1) {
        if let Err(err) = compile(&fname) {
            println!("Error compiling '{}': {}", fname, err);
        }
    }
}
