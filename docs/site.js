const popup = document.querySelector('#popup-demo');
const trayButton = document.querySelector('#tray-toggle');
const heroDemo = document.querySelector('.hero-demo');
let hideTimer;

function setPopup(open) {
  popup.classList.toggle('is-hidden', !open);
  trayButton.classList.toggle('active', open);
  trayButton.setAttribute('aria-expanded', String(open));
}

function scheduleHide() {
  clearTimeout(hideTimer);
  hideTimer = window.setTimeout(() => setPopup(false), 1500);
}

trayButton?.addEventListener('click', () => {
  const nextOpen = popup.classList.contains('is-hidden');
  setPopup(nextOpen);
  if (nextOpen) scheduleHide();
});

popup?.addEventListener('mouseenter', () => clearTimeout(hideTimer));
popup?.addEventListener('mouseleave', scheduleHide);
heroDemo?.addEventListener('mouseenter', () => clearTimeout(hideTimer));

document.querySelectorAll('a[href^="#"]').forEach((link) => {
  link.addEventListener('click', (event) => {
    const target = document.querySelector(link.getAttribute('href'));
    if (!target) return;
    event.preventDefault();
    target.scrollIntoView({ behavior: 'smooth' });
  });
});
