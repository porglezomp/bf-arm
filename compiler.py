from __future__ import print_function

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

def start_label(l): return 'BF_Start_' + str(l)
def end_label(l): return 'BF_End_' + str(l)
def instr(i): print('        ' + i)
def label(l): print(l + ':')

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
            instr('ldrb r1, [{}]'.format(datareg))
            instr('cmp  r1, 0')
            if node == '[':
                instr('beq  ' + end_label(lbl))
                label(start_label(lbl))
            elif node == ']':
                instr('bne  ' + start_label(lbl))
                label(end_label(lbl))                
        elif node in '.,':
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

def list_codegen(ast, toplevel=True):
    if toplevel:
        print('''\
        .text
        .global main
        .syntax unified
main:   ldr {}, =tape'''.format(datareg))
    for node in ast:
        codegen(node)
    if toplevel:
        print('''
        .data
tape:   .space 30000''')

ast = parse("""
+++++ +++
[
    >++++
    [
        >++
        >+++
        >+++
        >+
        <<<<-
    ]
    >+
    >+
    >-
    >>+
    [<]
    <-
]

>>.
>---.
+++++++..+++.
>>.
<-.
<.
+++.------.--------.
>>+.
>++.
""")
list_codegen(ast)
