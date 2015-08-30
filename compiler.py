from __future__ import print_function
import sys
import os

def parse(stream, is_list=False):
    stream = iter(stream)
    elements = []
    while True:
        try:
            next_item = next(stream)
        except StopIteration:
            if is_list:
                raise Exception("Unmatched [")
            break

        if next_item == '[':
            elements.append(parse(stream, is_list=True))
        elif next_item == ']':
            if is_list:
                return elements
            else:
                raise Exception("Unmatched ]")
        else:
            if next_item in '+-><.,':
                elements.append(next_item)

    return elements

output = []
def start_label(l): return 'BF_Start_' + str(l)
def end_label(l): return 'BF_End_' + str(l)
def instr(i): output.append(i)
def label(l): output.append(l + ':')

label_id = 0
datareg = 'r5'
def codegen(node, lbl=None):
    global label_id
    if isinstance(node, list):
        lbl = label_id
        label_id += 1
        codegen('[', lbl=lbl)
        list_codegen(node, toplevel=False)
        codegen(']', lbl=lbl)
    else:
        if node == '>':
            instr('add  {}, 1'.format(datareg))
        elif node == '<':
            instr('sub  {}, 1'.format(datareg))
        elif node in '+-':
            instr('ldrb r1, [{}]'.format(datareg))
            if node == '+':
                instr('add  r1, 1')
            else:
                instr('sub  r1, 1')
            instr('strb r1, [{}]'.format(datareg))
        elif node in '[]':
            instr('')            
            instr('ldrb r1, [{}]'.format(datareg))
            instr('cmp  r1, 0')
            if node == '[':
                instr('beq  ' + end_label(lbl))
                label(start_label(lbl))
            elif node == ']':
                instr('bne  ' + start_label(lbl))
                label(end_label(lbl))                
        elif node in '.,':
            instr('')
            if node == ',':
                instr('mov  r7, 3')  # Syscall 3 is read
                instr('mov  r0, 0')
            elif node == '.':
                instr('mov  r7, 4')
                instr('mov  r0, 1')                
            instr('mov  r1, {}'.format(datareg))
            instr('mov  r2, 1')
            instr('svc  0')
        else:
            raise Exception("Unexpected " + node)

def optimize(ins):
    ins = iter(ins)

    out = []
    prev = None
    skip = False
    # Dead store elimination
    for item in ins:
        if prev is None:
            pass
        elif skip:
            skip = False
        elif prev == 'strb r1, [r5]' and item == 'ldrb r1, [r5]':
            skip = True
        else:
            out.append(prev)
        prev = item
    out.append(prev)

    ins = iter(out)
    out = []
    prev = None
    # Constant folding
    for item in ins:
        if prev is None:
            pass
        elif ('add' in prev or 'sub' in prev) and ('add' in item or 'sub' in item):
            rega = prev.split(',')[0].split()[1]
            regb = item.split(',')[0].split()[1]
            if rega != regb:
                out.append(prev)
            else:
                a = int(prev.split(',')[-1])
                if 'sub' in prev:
                    a = -a

                b = int(item.split(',')[-1])
                if 'sub' in item:
                    b = -b
                
                op = 'add'
                if a + b < 0:
                    a, b, op = -a, -b, 'sub'

                item = op + '  ' + rega + ', ' + str(a + b)
        else:
            out.append(prev)
        prev = item
    out.append(prev)
    return out

def fmt(l):
    if ':' in l:
        return l
    else:
        return ' '*8 + l

def list_codegen(ast, toplevel=True):
    for node in ast:
        codegen(node)
    if toplevel:
        global output
        output = optimize(output)
        output = '''\
        .text
        .global main
        .syntax unified
main:   ldr {data}, =tape
{code}
        .data
tape:   .space 30000
'''.format(data=datareg, code='\n'.join(fmt(line) for line in output))
        return output

try:
    infile = sys.argv[1]
except:
    print("Usage: {} in [out]".format(os.path.basename(sys.argv[0])))
    exit(1)

try:
    outfile = sys.argv[2]
except:
    outfile = "out.s"

text = open(infile, 'r').read()
ast = parse(text)
output = list_codegen(ast)
open(outfile, 'w').write(output)
