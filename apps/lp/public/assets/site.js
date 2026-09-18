// kukuri LP (#1043)。JS が無くても全リンクが使える。JS は案内を OS に合わせるだけ。
(function () {
  'use strict';

  var root = document.documentElement;
  var ua = navigator.userAgent || '';
  var platform = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || '';

  var isMobile =
    /Android|iPhone|iPad|iPod|Mobile/i.test(ua) ||
    (navigator.userAgentData && navigator.userAgentData.mobile === true);

  if (isMobile) {
    root.classList.add('is-mobile');
  }

  // PC では、見ている OS のダウンロードを先頭の主ボタンにする。
  var os = /Win/i.test(platform) || /Windows/i.test(ua)
    ? 'windows'
    : /Linux/i.test(platform) || /Linux/i.test(ua)
      ? 'linux'
      : /Mac/i.test(platform) || /Mac OS/i.test(ua)
        ? 'mac'
        : 'other';
  root.setAttribute('data-os', os);

  document.querySelectorAll('[data-download-group]').forEach(function (group) {
    var preferred = group.querySelector('[data-os-target="' + os + '"]');
    if (preferred) {
      group.querySelectorAll('[data-os-target]').forEach(function (button) {
        button.classList.remove('button-primary');
        button.classList.add('button-secondary');
      });
      preferred.classList.remove('button-secondary');
      preferred.classList.add('button-primary');
      group.insertBefore(preferred, group.firstChild);
    }
  });

  document.querySelectorAll('[data-mac-note]').forEach(function (note) {
    note.hidden = os !== 'mac';
  });

  // スマートフォンでは PC で開くためのリンクを渡す。
  document.querySelectorAll('[data-handoff]').forEach(function (button) {
    button.addEventListener('click', function () {
      var status = button.parentElement.querySelector('.handoff-status');
      var url = location.origin + location.pathname;
      var done = function (message) {
        if (status) status.textContent = message;
      };
      if (navigator.share) {
        navigator
          .share({ title: document.title, url: url })
          .then(function () { done(button.getAttribute('data-shared')); })
          .catch(function () {});
        return;
      }
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard
          .writeText(url)
          .then(function () { done(button.getAttribute('data-copied')); })
          .catch(function () { done(url); });
        return;
      }
      done(url);
    });
  });
})();
