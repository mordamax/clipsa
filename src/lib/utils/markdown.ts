const HTML_ESCAPES: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;'
};

export function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (char) => HTML_ESCAPES[char]);
}

/** `code`, **bold**; [links](…) become their plain text. Input is escaped first. */
function inline(text: string): string {
  return text
    .split(/(`[^`]+`)/)
    .map((part, i) =>
      i % 2 === 1
        ? `<code>${escapeHtml(part.slice(1, -1))}</code>`
        : escapeHtml(part)
            .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
            .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    )
    .join('');
}

/**
 * Renders the small Markdown subset used in CHANGELOG.md release notes
 * (headings, `- ` bullets, paragraphs, **bold**, `code`) to HTML. Everything
 * else is shown as escaped text, so the result is safe for `{@html}`.
 */
export function renderReleaseNotes(markdown: string): string {
  const out: string[] = [];
  let paragraph: string[] = [];
  let listOpen = false;

  const flushParagraph = () => {
    if (paragraph.length) out.push(`<p>${paragraph.map(inline).join(' ')}</p>`);
    paragraph = [];
  };
  const closeList = () => {
    if (listOpen) out.push('</ul>');
    listOpen = false;
  };

  for (const raw of markdown.replace(/\r\n?/g, '\n').split('\n')) {
    const line = raw.trim();
    const heading = /^(#{1,6})\s+(.*)$/.exec(line);
    const bullet = /^[-*]\s+(.*)$/.exec(line);
    if (!line) {
      flushParagraph();
      closeList();
    } else if (heading) {
      flushParagraph();
      closeList();
      const tag = heading[1].length <= 2 ? 'h3' : 'h4';
      out.push(`<${tag}>${inline(heading[2])}</${tag}>`);
    } else if (bullet) {
      flushParagraph();
      if (!listOpen) out.push('<ul>');
      listOpen = true;
      out.push(`<li>${inline(bullet[1])}</li>`);
    } else if (listOpen && /^\s/.test(raw)) {
      // Indented continuation of the previous bullet.
      out[out.length - 1] = `${out[out.length - 1].slice(0, -'</li>'.length)} ${inline(line)}</li>`;
    } else {
      closeList();
      paragraph.push(line);
    }
  }
  flushParagraph();
  closeList();
  return out.join('');
}
