const grid = document.getElementById('grid');
const pill = document.getElementById('pill');
const tabs = Array.from(document.querySelectorAll('.tab'));
let active = 0;

function setActive(i) {
  active = i;
  tabs.forEach((t, idx) => t.classList.toggle('active', idx === i));
  pill.style.transform = `translateX(${i * 114}px)`;
}
function scrollToY(y) { grid.style.transform = `translateY(${y}px)`; }

tabs.forEach((t, i) => t.addEventListener('click', () => setActive(i)));
document.querySelectorAll('.card').forEach(c => {
  c.addEventListener('mousedown', () => c.classList.add('pressed'));
  c.addEventListener('mouseup', () => c.classList.remove('pressed'));
  c.addEventListener('mouseleave', () => c.classList.remove('pressed'));
});

// scripted demo (same timeline as the Slint demo)
const params = new URLSearchParams(location.search);
if (params.has('demo') || window.__TAURI_INTERNALS__) {
  let step = 0;
  const timer = setInterval(() => {
    step++;
    if (step === 1) scrollToY(-160);
    if (step === 2) scrollToY(-242);
    if (step === 3) setActive(1);
    if (step === 4) { setActive(2); scrollToY(0); }
    if (step === 5) setActive(0);
    if (step === 6) scrollToY(-120);
    if (step === 7) scrollToY(0);
    if (step >= 8) { clearInterval(timer); document.title = 'Andromeda UI Lab (Tauri) DONE'; }
  }, 900);
}
