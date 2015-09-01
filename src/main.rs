#![feature(io)]

use std::env;
use std::fs::File;
use std::io::{Read, BufReader};
use std::collections::VecDeque;
use std::iter::Iterator;

#[derive(Debug)]
enum AST {
    Move(i32),
    Inc(i32),
    Block(i32, Vec<AST>),
    Read,
    Write
}

#[derive(Debug)]
enum HIR {
    Move(i32),
    Inc(i32),
    Open(i32),
    Close(i32),
    Read,
    Write,
}

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

fn emit_asm(ir: &HIR) {
    match ir {
        &HIR::Move(i) => println!("        add  r5, {}", i),
        &HIR::Inc(i)  => {
            println!("        ldrb r1, [r5]");
            println!("        add  r1, {}", i);
            println!("        strb r1, [r5]");
        }
        &HIR::Open(i) => {
            println!("        ldrb r1, [r5]");
            println!("        cmp  r1, 0");
            println!("        beq  BF_End_{}", i);
            println!("BF_Start_{}:", i);
        }
        &HIR::Close(i) => {
            println!("        ldrb r1, [r5]");
            println!("        cmp  r1, 0");
            println!("        bne  BF_Start_{}", i);
            println!("BF_End_{}:", i);
        }
        &HIR::Write => {
            println!("        mov r7, 4");
            println!("        mov r0, 1");
            println!("        mov r1, r5");
            println!("        mov r2, 1");
            println!("        svc 0");
        }
        &HIR::Read => {
            println!("        mov r7, 3");
            println!("        mov r0, 0");
            println!("        mov r1, r5");
            println!("        mov r2, 1");
            println!("        svc 0");
        }
    }
}

fn postlude() {
    println!("        .data");
    println!("tape:   .space 30000");    
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

    prelude();
    for item in code.into_iter().map_window(2, constant_fold) {
        emit_asm(&item);
    }
    postlude();
    Ok(())
}

fn main() {
    for fname in env::args().skip(1) {
        if let Err(err) = compile(&fname) {
            println!("Error compiling '{file}': {error}", file=fname, error=err);
        }
    }
}
