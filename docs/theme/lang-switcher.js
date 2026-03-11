(function () {
  var isLocal = window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1';
  var jaUrl = isLocal
    ? window.location.href.replace(/localhost:\d+/, 'localhost:3001')
    : '/edgesentry-rs/ja/';
  var bar = document.createElement('div');
  bar.style.cssText = 'background:#f0f4ff;border-bottom:1px solid #c8d8ff;padding:6px 16px;font-size:0.85em;text-align:right;';
  bar.innerHTML = '🌐 <a href="' + jaUrl + '" style="color:#3a7bd5;text-decoration:none;">日本語版</a>';
  var content = document.querySelector('.content');
  if (content) content.prepend(bar);
})();
