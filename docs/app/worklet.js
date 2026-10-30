// The speakers' end: plays the chunks the engine's worker makes, asking for
// more while less than AHEAD is queued, and tells the page what is playing
// as each chunk starts.

const AHEAD = 0.4 * 48000;

class Musicbox extends AudioWorkletProcessor {
  constructor() {
    super();
    this.queue = [];
    this.at = 0;
    this.queued = 0;
    this.asked = 0;
    this.src = null;
    this.port.onmessage = (e) => {
      this.src = e.data.port;
      this.src.onmessage = (ev) => {
        this.queue.push(ev.data);
        this.queued += ev.data.pcm.length / 2;
        this.asked = Math.max(0, this.asked - ev.data.pcm.length / 2);
      };
      this.ask();
    };
  }

  ask() {
    const want = AHEAD - this.queued - this.asked;
    if (this.src && want > 0) {
      this.src.postMessage({ need: want });
      this.asked += want;
    }
  }

  process(_inputs, outputs) {
    const [l, r] = outputs[0];
    let i = 0;
    while (i < l.length && this.queue.length) {
      const c = this.queue[0];
      if (this.at === 0) {
        this.port.postMessage({ now: c.now });
      }
      const n = Math.min(l.length - i, c.pcm.length / 2 - this.at);
      for (let k = 0; k < n; k++) {
        l[i + k] = c.pcm[2 * (this.at + k)];
        if (r) r[i + k] = c.pcm[2 * (this.at + k) + 1];
      }
      i += n;
      this.at += n;
      this.queued -= n;
      if (this.at * 2 >= c.pcm.length) {
        this.queue.shift();
        this.at = 0;
      }
    }
    // (an underrun: silence till more comes)
    l.fill(0, i);
    if (r) r.fill(0, i);
    this.ask();
    return true;
  }
}

registerProcessor('musicbox', Musicbox);
