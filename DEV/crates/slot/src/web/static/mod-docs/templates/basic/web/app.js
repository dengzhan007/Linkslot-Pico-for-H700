(function () {
  'use strict';

  var info = null;
  var status = document.getElementById('status');
  var infoLine = document.getElementById('info');

  function say(text) {
    status.textContent = text;
  }

  LinkSlotMod.ready.then(function (value) {
    info = value;
    infoLine.textContent = (value.name || value.id) + ' v' + (value.version || '');
    return LinkSlotMod.request('files.read', { path: 'config.json' });
  }).then(function (file) {
    say((file && file.content) || 'no config yet');
  }).catch(function (error) {
    // A missing config is normal on first open.
    say('config: ' + ((error && error.message) || 'missing'));
  });

  document.getElementById('save').addEventListener('click', function () {
    LinkSlotMod.request('files.write', {
      path: 'config.json',
      content: JSON.stringify({ hello: 'world' }, null, 2)
    }).then(function () {
      say('saved');
    }).catch(function (error) {
      say((error && error.message) || 'save failed');
    });
  });

  // Web API example (requires context:read in mod.json and user grant):
  // LinkSlotMod.request('api', { method: 'GET', path: '/api/status' })
  //   .then(function (data) { say(JSON.stringify(data)); });
})();
