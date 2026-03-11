(function () {
  var isLocal = window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1';
  var enUrl = isLocal
    ? window.location.href.replace(/localhost:\d+/, 'localhost:3000')
    : '/edgesentry-rs/en/';
  var bar = document.createElement('div');
  bar.style.cssText = 'background:#f0f4ff;border-bottom:1px solid #c8d8ff;padding:6px 16px;font-size:0.85em;text-align:right;';
  bar.innerHTML = '🌐 <a href="' + enUrl + '" style="color:#3a7bd5;text-decoration:none;">English</a>';
  var content = document.querySelector('.content');
  if (content) content.prepend(bar);
})();
