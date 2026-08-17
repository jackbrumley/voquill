(function () {
  'use strict';

  var PLACEHOLDER = '# Select a platform above';

  var COMMANDS = {
    linux: {
      system: {
        label: 'System Package',
        command: 'curl -sf https://voquill.org/install.sh | sudo bash -s -- --system --yes',
      },
      appimage: {
        label: 'AppImage',
        command: 'curl -sf https://voquill.org/install.sh | bash',
      },
      uninstall: {
        label: 'Uninstall',
        command: 'curl -sf https://voquill.org/uninstall.sh | bash',
      },
    },
    windows: {
      powershell: {
        label: 'PowerShell',
        command: 'irm https://voquill.org/install.ps1 | iex',
      },
      uninstall: {
        label: 'Uninstall',
        command: 'irm https://voquill.org/uninstall.ps1 | iex',
      },
    },
  };

  var COPY_ICON =
    '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
    '<rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>' +
    '<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>' +
    '</svg>';

  var CHECK_ICON =
    '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
    '<polyline points="20 6 9 17 4 12"></polyline>' +
    '</svg>';

  var platformLabels = document.querySelectorAll('[data-platform]');
  var codeBlock = document.getElementById('install-code');
  var copyBtn = document.getElementById('install-copy-btn');
  var variantContainer = document.getElementById('install-variants');
  var selectedPlatform = null;
  var selectedVariant = null;

  function renderCommand() {
    if (!selectedPlatform) {
      codeBlock.textContent = PLACEHOLDER;
      codeBlock.classList.add('placeholder');
      variantContainer.innerHTML = '';
      copyBtn.style.display = 'none';
      return;
    }
    codeBlock.classList.remove('placeholder');
    copyBtn.style.display = '';

    var platform = COMMANDS[selectedPlatform];
    if (!platform) return;
    var variant = platform[selectedVariant];
    if (!variant) {
      var keys = Object.keys(platform);
      selectedVariant = keys[0];
      variant = platform[keys[0]];
    }
    codeBlock.textContent = variant.command;
    updateVariantButtons();
  }

  function updateVariantButtons() {
    if (!selectedPlatform) return;
    var platform = COMMANDS[selectedPlatform];
    if (!platform) return;
    var keys = Object.keys(platform);
    var html = '';
    for (var i = 0; i < keys.length; i++) {
      var key = keys[i];
      var v = platform[key];
      var active = key === selectedVariant ? ' active' : '';
      html += '<button class="install-variant-btn' + active + '" data-install-variant="' + key + '">' + v.label + '</button>';
    }
    variantContainer.innerHTML = html;
    var variantButtons = document.querySelectorAll('[data-install-variant]');
    for (var j = 0; j < variantButtons.length; j++) {
      variantButtons[j].addEventListener('click', onVariantClick);
    }
  }

  function onPlatformClick(e) {
    var btn = e.currentTarget;
    selectedPlatform = btn.getAttribute('data-platform');
    var keys = Object.keys(COMMANDS[selectedPlatform]);
    selectedVariant = keys[0];
    for (var i = 0; i < platformLabels.length; i++) {
      platformLabels[i].classList.toggle('active', platformLabels[i] === btn);
    }
    renderCommand();
  }

  function onVariantClick(e) {
    var btn = e.currentTarget;
    selectedVariant = btn.getAttribute('data-install-variant');
    var siblings = variantContainer.querySelectorAll('.install-variant-btn');
    for (var i = 0; i < siblings.length; i++) {
      siblings[i].classList.toggle('active', siblings[i] === btn);
    }
    renderCommand();
  }

  function onCopyClick() {
    var text = codeBlock.textContent;
    if (!text || text === PLACEHOLDER) return;
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(function () {
        showCopiedFeedback();
      }, function () {
        fallbackCopy(text);
      });
    } else {
      fallbackCopy(text);
    }
  }

  function fallbackCopy(text) {
    var textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    document.body.appendChild(textarea);
    textarea.select();
    try {
      document.execCommand('copy');
      showCopiedFeedback();
    } catch (e) {
      window.getSelection().removeAllRanges();
    }
    document.body.removeChild(textarea);
  }

  function showCopiedFeedback() {
    copyBtn.innerHTML = CHECK_ICON;
    copyBtn.classList.add('copied');
    setTimeout(function () {
      copyBtn.innerHTML = COPY_ICON;
      copyBtn.classList.remove('copied');
    }, 1500);
  }

  copyBtn.innerHTML = COPY_ICON;

  for (var i = 0; i < platformLabels.length; i++) {
    platformLabels[i].addEventListener('click', onPlatformClick);
  }
  copyBtn.addEventListener('click', onCopyClick);

  renderCommand();
})();