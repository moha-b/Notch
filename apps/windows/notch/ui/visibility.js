// The notch's visibility modes and all-displays fleet, matching the Mac NotchVisibility: the card stays
// open when always shown, a peek never overrides Hide, and only the main window plays sounds.
globalThis.NotchVisibility = {
  keepsCardOpen(mode) {
    return mode === 'always_show';
  },

  allowsPeek(mode) {
    return mode !== 'hidden';
  },

  // Every display's notch receives the same alerts, so the chime must come from one window
  playsSounds(label) {
    return label === 'notch';
  },
};
