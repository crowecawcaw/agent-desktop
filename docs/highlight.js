document.querySelectorAll('pre code').forEach(function(block) {
  var lines = block.textContent.split('\n');
  var out = lines.map(function(line) {
    if (/^\s*#/.test(line)) {
      return '<span class="hl-comment">' + esc(line) + '</span>';
    }
    // Tokenize the line so each part gets exactly one highlight
    return line.replace(
      /(agent-desktop\s+\w+|cargo install\s+\S+)|(--[\w-]+)|("[^"]*"|'[^']*')|(#.*$)|(\S+)/g,
      function(m, cmd, flag, str, comment, other) {
        if (cmd) return '<span class="hl-command">' + esc(cmd) + '</span>';
        if (flag) return '<span class="hl-flag">' + esc(flag) + '</span>';
        if (str) return '<span class="hl-string">' + esc(str) + '</span>';
        if (comment) return '<span class="hl-comment">' + esc(comment) + '</span>';
        return esc(m);
      }
    );
  });
  block.innerHTML = out.join('\n');
});

function esc(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
