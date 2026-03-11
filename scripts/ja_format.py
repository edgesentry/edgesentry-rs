#!/usr/bin/env python3
"""
Apply Japanese typographic spacing rules to Markdown files:
- Half-width space between CJK and ASCII characters (both directions)
Spaces are added OUTSIDE bold/italic markers, never inside (**テキスト** stays intact).
Code blocks and inline code spans are left unchanged.
"""
import re, os, sys

CJK = r'[\u3000-\u303f\u3040-\u309f\u30a0-\u30ff\u4e00-\u9fff\uff00-\uffef\u3400-\u4dbf]'
ASCII_CHAR = r'[A-Za-z0-9]'


def add_spaces(text):
    # Remove spaces immediately inside bold/italic markers (either side independently)
    # e.g. ** テキスト** → **テキスト**  and  **テキスト ** → **テキスト**
    text = re.sub(r'\*\*\s+([^*])', r'**\1', text)
    text = re.sub(r'([^*])\s+\*\*', r'\1**', text)
    text = re.sub(r'\*\s+([^*])', r'*\1', text)
    text = re.sub(r'([^*])\s+\*([^*])', r'\1*\2', text)
    # Space between ASCII letters/digits and CJK (both directions)
    text = re.sub(f'({ASCII_CHAR})({CJK})', r'\1 \2', text)
    text = re.sub(f'({CJK})({ASCII_CHAR})', r'\1 \2', text)
    return text


def process_line(line):
    parts = re.split(r'(`[^`\n]+`)', line)
    result = []
    for i, part in enumerate(parts):
        if i % 2 == 1:
            result.append(part)
        else:
            result.append(add_spaces(part))
    return ''.join(result)


def process_file(path):
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()
    lines = content.split('\n')
    result = []
    in_code_block = False
    for line in lines:
        if re.match(r'^\s*```', line):
            in_code_block = not in_code_block
        if in_code_block or line.startswith('    '):
            result.append(line)
        else:
            result.append(process_line(line))
    new_content = '\n'.join(result)
    if new_content != content:
        with open(path, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f'Updated: {path}')
    else:
        print(f'No change: {path}')


if __name__ == '__main__':
    paths = sys.argv[1:] if len(sys.argv) > 1 else []
    if not paths:
        base = os.path.join(os.path.dirname(__file__), '..', 'docs', 'ja', 'src')
        paths = [os.path.join(base, f) for f in sorted(os.listdir(base)) if f.endswith('.md')]
    for p in paths:
        process_file(p)
