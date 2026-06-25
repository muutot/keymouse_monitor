import re

def split_entry_text(full):
    m = re.search(r'\s—\s(\[[^\]]*\]\([^)]*\)(?:[,\s]*\[[^\]]*\]\([^)]*\))*)$', full)
    if not m:
        m = re.search(r'\s—\s(\((?:\s*\[[^\]]*\]\([^)]*\)[,\s]*)+\))$', full)
    if m:
        desc = full[:m.start()].strip()
        link_str = m.group(1).strip()
        return desc, ' — ' + link_str
    return full.strip(), ''

def main():
    with open('CHANGELOG.md', 'r', encoding='utf-8') as f:
        lines = f.read().split('\n')

    result = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if not line.startswith('- '):
            result.append(line)
            i += 1
            continue

        parts = [line]
        j = i + 1
        while j < len(lines) and lines[j].startswith('  '):
            parts.append(lines[j])
            j += 1

        full = ' '.join(p.strip() for p in parts)
        desc_text, link_str = split_entry_text(full)

        if desc_text.startswith('- '):
            desc_text = desc_text[2:].strip()
        desc_text = desc_text.strip()

        # Determine effective limits: last line needs room for ` — [`...`]` overhead
        # Link overhead: ` — ` (3) + `[`` ` + ` `` ` + `]` (4) = 7 display chars
        last_line_reserve = 7 if link_str else 0
        effective_limit = 88
        
        words = desc_text.split()
        n = len(words)
        if n == 0:
            wrapped = ['']
        elif n == 1:
            wrapped = [words[0]]
        else:
            # Simple minimum-raggedness DP: cost = sum of (limit - line_len)^2, except last line gets tighter limit
            INF = 10**9
            cost = [INF] * (n + 1)
            break_at = [-1] * (n + 1)
            cost[0] = 0
            
            for i in range(1, n + 1):
                # Try all possible previous break points j (0..i-1)
                line_len = 0
                # Count chars of words[j..i-1] incrementally
                for j in range(i - 1, -1, -1):
                    if j == i - 1:
                        line_len = len(words[j])
                    else:
                        line_len += 1 + len(words[j])  # space + word
                    
                    prefix = '- ' if j == 0 else '  '
                    total = line_len + len(prefix)
                    
                    # Check if this line exceeds its limit
                    is_last = (i == n)
                    limit = effective_limit - (last_line_reserve if is_last else 0)
                    if total > limit:
                        break  # earlier j means longer line, so stop
                    
                    waste = limit - total
                    line_cost = waste * waste
                    
                    if cost[j] + line_cost < cost[i]:
                        cost[i] = cost[j] + line_cost
                        break_at[i] = j
            
            # If DP failed (no valid break found), fall back to greedy
            if cost[n] >= INF:
                # Greedy fallback
                wrapped = []
                cur = ''
                for w in words:
                    prefix = '- ' if not wrapped else '  '
                    test = (cur + ' ' + w).strip()
                    if len(prefix + test) <= effective_limit:
                        cur = test
                    else:
                        wrapped.append(cur)
                        cur = w
                if cur:
                    wrapped.append(cur)
            else:
                # Reconstruct lines
                wrapped = []
                end = n
                while end > 0:
                    start = break_at[end]
                    wrapped.insert(0, ' '.join(words[start:end]))
                    end = start
        
        if not wrapped:
            wrapped = ['']

        # Backward rebalance: push words from earlier lines to later lines
        # to minimize short lines (threshold: 45 display chars minimum)
        MIN_CHARS = 45
        for k in range(len(wrapped) - 1, 0, -1):
            prefix_cur = '  '
            prefix_prev = '- ' if k == 1 else '  '
            cur_text = wrapped[k]
            while True:
                cur_len = len(prefix_cur + cur_text)
                prev_words = wrapped[k - 1].split()
                if cur_len >= MIN_CHARS or len(prev_words) <= 1:
                    break
                pulled = prev_words[-1]
                new_prev = ' '.join(prev_words[:-1])
                new_prev_len = len(prefix_prev + new_prev)
                new_cur = pulled + ' ' + cur_text
                new_cur_len = len(prefix_cur + new_cur)
                if new_cur_len <= effective_limit and new_prev_len >= MIN_CHARS:
                    cur_text = new_cur
                    wrapped[k - 1] = new_prev
                else:
                    break
            wrapped[k] = cur_text

        if link_str:
            wrapped[-1] = wrapped[-1] + link_str

        result.append('- ' + wrapped[0])
        for l in wrapped[1:]:
            result.append('  ' + l)
        i = j

    with open('CHANGELOG.md', 'w', encoding='utf-8') as f:
        f.write('\n'.join(result) + '\n')

    print(f'Total lines: {len(result)}')

    over = 0
    for n, l in enumerate(result, 1):
        if l.startswith('#') or l.startswith('>'):
            continue
        display = re.sub(r'\[([^\]]*)\]\([^)]*\)', '', l).rstrip()
        if len(display) > 88:
            over += 1
            if over <= 3:
                print(f'  OVER line {n}: {len(display)} chars')
    print(f'Over 88: {over}')

    short = 0
    for n, l in enumerate(result, 1):
        if l.startswith('#') or l.startswith('>'):
            continue
        display = re.sub(r'\[([^\]]*)\]\([^)]*\)', '', l).rstrip()
        t = display.strip()
        if not t:
            continue
        if len(t) < 40:
            short += 1
            if short <= 10:
                print(f'  SHORT line {n}: {len(t)} chars: {t}')
    print(f'Under 40: {short}')

if __name__ == '__main__':
    main()
