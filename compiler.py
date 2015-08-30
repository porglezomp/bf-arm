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

label_id = 0
def codegen(node):
    global label_id
    if isinstance(node, list):
        codegen('[')
        list_codegen(node, toplevel=False)
        codegen(']')
    else:
        print(node, end='')

def list_codegen(ast, toplevel=True):
    if toplevel:
        print('')
    for node in ast:
        codegen(node)
    if toplevel:
        print('')

ast = parse("this will add 3 to 4 +++>++++<[->+<] it should produce 7.")
list_codegen(ast)
