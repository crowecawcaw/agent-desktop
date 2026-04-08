document.querySelectorAll('pre code').forEach(function(block) {
  var html = block.innerHTML;
  html = html.split('\n').map(function(line) {
    if (/^\s*#/.test(line)) {
      return '<span class="hl-comment">' + line + '</span>';
    }
    line = line
      .replace(/(--[\w-]+)/g, '<span class="hl-flag">$1</span>')
      .replace(/(&quot;[^&]*&quot;|&#39;[^&]*&#39;|"[^"]*"|'[^']*')/g, '<span class="hl-string">$1</span>')
      .replace(/^(\s*)(agent-desktop\s+\w+)/, '$1<span class="hl-command">$2</span>')
      .replace(/^(\s*)(cargo install\s+\S+)/, '$1<span class="hl-command">$2</span>');
    return line;
  }).join('\n');
  block.innerHTML = html;
});
