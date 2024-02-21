window.addEventListener('contextmenu', function (e) {
    e.preventDefault();
});

// Prevent text selection
window.addEventListener('selectstart', function (e) {
    e.preventDefault();
});

// Prevent Ctrl
window.addEventListener('keydown', function (e) {
    if (e.ctrlKey) {
        e.preventDefault();
    }
});
