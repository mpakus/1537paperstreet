const header = document.querySelector('.site-header')
const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches

function onScroll() {
  if (!header) return
  header.classList.toggle('is-scrolled', window.scrollY > 8)
}

onScroll()
window.addEventListener('scroll', onScroll, { passive: true })

const reveals = document.querySelectorAll('.reveal')

function revealPassed() {
  const limit = window.innerHeight * 0.92
  reveals.forEach((el) => {
    if (el.getBoundingClientRect().top < limit) {
      el.classList.add('is-in')
    }
  })
}

if (reduce) {
  reveals.forEach((el) => el.classList.add('is-in'))
} else {
  revealPassed()
  window.addEventListener('scroll', revealPassed, { passive: true })
  window.addEventListener('resize', revealPassed)
}
