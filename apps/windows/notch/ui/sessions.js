// "Is it working?" per provider id. Claude profiles (claude, claude-<slug>) read their own hook and
// transcript sessions; every other provider, Codex profiles included, reads the activity probe.
globalThis.NotchSessions = {
  isClaude(id) {
    return id === 'claude' || id.startsWith('claude-');
  },

  // Sessions from builds before profile routing carry no provider and belong to the default profile
  sessionsOf(id, snapshot) {
    return (snapshot.sessions || []).filter(session => (session.provider || 'claude') === id);
  },

  // running | attention | done | idle
  workState(id, snapshot, activity) {
    if (this.isClaude(id)) {
      const states = new Set(this.sessionsOf(id, snapshot).map(session => session.state));
      const state = ['attention', 'running', 'done'].find(candidate => states.has(candidate));
      if (state) return state;
      // Cloud sessions leave no transcript; only the default desktop app's network activity can stand in
      if (id === 'claude' && activity.some(a => a.provider === 'claude' && a.state === 'busy')) return 'running';
      return 'idle';
    }
    const probe = id === 'antigravity' ? 'gemini' : id;
    const acts = activity.filter(a => a.provider === probe);
    if (acts.some(a => a.state === 'waiting')) return 'attention';
    if (acts.some(a => a.state === 'busy')) return 'running';
    return 'idle';
  },
};
