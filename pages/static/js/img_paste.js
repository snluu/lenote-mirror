var __pasteCallbacks = {};

function setImagePasteCallback(id, cb) {
  __pasteCallbacks[id] = cb;
  console.log("Image paste callback set");
}

function handleInputPaste(id, event) {
  let pasteCallback = __pasteCallbacks[id];
  if (pasteCallback === null || pasteCallback === undefined) {
    return;
  }

  // use event.originalEvent.clipboard for newer chrome versions
  var items = (event.clipboardData || event.originalEvent.clipboardData).items;
  // find pasted image among pasted items
  var blob = null;
  for (var i = 0; i < items.length; i++) {
    if (items[i].type.indexOf("image") === 0) {
      blob = items[i].getAsFile();
    }
  }
  // load image if there is a pasted image
  if (blob !== null) {
    var reader = new FileReader();
    reader.onload = function (event) {
      pasteCallback(event.target.result); // data url!
    };
    reader.readAsDataURL(blob);
  }
}
