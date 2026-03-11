(function () {
  var bar = document.createElement('div');
  bar.style.cssText = 'background:#f0f4ff;border-bottom:1px solid #c8d8ff;padding:6px 16px;font-size:0.85em;text-align:right;';
  bar.innerHTML = '🌐 <a href="/edgesentry-rs/en/" style="color:#3a7bd5;text-decoration:none;">English</a>';
  var content = document.querySelector('.content');
  if (content) content.prepend(bar);
})();
