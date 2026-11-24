/* Standalone mod documentation page: language switch, live version, copy
 * buttons and the table of contents.  Kept separate from app.js so the main
 * manager page no longer carries the developer reference. */
(function () {
  'use strict';

  var lang = 'zh-CN';
  var COPY_OK = { 'zh-CN': '已复制', 'en-US': 'Copied' };
  var COPY_FAIL = { 'zh-CN': '复制失败，请手动选择文本', 'en-US': 'Copy failed; select the text manually' };

  function $(id) {
    return document.getElementById(id);
  }

  function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) {
      return navigator.clipboard.writeText(text);
    }
    return new Promise(function (resolve, reject) {
      var area = document.createElement('textarea');
      area.value = text;
      area.setAttribute('readonly', 'readonly');
      area.style.position = 'fixed';
      area.style.left = '-9999px';
      document.body.appendChild(area);
      area.select();
      try {
        if (document.execCommand('copy')) resolve();
        else reject(new Error('copy failed'));
      } catch (error) {
        reject(error);
      } finally {
        document.body.removeChild(area);
      }
    });
  }

  function status(text) {
    var node = $('docCopyStatus');
    if (!node) return;
    node.textContent = text;
    window.setTimeout(function () {
      if (node.textContent === text) node.textContent = '';
    }, 2600);
  }

  function applyLanguage(next) {
    lang = next === 'en-US' ? 'en-US' : 'zh-CN';
    document.documentElement.lang = lang;
    var zh = $('docsZh');
    var en = $('docsEn');
    if (zh) zh.hidden = lang === 'en-US';
    if (en) en.hidden = lang !== 'en-US';
    var title = document.querySelector('title');
    if (title) {
      title.textContent = lang === 'en-US' ? 'Mod docs · linkSlot' : 'mod 开发文档 · linkSlot';
    }
  }

  function loadLanguage() {
    return fetch('/api/i18n', { cache: 'no-store' })
      .then(function (response) {
        if (!response.ok) throw new Error('HTTP ' + response.status);
        return response.json();
      })
      .then(function (data) {
        applyLanguage(data && data.language);
      })
      .catch(function () {
        // The Chinese card is the built-in fallback and is visible by default.
        applyLanguage('zh-CN');
      });
  }

  function fillVersion(version) {
    var value = version || '1.x';
    var nodes = document.querySelectorAll('[data-doc-version]');
    for (var i = 0; i < nodes.length; i += 1) {
      nodes[i].textContent = version || '—';
    }
    ['mod-ai-prompt', 'mod-ai-prompt-en'].forEach(function (id) {
      var node = $(id);
      if (node) node.textContent = node.textContent.replace(/\{\{version\}\}/g, value);
    });
  }

  function loadVersion() {
    return fetch('/api/status', { cache: 'no-store' })
      .then(function (response) {
        if (!response.ok) throw new Error('HTTP ' + response.status);
        return response.json();
      })
      .then(function (data) {
        fillVersion(data && data.slot);
      })
      .catch(function () {
        fillVersion('');
      });
  }

  function bindCopy() {
    var buttons = document.querySelectorAll('[data-copy]');
    for (var i = 0; i < buttons.length; i += 1) {
      buttons[i].addEventListener('click', function () {
        var target = $(this.getAttribute('data-copy'));
        if (!target) return;
        copyText(target.textContent)
          .then(function () {
            status(COPY_OK[lang]);
          })
          .catch(function () {
            status(COPY_FAIL[lang]);
          });
      });
    }
  }

  function bindExport() {
    var buttons = document.querySelectorAll('[data-export-mod-docs]');
    for (var i = 0; i < buttons.length; i += 1) {
      buttons[i].addEventListener('click', function () {
        window.location.href = '/api/mods/docs.zip';
      });
    }
  }

  function bindToc() {
    var links = document.querySelectorAll('.docs-toc a');
    for (var i = 0; i < links.length; i += 1) {
      links[i].addEventListener('click', function (event) {
        var target = $(this.getAttribute('href').slice(1));
        if (!target) return;
        event.preventDefault();
        // Some TOC entries point at a heading or alias inside the full API
        // details; open every ancestor so the target is actually visible.
        var parent = target.parentElement;
        while (parent) {
          if (parent.tagName === 'DETAILS') parent.open = true;
          parent = parent.parentElement;
        }
        target.scrollIntoView({ behavior: 'smooth', block: 'start' });
      });
    }
  }

  function start() {
    bindCopy();
    bindExport();
    bindToc();
    loadLanguage();
    loadVersion();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', start);
  } else {
    start();
  }
})();
