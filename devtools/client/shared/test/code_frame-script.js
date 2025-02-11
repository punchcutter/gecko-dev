/* eslint-disable */
"use strict";

const { loadSubScript } = Cc["@mozilla.org/moz/jssubscript-loader;1"].
                          getService(Ci.mozIJSSubScriptLoader);

// Set up a dummy environment so that EventUtils works. We need to be careful to
// pass a window object into each EventUtils method we call rather than having
// it rely on the |window| global.
const EventUtils = {};
EventUtils.window = content;
EventUtils.parent = EventUtils.window;
EventUtils._EU_Ci = Ci;
EventUtils._EU_Cc = Cc;
EventUtils.navigator = content.navigator;
EventUtils.KeyboardEvent = content.KeyboardEvent;
loadSubScript("chrome://mochikit/content/tests/SimpleTest/EventUtils.js", EventUtils);

dump("Frame script loaded.\n");

var workers = {};

this._eval = function(string) {
  dump("Evalling string.\n");

  return content.eval(string);
};

this.generateMouseClick = function(path) {
  dump("Generating mouse click.\n");

  const target = eval(path);
  EventUtils.synthesizeMouseAtCenter(target, {},
                                     target.ownerDocument.defaultView);
};

this.createWorker = function(url) {
  dump("Creating worker with url '" + url + "'.\n");

  return new Promise(function(resolve, reject) {
    const worker = new content.Worker(url);
    worker.addEventListener("message", function() {
      workers[url] = worker;
      resolve();
    }, {once: true});
  });
};

this.terminateWorker = function(url) {
  dump("Terminating worker with url '" + url + "'.\n");

  workers[url].terminate();
  delete workers[url];
};

this.postMessageToWorker = function(url, message) {
  dump("Posting message to worker with url '" + url + "'.\n");

  return new Promise(function(resolve) {
    const worker = workers[url];
    worker.postMessage(message);
    worker.addEventListener("message", function() {
      resolve();
    }, {once: true});
  });
};

addMessageListener("jsonrpc", function({ data: { method, params, id } }) {
  method = this[method];
  Promise.resolve().then(function() {
    return method.apply(undefined, params);
  }).then(function(result) {
    sendAsyncMessage("jsonrpc", {
      result: result,
      error: null,
      id: id
    });
  }, function(error) {
    sendAsyncMessage("jsonrpc", {
      result: null,
      error: error.message.toString(),
      id: id
    });
  });
});

addMessageListener("test:postMessageToWorker", function(message) {
  dump("Posting message '" + message.data.message + "' to worker with url '" +
       message.data.url + "'.\n");

  let worker = workers[message.data.url];
  worker.postMessage(message.data.message);
  worker.addEventListener("message", function() {
    sendAsyncMessage("test:postMessageToWorker");
  }, {once: true});
});
